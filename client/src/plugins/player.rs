use bevy::prelude::*;
use pot_shared::types::Direction;

use crate::app_state::AppState;
use crate::components::combat::{AbilityState, AnimationPhase, Cooldowns, Hurtbox, SelectedAbility};
use crate::components::player::*;
use crate::content::{AbilityDefs, CombatFeelConfig};
use crate::plugins::input::{GameInput, InputBuffer};
use crate::plugins::run::{ArenaCollision, resolve_world_collision};
use crate::plugins::vfx::gameplay_unfrozen;
use crate::rendering::isometric::WorldPosition;
use crate::rendering::sprites::{SpriteAssets, CharacterAtlasLayout};

pub struct PlayerPlugin;

/// Marker for the weapon swing arc sprite.
#[derive(Component)]
pub struct WeaponSwing {
    pub owner: Entity,
    pub elapsed_frames: u32,
    pub total_frames: u32,
    pub start_angle: f32,
    pub sweep_angle: f32,
    pub radius: f32,
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Run), spawn_player)
            .add_systems(OnExit(AppState::Run), despawn_player)
            .add_systems(
                Update,
                (
                    invulnerability_tick_system,
                    ability_slot_selection_system,
                    player_facing_system,
                    dodge_roll_start_system,
                    player_movement_system,
                    dodge_roll_progression_system,
                )
                    .chain()
                    .run_if(gameplay_unfrozen)
                    .run_if(in_state(AppState::Run)),
            )
            .add_systems(
                Update,
                (player_animation_system, weapon_swing_system)
                    .run_if(gameplay_unfrozen)
                    .run_if(in_state(AppState::Run)),
            );
    }
}

/// Spawn the player entity with all required components.
fn spawn_player(
    mut commands: Commands,
    sprite_assets: Res<SpriteAssets>,
    atlas_layout: Res<CharacterAtlasLayout>,
    ability_defs: Res<AbilityDefs>,
) {
    let base_stats = &ability_defs.base_stats;
    let dodge_roll = &ability_defs.dodge_roll;

    commands.spawn((
        // Identity and stats.
        Player,
        Health {
            current: base_stats.hp as f32,
            max: base_stats.hp as f32,
        },
        Mana {
            current: base_stats.mana as f32,
            max: base_stats.mana as f32,
        },
        MovementSpeed(base_stats.move_speed),
        Facing::default(),
        AimVector::default(),
        AimTarget(Vec2::new(0.0, -120.0)),
        DodgeState {
            speed: base_stats.move_speed * dodge_roll.speed_multiplier,
            active_frames: dodge_roll.active_frames,
            recovery_frames: dodge_roll.recovery_frames,
            cancel_frame: dodge_roll.cancel_frame,
            iframe_frames: dodge_roll.active_frames.saturating_sub(1),
            ..default()
        },
        DodgeCooldown {
            frames_remaining: 0,
            max_frames: dodge_roll.cooldown_frames.unwrap_or(18),
        },
        Invulnerable::default(),
        Velocity::default(),
        // Combat and animation.
        (
            AbilityState::default(),
            Cooldowns::default(),
            SelectedAbility::default(),
            AnimationTimer::default(),
            WorldPosition::new(0.0, 0.0),
            Hurtbox {
                radius: 12.0,
                faction: pot_shared::types::Faction::Player,
            },
            Sprite {
                image: sprite_assets.player.clone(),
                custom_size: Some(Vec2::new(384.0, 384.0)),
                texture_atlas: Some(TextureAtlas {
                    layout: atlas_layout.layout.clone(),
                    index: 0,
                }),
                ..default()
            },
            Transform::default(),
        ),
    ));
}

fn despawn_player(
    mut commands: Commands,
    query: Query<Entity, With<Player>>,
    swing_query: Query<Entity, With<WeaponSwing>>,
) {
    for entity in query.iter().chain(swing_query.iter()) {
        commands.entity(entity).despawn();
    }
}

