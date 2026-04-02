use bevy::prelude::*;

use crate::app_state::AppState;
use crate::components::combat::*;
use crate::components::enemy::Staggered;
use crate::components::player::*;
use crate::content::{AbilityDefs, CombatFeelConfig};
use crate::plugins::input::{DebugOverlayVisible, GameInput, InputBuffer};
use crate::plugins::patch_notes::PatchNoteModifiers;
use crate::plugins::ui::ComplianceMeter;
use crate::plugins::vfx::{
    DamageNumberMsg, HitFlashMsg, HitstopMsg, ParticleBurstMsg, ScreenFlashMsg,
};
use crate::plugins::camera::ShakeQueue;
use crate::rendering::isometric::WorldPosition;
use crate::rendering::sprites::SpriteAssets;
use pot_shared::ability_defs::AbilityType;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<HitMsg>()
            .add_systems(Update, (
                ability_input_system,
                ability_pipeline_system,
                cooldown_tick_system,
                hit_detection_system,
                damage_application_system,
                knockback_system,
                projectile_system,
                aoe_system,
                shield_tick_system,
                mana_regen_system,
                knockback_cleanup_system,
            ).chain().run_if(in_state(AppState::Run)))
            .add_systems(Update,
                debug_hitbox_visualization_system
                    .run_if(in_state(AppState::Run)),
            );
    }
}

/// Message fired when a hit is detected.
#[derive(Message, Clone, Debug)]
pub struct HitMsg {
    pub attacker: Entity,
    pub target: Entity,
    pub damage: f32,
    pub damage_type: pot_shared::types::DamageType,
    pub knockback_dir: Vec2,
    pub knockback_force: f32,
    pub is_critical: bool,
    pub position: Vec2,
}

/// Handle attack input: if attack is buffered and ability is idle/cancelable, start ability.
/// Deducts mana on cast. Sets per-ability cooldown from AbilityDef data.
fn ability_input_system(
    game_input: Res<GameInput>,
    mut buffer: ResMut<InputBuffer>,
    ability_defs: Res<AbilityDefs>,
    patch_mods: Option<Res<PatchNoteModifiers>>,
    mut compliance: ResMut<ComplianceMeter>,
    mut query: Query<(
        &mut AbilityState,
        &mut Cooldowns,
        &SelectedAbility,
        &mut DodgeState,
        &mut Mana,
        &WorldPosition,
    ), With<Player>>,
) {
    let Ok((mut ability_state, mut cooldowns, selected, mut dodge, mut mana, _world_pos)) = query.single_mut() else {
        return;
    };

    // Check for buffered attack.
    let wants_attack = game_input.attack_pressed || buffer.attack_buffered();

    if !wants_attack {
        return;
    }

    let slot = selected.0;

    // Can we start an ability?
    let can_start = ability_state.is_idle()
        || ability_state.can_cancel()
        || (dodge.can_cancel());

    if !can_start || !cooldowns.is_ready(slot) {
        return;
    }

    // Look up frame data from the loaded ability definitions.
    let (anticipation, active, recovery, cancel, mut mana_cost, cooldown_frames) =
        if let Some(def) = ability_defs.get_by_slot(slot) {
            // Convert cooldown_ms to frames (60fps = 16.67ms per frame).
            let cd_frames = (def.cooldown_ms as f32 / 16.67).round() as u32;
            (
                def.anticipation_frames,
                def.active_frames,
                def.recovery_frames,
                def.cancel_frame,
                def.mana_cost as f32,
                cd_frames,
            )
        } else {
            // Fallback if slot has no definition.
            (2, 3, 4, 6, 0.0, 60)
        };

    // Apply patch note mana cost modifier.
    if let Some(ref mods) = patch_mods {
        mana_cost *= mods.mana_cost_mult;
    }

    // Check mana.
    if mana.current < mana_cost {
        return;
    }

    // Cancel dodge if in cancel window.
    if dodge.active && dodge.can_cancel() {
        dodge.active = false;
        dodge.frame = 0;
    }

    // Deduct mana.
    mana.current -= mana_cost;

    // Increment compliance: using abilities shows compliance.
    compliance.add(1.0);

    // Set per-ability cooldown max before triggering.
    cooldowns.max_slots[slot] = cooldown_frames;

    ability_state.start_ability(slot, anticipation, active, recovery, cancel);
    cooldowns.trigger(slot);
    buffer.consume_attack();
}

