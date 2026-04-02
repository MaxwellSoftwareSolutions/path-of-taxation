use bevy::prelude::*;
use pot_shared::types::Direction;

use crate::app_state::AppState;
use crate::components::combat::{AbilityState, AnimationPhase, Cooldowns, Hurtbox, SelectedAbility};
use crate::components::player::*;
use crate::plugins::input::GameInput;
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
            .add_systems(Update, (
                player_movement_system,
                dodge_roll_system,
                player_facing_system,
                ability_slot_selection_system,
                invulnerability_tick_system,
            ).run_if(in_state(AppState::Run)))
            .add_systems(Update, (
                player_animation_system,
                weapon_swing_system,
            ).run_if(in_state(AppState::Run)));
    }
}

/// Spawn the player entity with all required components.
fn spawn_player(
    mut commands: Commands,
    sprite_assets: Res<SpriteAssets>,
    atlas_layout: Res<CharacterAtlasLayout>,
) {
    commands.spawn((
        // Identity and stats.
        Player,
        Health::default(),
        Mana::default(),
        MovementSpeed::default(),
        Facing::default(),
        DodgeState::default(),
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

/// Move the player based on input (60fps).
fn player_movement_system(
    game_input: Res<GameInput>,
    mut query: Query<(&MovementSpeed, &DodgeState, &AbilityState, &mut WorldPosition), With<Player>>,
    time: Res<Time>,
) {
    let Ok((speed, dodge, ability_state, mut world_pos)) = query.single_mut() else {
        return;
    };

    // Cannot move during active dodge or non-idle ability phase.
    if dodge.active || !ability_state.is_idle() {
        return;
    }

    let dir = game_input.move_direction;
    if dir == Vec2::ZERO {
        return;
    }

    let delta = dir * speed.0 * time.delta_secs();
    world_pos.x += delta.x;
    world_pos.y += delta.y;
}

/// Handle dodge roll initiation and progression.
fn dodge_roll_system(
    game_input: Res<GameInput>,
    mut query: Query<(
        &mut DodgeState,
        &mut Invulnerable,
        &mut WorldPosition,
        &AbilityState,
    ), With<Player>>,
    time: Res<Time>,
) {
    let Ok((mut dodge, mut invuln, mut world_pos, ability_state)) = query.single_mut() else {
        return;
    };

    // Initiate dodge if not already dodging and ability allows cancel.
    if !dodge.active && game_input.dodge_pressed && (ability_state.is_idle() || ability_state.can_cancel()) {
        let dir = if game_input.move_direction != Vec2::ZERO {
            game_input.move_direction.normalize()
        } else {
            // Default dodge direction: face direction or south.
            Vec2::new(0.0, -1.0)
        };
        dodge.active = true;
        dodge.frame = 0;
        dodge.direction = dir;
    }

    if !dodge.active {
        return;
    }

    // Apply dodge movement during active frames.
    if dodge.frame < DodgeState::ACTIVE_FRAMES {
        let delta = dodge.direction * dodge.speed * time.delta_secs();
        world_pos.x += delta.x;
        world_pos.y += delta.y;
        // Grant i-frames during active phase.
        invuln.frames_remaining = 2; // Refreshed each frame during active.
    }

    dodge.frame += 1;

    // End dodge.
    if dodge.is_finished() {
        dodge.active = false;
        dodge.frame = 0;
    }
}

/// Update player facing based on gamepad aim, movement direction, or mouse position.
fn player_facing_system(
    game_input: Res<GameInput>,
    mut query: Query<(&WorldPosition, &mut Facing), With<Player>>,
) {
    let Ok((world_pos, mut facing)) = query.single_mut() else {
        return;
    };

    // Priority: gamepad right stick > movement direction > mouse.
    let dir = if let Some(aim) = game_input.aim_direction {
        aim
    } else if game_input.move_direction != Vec2::ZERO {
        game_input.move_direction
    } else {
        // Mouse is in screen space; convert to approximate direction from player.
        let player_screen = crate::rendering::isometric::world_to_screen(world_pos.x, world_pos.y);
        let diff = game_input.mouse_world_pos - player_screen;
        if diff.length_squared() > 1.0 {
            diff.normalize()
        } else {
            return;
        }
    };

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
    game_input: Res<GameInput>,
    mut query: Query<(
        Entity,
        &Facing,
        &DodgeState,
        &AbilityState,
        &WorldPosition,
        &mut AnimationTimer,
        &mut Sprite,
    ), With<Player>>,
    swing_query: Query<&WeaponSwing>,
    mut commands: Commands,
) {
    let Ok((entity, facing, dodge, ability_state, world_pos, mut anim, mut sprite)) = query.single_mut() else {
        return;
    };

    let row = direction_to_flare_row(facing.0);

    let is_attacking = !ability_state.is_idle();
    let is_moving = game_input.move_direction != Vec2::ZERO
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
            let dir = facing_to_vec2(&facing.0);
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