fn dodge_roll_start_system(
    game_input: Res<GameInput>,
    mut buffer: ResMut<InputBuffer>,
    mut query: Query<
        (
            &mut DodgeState,
            &mut DodgeCooldown,
            &mut Velocity,
            &AimVector,
            &Facing,
            &AbilityState,
        ),
        With<Player>,
    >,
) {
    let Ok((mut dodge, mut cooldown, mut velocity, aim, facing, ability_state)) =
        query.single_mut()
    else {
        return;
    };

    let wants_dodge = game_input.dodge_pressed || buffer.dodge_buffered();
    if dodge.active || cooldown.frames_remaining > 0 || !wants_dodge {
        return;
    }

    if !(ability_state.is_idle() || ability_state.can_cancel() || dodge.can_cancel()) {
        return;
    }

    let dir = if game_input.move_direction != Vec2::ZERO {
        game_input.move_direction.normalize()
    } else if aim.0.length_squared() > 0.001 {
        aim.0.normalize()
    } else {
        facing_to_vec2(&facing.0)
    };

    if dir == Vec2::ZERO {
        return;
    }

    dodge.active = true;
    dodge.frame = 0;
    dodge.direction = dir;
    velocity.0 = dir * dodge.speed;
    cooldown.frames_remaining = cooldown.max_frames;
    buffer.consume_dodge();
}

/// Move the player based on authored locomotion values instead of binary on/off control.
fn player_movement_system(
    game_input: Res<GameInput>,
    feel: Res<CombatFeelConfig>,
    arena: Option<Res<ArenaCollision>>,
    time: Res<Time>,
    mut query: Query<(
        &MovementSpeed,
        &DodgeState,
        &AbilityState,
        &mut DodgeCooldown,
        &mut Invulnerable,
        &mut Velocity,
        &mut WorldPosition,
    ), With<Player>>,
) {
    let Ok((speed, dodge, ability_state, mut cooldown, mut invuln, mut velocity, mut world_pos)) =
        query.single_mut()
    else {
        return;
    };

    let dt = time.delta_secs();
    cooldown.frames_remaining = cooldown.frames_remaining.saturating_sub(1);

    if dodge.active {
        let burst_t = if dodge.active_frames == 0 {
            1.0
        } else {
            (dodge.frame as f32 / dodge.active_frames as f32).clamp(0.0, 1.0)
        };
        let burst_speed = if dodge.frame < dodge.active_frames {
            dodge.speed * (1.0 - burst_t * 0.38)
        } else {
            dodge.speed * 0.24
        };

        velocity.0 = dodge.direction * burst_speed;
        let desired = Vec2::new(
            world_pos.x + velocity.0.x * dt,
            world_pos.y + velocity.0.y * dt,
        );
        let resolved = if let Some(arena) = &arena {
            resolve_world_collision(desired, 12.0, arena)
        } else {
            desired
        };
        world_pos.x = resolved.x;
        world_pos.y = resolved.y;

        if dodge.is_in_iframes() {
            invuln.frames_remaining = invuln.frames_remaining.max(2);
        }
        return;
    }

    let move_multiplier = match ability_state.phase {
        AnimationPhase::Idle => 1.0,
        AnimationPhase::Recovery => feel.player_recovery_move_multiplier,
        AnimationPhase::Anticipation | AnimationPhase::Active => feel.player_attack_move_multiplier,
    };

    let desired_velocity = if game_input.move_direction == Vec2::ZERO {
        Vec2::ZERO
    } else {
        game_input.move_direction.normalize() * speed.0 * move_multiplier
    };

    if desired_velocity == Vec2::ZERO {
        velocity.0 = move_vec_toward(velocity.0, Vec2::ZERO, feel.player_deceleration * dt);
    } else {
        velocity.0 = move_vec_toward(velocity.0, desired_velocity, feel.player_acceleration * dt);
    }

    let desired = Vec2::new(world_pos.x + velocity.0.x * dt, world_pos.y + velocity.0.y * dt);
    let resolved = if let Some(arena) = &arena {
        resolve_world_collision(desired, 12.0, arena)
    } else {
        desired
    };
    world_pos.x = resolved.x;
    world_pos.y = resolved.y;
}