/// Advance the ability pipeline each frame: anticipation -> active -> recovery -> idle.
/// On entering Active phase, spawn the appropriate entity type based on ability slot.
fn ability_pipeline_system(
    mut query: Query<(Entity, &mut AbilityState, &mut WorldPosition, &Facing), With<Player>>,
    ability_defs: Res<AbilityDefs>,
    mut commands: Commands,
    sprite_assets: Option<Res<SpriteAssets>>,
) {
    for (player_entity, mut state, mut world_pos, facing) in &mut query {
        if state.is_idle() {
            continue;
        }

        state.frame_in_phase += 1;

        match state.phase {
            AnimationPhase::Anticipation => {
                if state.frame_in_phase >= state.anticipation_frames {
                    state.phase = AnimationPhase::Active;
                    state.frame_in_phase = 0;

                    let slot = state.current_slot.unwrap_or(0);
                    let dir = facing_to_vec2(&facing.0);

                    // Look up the ability definition for this slot.
                    let ability_def = ability_defs.get_by_slot(slot);
                    let ability_type = ability_def.map(|d| d.ability_type).unwrap_or(AbilityType::Melee);

                    match ability_type {
                        AbilityType::Projectile => {
                            let def = ability_def.unwrap();
                            let count = def.projectile_count.unwrap_or(1);
                            let speed = def.projectile_speed.unwrap_or(800.0);
                            let spread_deg = def.projectile_spread_deg.unwrap_or(0.0);
                            let pierce = def.pierce_count.unwrap_or(0);
                            let lifetime_ms = def.projectile_lifetime_ms.unwrap_or(1200);
                            let lifetime_frames = (lifetime_ms as f32 / 16.67).round() as u32;
                            let base_damage = def.base_damage as f32;
                            let damage_type = def.damage_type;

                            // Spawn projectiles in a spread arc.
                            let base_angle = dir.y.atan2(dir.x);
                            let spread_rad = spread_deg.to_radians();

                            for i in 0..count {
                                let angle_offset = if count <= 1 {
                                    0.0
                                } else {
                                    let frac = i as f32 / (count - 1) as f32;
                                    -spread_rad / 2.0 + spread_rad * frac
                                };
                                let angle = base_angle + angle_offset;
                                let proj_dir = Vec2::new(angle.cos(), angle.sin());

                                let proj_pos = WorldPosition::new(
                                    world_pos.x + dir.x * 20.0,
                                    world_pos.y + dir.y * 20.0,
                                );

                                commands.spawn((
                                    Projectile {
                                        speed,
                                        direction: proj_dir,
                                        lifetime_frames,
                                        elapsed_frames: 0,
                                        pierce_remaining: pierce,
                                        radius: 8.0,
                                    },
                                    Hitbox {
                                        radius: 8.0,
                                        faction: pot_shared::types::Faction::Player,
                                        already_hit: Vec::new(),
                                    },
                                    Damage {
                                        amount: base_damage,
                                        damage_type,
                                        knockback_force: 80.0,
                                        knockback_dir: proj_dir,
                                        is_critical: false,
                                    },
                                    proj_pos,
                                    Sprite {
                                        color: Color::srgb(3.0, 2.0, 0.5), // bright HDR for bloom
                                        custom_size: Some(Vec2::new(16.0, 8.0)),
                                        ..default()
                                    },
                                    Transform::default(),
                                    HitboxLifetime { frames_remaining: lifetime_frames },
                                ));
                            }
                        }

                        AbilityType::AoE => {
                            let def = ability_def.unwrap();
                            let aoe_radius = def.aoe_radius.unwrap_or(128.0);
                            let duration_ms = def.aoe_duration_ms.unwrap_or(4000);
                            let tick_ms = def.aoe_tick_interval_ms.unwrap_or(500);
                            let lifetime_frames = (duration_ms as f32 / 16.67).round() as u32;
                            let tick_frames = (tick_ms as f32 / 16.67).round() as u32;
                            let base_damage = def.base_damage as f32;
                            let damage_type = def.damage_type;

                            // Spawn AoE zone 100 units in facing direction.
                            let aoe_pos = WorldPosition::new(
                                world_pos.x + dir.x * 100.0,
                                world_pos.y + dir.y * 100.0,
                            );

                            commands.spawn((
                                AoeZone {
                                    radius: aoe_radius,
                                    damage_per_tick: base_damage,
                                    damage_type,
                                    tick_interval_frames: tick_frames,
                                    frames_since_tick: tick_frames, // trigger first tick immediately
                                    lifetime_frames,
                                    elapsed_frames: 0,
                                },
                                Hitbox {
                                    radius: aoe_radius,
                                    faction: pot_shared::types::Faction::Player,
                                    already_hit: Vec::new(),
                                },
                                Damage {
                                    amount: base_damage,
                                    damage_type,
                                    knockback_force: 30.0,
                                    knockback_dir: dir,
                                    is_critical: false,
                                },
                                aoe_pos,
                                Sprite {
                                    color: Color::srgba(2.0, 0.3, 0.3, 0.25), // translucent red, HDR
                                    custom_size: Some(Vec2::splat(aoe_radius * 2.0)),
                                    ..default()
                                },
                                Transform::default(),
                                HitboxLifetime { frames_remaining: lifetime_frames },
                            ));
                        }

                        AbilityType::Shield => {
                            let def = ability_def.unwrap();
                            let shield_hp = def.shield_amount.unwrap_or(50) as f32;
                            let duration_ms = def.shield_duration_ms.unwrap_or(8000);
                            let duration_frames = (duration_ms as f32 / 16.67).round() as u32;

                            commands.entity(player_entity).insert(ShieldState {
                                amount: shield_hp,
                                max_amount: shield_hp,
                                absorbed: 0.0,
                                frames_remaining: duration_frames,
                            });
                        }

                        AbilityType::Channel => {
                            // Simplified beam: spawn a long thin hitbox extending in facing direction.
                            let def = ability_def.unwrap();
                            let base_damage = def.base_damage as f32;
                            let damage_type = def.damage_type;
                            let beam_length = 120.0;
                            let beam_radius = 30.0;

                            // Position the beam hitbox at the midpoint of the beam.
                            let beam_pos = WorldPosition::new(
                                world_pos.x + dir.x * (beam_length / 2.0),
                                world_pos.y + dir.y * (beam_length / 2.0),
                            );

                            // Use active_frames as lifetime for the beam hitbox.
                            let beam_lifetime = state.active_frames;

                            commands.spawn((
                                Hitbox {
                                    radius: beam_radius,
                                    faction: pot_shared::types::Faction::Player,
                                    already_hit: Vec::new(),
                                },
                                Damage {
                                    amount: base_damage,
                                    damage_type,
                                    knockback_force: 40.0,
                                    knockback_dir: dir,
                                    is_critical: false,
                                },
                                beam_pos,
                                Sprite {
                                    color: Color::srgba(0.5, 3.0, 3.0, 0.6), // cyan HDR beam
                                    custom_size: Some(Vec2::new(beam_length, 12.0)),
                                    ..default()
                                },
                                Transform::default(),
                                HitboxLifetime { frames_remaining: beam_lifetime },
                            ));
                        }

                        AbilityType::Teleport => {
                            let def = ability_def.unwrap();
                            let teleport_dist = def.teleport_range.unwrap_or(150.0);

                            // Save origin for AoE field.
                            let origin_x = world_pos.x;
                            let origin_y = world_pos.y;

                            // Teleport the player forward.
                            world_pos.x += dir.x * teleport_dist;
                            world_pos.y += dir.y * teleport_dist;

                            // Spawn a small AoE field at the origin position.
                            let aoe_radius = def.aoe_radius.unwrap_or(96.0);
                            let duration_ms = def.aoe_duration_ms.unwrap_or(3000);
                            let tick_ms = def.aoe_tick_interval_ms.unwrap_or(500);
                            let lifetime_frames = (duration_ms as f32 / 16.67).round() as u32;
                            let tick_frames = (tick_ms as f32 / 16.67).round() as u32;
                            let aoe_damage = 6.0; // small lingering field damage
                            let damage_type = def.damage_type;

                            commands.spawn((
                                AoeZone {
                                    radius: aoe_radius,
                                    damage_per_tick: aoe_damage,
                                    damage_type,
                                    tick_interval_frames: tick_frames,
                                    frames_since_tick: tick_frames,
                                    lifetime_frames,
                                    elapsed_frames: 0,
                                },
                                Hitbox {
                                    radius: aoe_radius,
                                    faction: pot_shared::types::Faction::Player,
                                    already_hit: Vec::new(),
                                },
                                Damage {
                                    amount: aoe_damage,
                                    damage_type,
                                    knockback_force: 30.0,
                                    knockback_dir: dir,
                                    is_critical: false,
                                },
                                WorldPosition::new(origin_x, origin_y),
                                Sprite {
                                    color: Color::srgba(1.5, 0.2, 2.0, 0.3), // purple HDR
                                    custom_size: Some(Vec2::splat(aoe_radius * 2.0)),
                                    ..default()
                                },
                                Transform::default(),
                                HitboxLifetime { frames_remaining: lifetime_frames },
                            ));
                        }

                        AbilityType::Melee => {
                            // Default melee hitbox (fallback).
                            let hitbox_pos = WorldPosition::new(
                                world_pos.x + dir.x * 30.0,
                                world_pos.y + dir.y * 30.0,
                            );
                            let base_damage = ability_def.map(|d| d.base_damage as f32).unwrap_or(25.0);
                            let damage_type = ability_def.map(|d| d.damage_type)
                                .unwrap_or(pot_shared::types::DamageType::Penalty);

                            commands.spawn((
                                Hitbox {
                                    radius: 20.0,
                                    faction: pot_shared::types::Faction::Player,
                                    already_hit: Vec::new(),
                                },
                                Damage {
                                    amount: base_damage,
                                    damage_type,
                                    knockback_force: 150.0,
                                    knockback_dir: dir,
                                    is_critical: false,
                                },
                                hitbox_pos,
                                {
                                    let mut s = Sprite {
                                        custom_size: Some(Vec2::new(40.0, 40.0)),
                                        ..default()
                                    };
                                    if let Some(ref assets) = sprite_assets {
                                        s.image = assets.slash.clone();
                                    } else {
                                        s.color = Color::srgba(3.0, 3.0, 0.5, 0.5);
                                    }
                                    s
                                },
                                Transform::default(),
                                HitboxLifetime { frames_remaining: 3 },
                            ));
                        }
                    }
                }
            }
            AnimationPhase::Active => {
                if state.frame_in_phase >= state.active_frames {
                    state.phase = AnimationPhase::Recovery;
                    state.frame_in_phase = 0;
                }
            }
            AnimationPhase::Recovery => {
                if state.frame_in_phase >= state.recovery_frames {
                    state.reset();
                }
            }
            AnimationPhase::Idle => {}
        }
    }
}