fn dodge_roll_progression_system(
    mut query: Query<(&mut DodgeState, &mut Velocity), With<Player>>,
) {
    let Ok((mut dodge, mut velocity)) = query.single_mut() else {
        return;
    };

    if !dodge.active {
        return;
    }

    dodge.frame += 1;
    if dodge.is_finished() {
        dodge.active = false;
        dodge.frame = 0;
        velocity.0 *= 0.35;
    }
}

/// Update player facing based on gamepad aim, movement direction, or mouse position.
fn player_facing_system(
    game_input: Res<GameInput>,
    mut query: Query<(&WorldPosition, &mut Facing, &mut AimVector, &mut AimTarget), With<Player>>,
) {
    let Ok((world_pos, mut facing, mut aim, mut aim_target)) = query.single_mut() else {
        return;
    };

    let player_world = Vec2::new(world_pos.x, world_pos.y);

    // Priority: gamepad right stick > mouse target > movement direction.
    let (dir, target) = if let Some(gamepad_aim) = game_input.aim_direction {
        let normalized = gamepad_aim.normalize_or_zero();
        (normalized, player_world + normalized * 220.0)
    } else {
        let mouse_delta = game_input.mouse_world_pos - player_world;
        if mouse_delta.length_squared() > 9.0 {
            let normalized = mouse_delta.normalize();
            (normalized, game_input.mouse_world_pos)
        } else if game_input.move_direction != Vec2::ZERO {
            let normalized = game_input.move_direction.normalize();
            (normalized, player_world + normalized * 120.0)
        } else {
            (aim.0, aim_target.0)
        }
    };

    if dir.length_squared() <= f32::EPSILON {
        return;
    }

    aim.0 = dir;
    aim_target.0 = target;

    let angle = dir.y.atan2(dir.x);
    facing.0 = Direction::from_angle(angle);
}

/// Switch selected ability slot on number key press or gamepad bumper cycle.
fn ability_slot_selection_system(
    game_input: Res<GameInput>,
    mut query: Query<&mut SelectedAbility, With<Player>>,
) {
    if let Some(slot) = game_input.ability_slot {
        let Ok(mut selected) = query.single_mut() else {
            return;
        };
        const NUM_SLOTS: usize = 6;
        if slot == usize::MAX {
            // Cycle forward (gamepad right bumper/trigger).
            selected.0 = (selected.0 + 1) % NUM_SLOTS;
        } else if slot == usize::MAX - 1 {
            // Cycle backward (gamepad left bumper/trigger).
            selected.0 = (selected.0 + NUM_SLOTS - 1) % NUM_SLOTS;
        } else {
            selected.0 = slot;
        }
    }
}

/// Tick down invulnerability frames each frame.
fn invulnerability_tick_system(
    mut query: Query<&mut Invulnerable>,
) {
    for mut invuln in &mut query {
        invuln.frames_remaining = invuln.frames_remaining.saturating_sub(1);
    }
}

fn move_vec_toward(current: Vec2, target: Vec2, max_delta: f32) -> Vec2 {
    let delta = target - current;
    let distance = delta.length();

    if distance <= max_delta || distance <= f32::EPSILON {
        target
    } else {
        current + delta / distance * max_delta
    }
}

/// Map a Direction to the FLARE sprite sheet row index.
/// Row 0=S, 1=SW, 2=W, 3=NW, 4=N, 5=NE, 6=E, 7=SE.
fn direction_to_flare_row(dir: Direction) -> u32 {
    match dir {
        Direction::S  => 0,
        Direction::SW => 1,
        Direction::W  => 2,
        Direction::NW => 3,
        Direction::N  => 4,
        Direction::NE => 5,
        Direction::E  => 6,
        Direction::SE => 7,
    }
}