/// Temporary hitbox lifetime component.
#[derive(Component)]
pub struct HitboxLifetime {
    pub frames_remaining: u32,
}

/// Tick cooldowns for all entities with cooldowns.
fn cooldown_tick_system(mut query: Query<&mut Cooldowns>) {
    for mut cd in &mut query {
        cd.tick();
    }
}

/// Circle-circle collision detection between hitboxes and hurtboxes.
fn hit_detection_system(
    mut hitbox_query: Query<(Entity, &mut Hitbox, &Damage, &WorldPosition, &mut HitboxLifetime)>,
    hurtbox_query: Query<(Entity, &Hurtbox, &WorldPosition)>,
    mut hit_msgs: MessageWriter<HitMsg>,
    mut commands: Commands,
) {
    for (hb_entity, mut hitbox, damage, hb_pos, mut lifetime) in &mut hitbox_query {
        // Tick lifetime.
        lifetime.frames_remaining = lifetime.frames_remaining.saturating_sub(1);
        if lifetime.frames_remaining == 0 {
            commands.entity(hb_entity).despawn();
            continue;
        }

        for (hurt_entity, hurtbox, hurt_pos) in &hurtbox_query {
            // Skip same faction.
            if hitbox.faction == hurtbox.faction {
                continue;
            }

            // Skip already hit.
            if hitbox.already_hit.contains(&hurt_entity) {
                continue;
            }

            // Circle-circle collision.
            let dist = hb_pos.distance_to(hurt_pos);
            if dist <= hitbox.radius + hurtbox.radius {
                hitbox.already_hit.push(hurt_entity);

                let dir = if dist > 0.01 {
                    Vec2::new(hurt_pos.x - hb_pos.x, hurt_pos.y - hb_pos.y).normalize()
                } else {
                    damage.knockback_dir
                };

                hit_msgs.write(HitMsg {
                    attacker: hb_entity,
                    target: hurt_entity,
                    damage: damage.amount,
                    damage_type: damage.damage_type,
                    knockback_dir: dir,
                    knockback_force: damage.knockback_force,
                    is_critical: damage.is_critical,
                    position: Vec2::new(hurt_pos.x, hurt_pos.y),
                });
            }
        }
    }
}

/// Apply damage from hit messages, spawn VFX messages.
/// Checks for ShieldState on the target -- shield absorbs damage before HP.
fn damage_application_system(
    mut hit_msgs: MessageReader<HitMsg>,
    mut health_query: Query<(&mut Health, Option<&mut Invulnerable>, Option<&mut ShieldState>)>,
    player_query: Query<Entity, With<Player>>,
    feel: Res<CombatFeelConfig>,
    mut compliance: ResMut<ComplianceMeter>,
    mut hitstop_msgs: MessageWriter<HitstopMsg>,
    mut hit_flash_msgs: MessageWriter<HitFlashMsg>,
    mut damage_number_msgs: MessageWriter<DamageNumberMsg>,
    mut particle_msgs: MessageWriter<ParticleBurstMsg>,
    mut screen_flash_msgs: MessageWriter<ScreenFlashMsg>,
    mut shake_queue: ResMut<ShakeQueue>,
    mut commands: Commands,
) {
    let player_entity = player_query.single().ok();

    for hit in hit_msgs.read() {
        // Check invulnerability.
        if let Ok((mut health, invuln, shield)) = health_query.get_mut(hit.target) {
            if let Some(invuln) = invuln {
                if invuln.frames_remaining > 0 {
                    continue;
                }
            }

            let mut remaining_damage = hit.damage;

            // Shield absorbs damage first.
            if let Some(mut shield) = shield {
                if shield.amount > 0.0 {
                    let absorbed = remaining_damage.min(shield.amount);
                    shield.amount -= absorbed;
                    shield.absorbed += absorbed;
                    remaining_damage -= absorbed;

                    // If shield breaks, remove it (will be cleaned up by shield_tick_system).
                }
            }

            health.current -= remaining_damage;

            // Apply knockback.
            commands.entity(hit.target).insert(Knockback {
                direction: hit.knockback_dir,
                initial_force: hit.knockback_force,
                elapsed_frames: 0,
                total_frames: feel.knockback_total_frames,
            });

            // Apply stagger to enemies.
            commands.entity(hit.target).insert(Staggered { frames_remaining: feel.stagger_frames });

            // Send VFX messages using combat feel config.
            let hitstop_frames = if hit.is_critical { feel.hitstop_crit_frames } else { feel.hitstop_normal_frames };
            hitstop_msgs.write(HitstopMsg {
                attacker: hit.attacker,
                target: hit.target,
                freeze_frames: hitstop_frames,
            });

            hit_flash_msgs.write(HitFlashMsg {
                entity: hit.target,
                flash_frames: feel.hit_flash_frames,
            });

            damage_number_msgs.write(DamageNumberMsg {
                position: hit.position,
                amount: hit.damage,
                is_critical: hit.is_critical,
            });

            // HDR particle colors for bloom glow.
            let particle_count = if hit.is_critical { feel.particle_crit_count } else { feel.particle_normal_count };
            particle_msgs.write(ParticleBurstMsg {
                position: hit.position,
                direction: hit.knockback_dir,
                count: particle_count,
                color: Color::srgb(2.0, 0.6, 0.6),
            });

            // Screen shake from combat feel config.
            let shake_intensity = if hit.is_critical { feel.shake_crit_intensity } else { feel.shake_normal_intensity };
            let shake_frames = if hit.is_critical { feel.shake_crit_frames } else { feel.shake_normal_frames };
            shake_queue.push(hit.knockback_dir, shake_intensity, shake_frames);

            // Red screen flash when the player takes damage.
            if Some(hit.target) == player_entity {
                screen_flash_msgs.write(ScreenFlashMsg {
                    color: Color::srgba(0.6, 0.05, 0.05, 0.25),
                });
                // Taking damage decreases compliance.
                compliance.add(-2.0);
            }
        }
    }
}