/// Update the player's TextureAtlas index based on facing direction, movement, and attacks.
fn player_animation_system(
    _game_input: Res<GameInput>,
    mut query: Query<(
        Entity,
        &Facing,
        &AimVector,
        &DodgeState,
        &AbilityState,
        &WorldPosition,
        &Velocity,
        &mut AnimationTimer,
        &mut Sprite,
    ), With<Player>>,
    swing_query: Query<&WeaponSwing>,
    mut commands: Commands,
) {
    let Ok((entity, facing, aim, dodge, ability_state, world_pos, velocity, mut anim, mut sprite)) = query.single_mut() else {
        return;
    };

    let row = direction_to_flare_row(facing.0);

    let is_attacking = !ability_state.is_idle();
    let is_moving = velocity.0.length_squared() > 180.0
        && !dodge.active
        && ability_state.is_idle();

    if is_attacking {
        // During attack: rapid frame cycle (2-frame speed) through cols 4-7 for attack look.
        anim.frame_counter += 1;
        if anim.frame_counter >= 3 {
            anim.frame_counter = 0;
            anim.current_column = 4 + (anim.current_column.wrapping_sub(3)) % 4;
        }

        // Spawn weapon swing arc on Anticipation frame 0 (once per attack).
        if matches!(ability_state.phase, AnimationPhase::Anticipation)
            && ability_state.frame_in_phase == 0
            && !swing_query.iter().any(|s| s.owner == entity)
        {
            let dir = if aim.0.length_squared() > 0.001 {
                aim.0.normalize()
            } else {
                facing_to_vec2(&facing.0)
            };
            let base_angle = dir.y.atan2(dir.x);
            let screen = crate::rendering::isometric::world_to_screen(world_pos.x, world_pos.y);

            commands.spawn((
                WeaponSwing {
                    owner: entity,
                    elapsed_frames: 0,
                    total_frames: 8,
                    start_angle: base_angle + 1.2,
                    sweep_angle: -2.4,
                    radius: 35.0,
                },
                Sprite {
                    color: Color::srgba(0.9, 0.8, 0.5, 0.7),
                    custom_size: Some(Vec2::new(30.0, 6.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(screen.x, screen.y, 85.0)),
            ));
        }
    } else if is_moving {
        anim.frame_counter += 1;
        if anim.frame_counter >= anim.frames_per_step {
            anim.frame_counter = 0;
            anim.current_column = (anim.current_column + 1) % 8;
        }
    } else {
        anim.frame_counter = 0;
        anim.current_column = 0;
    }

    if let Some(ref mut atlas) = sprite.texture_atlas {
        atlas.index = (row * 8 + anim.current_column) as usize;
    }
}

/// Animate the weapon swing arc and despawn when finished.
fn weapon_swing_system(
    mut query: Query<(Entity, &mut WeaponSwing, &mut Transform, &mut Sprite)>,
    player_query: Query<&WorldPosition, With<Player>>,
    mut commands: Commands,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };
    let player_screen = crate::rendering::isometric::world_to_screen(player_pos.x, player_pos.y);

    for (entity, mut swing, mut transform, mut sprite) in &mut query {
        swing.elapsed_frames += 1;
        let t = swing.elapsed_frames as f32 / swing.total_frames as f32;

        // Sweep angle over time.
        let current_angle = swing.start_angle + swing.sweep_angle * t;
        let x = player_screen.x + current_angle.cos() * swing.radius;
        let y = player_screen.y + current_angle.sin() * swing.radius;
        transform.translation.x = x;
        transform.translation.y = y;
        transform.rotation = Quat::from_rotation_z(current_angle);

        // Fade out in second half and grow slightly.
        let alpha = if t > 0.5 { 1.0 - (t - 0.5) * 2.0 } else { 0.8 };
        let scale = 1.0 + t * 0.5;
        sprite.color = Color::srgba(0.9, 0.8, 0.5, alpha.max(0.0));
        sprite.custom_size = Some(Vec2::new(30.0 * scale, 6.0 * scale));

        if swing.elapsed_frames >= swing.total_frames {
            commands.entity(entity).despawn();
        }
    }
}

/// Convert a facing Direction to a Vec2.
fn facing_to_vec2(dir: &Direction) -> Vec2 {
    match dir {
        Direction::N => Vec2::new(0.0, 1.0),
        Direction::NE => Vec2::new(0.707, 0.707),
        Direction::E => Vec2::new(1.0, 0.0),
        Direction::SE => Vec2::new(0.707, -0.707),
        Direction::S => Vec2::new(0.0, -1.0),
        Direction::SW => Vec2::new(-0.707, -0.707),
        Direction::W => Vec2::new(-1.0, 0.0),
        Direction::NW => Vec2::new(-0.707, 0.707),
    }
}