/// Apply knockback movement with ease-out curve.
fn knockback_system(
    mut query: Query<(&mut Knockback, &mut WorldPosition)>,
    time: Res<Time>,
) {
    for (mut kb, mut world_pos) in &mut query {
        let force = kb.current_force();
        let delta = kb.direction * force * time.delta_secs();
        world_pos.x += delta.x;
        world_pos.y += delta.y;

        kb.elapsed_frames += 1;
    }
}

/// Move projectiles and despawn when lifetime expires.
fn projectile_system(
    mut query: Query<(Entity, &mut Projectile, &mut WorldPosition)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut proj, mut world_pos) in &mut query {
        let delta = proj.direction * proj.speed * time.delta_secs();
        world_pos.x += delta.x;
        world_pos.y += delta.y;

        proj.elapsed_frames += 1;
        if proj.elapsed_frames >= proj.lifetime_frames {
            commands.entity(entity).despawn();
        }
    }
}

/// Tick AoE zones: advance timers, reset hitbox on each tick so AoE can multi-hit.
fn aoe_system(
    mut query: Query<(Entity, &mut AoeZone, &mut Hitbox, &mut Damage)>,
    mut commands: Commands,
) {
    for (entity, mut zone, mut hitbox, mut damage) in &mut query {
        zone.elapsed_frames += 1;
        zone.frames_since_tick += 1;

        if zone.elapsed_frames >= zone.lifetime_frames {
            commands.entity(entity).despawn();
            continue;
        }

        // On each tick interval, clear already_hit so AoE can damage again.
        if zone.frames_since_tick >= zone.tick_interval_frames {
            zone.frames_since_tick = 0;
            hitbox.already_hit.clear();
            damage.amount = zone.damage_per_tick;
        }

        // Pulse the sprite alpha for visual feedback (handled via Sprite, but
        // we can modulate damage.amount is already set above).
    }
}

/// Remove finished knockback components to avoid stale data.
fn knockback_cleanup_system(
    mut commands: Commands,
    query: Query<(Entity, &Knockback)>,
) {
    for (entity, kb) in &query {
        if kb.is_finished() {
            commands.entity(entity).remove::<Knockback>();
        }
    }
}

/// Tick shield duration. Decrement frames, remove when expired or broken, heal 30% of absorbed.
fn shield_tick_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut ShieldState, &mut Health)>,
) {
    for (entity, mut shield, mut health) in &mut query {
        shield.frames_remaining = shield.frames_remaining.saturating_sub(1);

        if shield.frames_remaining == 0 || shield.amount <= 0.0 {
            // Shield expired or broke: heal 30% of absorbed damage.
            let heal = shield.absorbed * 0.3;
            health.current = (health.current + heal).min(health.max);
            commands.entity(entity).remove::<ShieldState>();
        }
    }
}

/// Regenerate mana over time: 5 mana per second.
fn mana_regen_system(
    mut query: Query<&mut Mana, With<Player>>,
    time: Res<Time>,
) {
    for mut mana in &mut query {
        mana.current = (mana.current + 5.0 * time.delta_secs()).min(mana.max);
    }
}

/// Marker for debug hitbox/hurtbox visualization circles.
#[derive(Component)]
pub struct DebugCollisionCircle;

/// Show hitbox and hurtbox circles when debug overlay (F1) is active.
/// Spawns/despawns wireframe circles each frame based on debug visibility.
fn debug_hitbox_visualization_system(
    debug_vis: Res<DebugOverlayVisible>,
    existing_circles: Query<Entity, With<DebugCollisionCircle>>,
    hitbox_query: Query<(&Hitbox, &WorldPosition)>,
    hurtbox_query: Query<(&Hurtbox, &WorldPosition)>,
    mut commands: Commands,
) {
    // Always clean up old debug circles.
    for entity in &existing_circles {
        commands.entity(entity).despawn();
    }

    if !debug_vis.0 {
        return;
    }

    // Spawn debug circles for active hitboxes (yellow).
    for (hitbox, world_pos) in &hitbox_query {
        let screen = crate::rendering::isometric::world_to_screen(world_pos.x, world_pos.y);
        let diameter = hitbox.radius * 2.0;
        commands.spawn((
            DebugCollisionCircle,
            Sprite {
                color: Color::srgba(1.0, 1.0, 0.0, 0.3),
                custom_size: Some(Vec2::splat(diameter)),
                ..default()
            },
            Transform::from_translation(Vec3::new(screen.x, screen.y, 200.0)),
        ));
    }

    // Spawn debug circles for hurtboxes (cyan for player, red for enemy).
    for (hurtbox, world_pos) in &hurtbox_query {
        let screen = crate::rendering::isometric::world_to_screen(world_pos.x, world_pos.y);
        let diameter = hurtbox.radius * 2.0;
        let color = match hurtbox.faction {
            pot_shared::types::Faction::Player => Color::srgba(0.0, 1.0, 1.0, 0.3),
            pot_shared::types::Faction::Enemy => Color::srgba(1.0, 0.2, 0.2, 0.3),
            _ => Color::srgba(0.5, 0.5, 0.5, 0.3),
        };
        commands.spawn((
            DebugCollisionCircle,
            Sprite {
                color,
                custom_size: Some(Vec2::splat(diameter)),
                ..default()
            },
            Transform::from_translation(Vec3::new(screen.x, screen.y, 200.0)),
        ));
    }
}

/// Convert a facing direction to a Vec2.
fn facing_to_vec2(dir: &pot_shared::types::Direction) -> Vec2 {
    use pot_shared::types::Direction::*;
    match dir {
        N => Vec2::new(0.0, 1.0),
        NE => Vec2::new(0.707, 0.707),
        E => Vec2::new(1.0, 0.0),
        SE => Vec2::new(0.707, -0.707),
        S => Vec2::new(0.0, -1.0),
        SW => Vec2::new(-0.707, -0.707),
        W => Vec2::new(-1.0, 0.0),
        NW => Vec2::new(-0.707, 0.707),
    }
}
