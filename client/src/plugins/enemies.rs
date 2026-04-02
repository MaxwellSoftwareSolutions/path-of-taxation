use bevy::prelude::*;

use pot_shared::constants::MAX_ACTIVE_ATTACKERS;
use pot_shared::enemy_defs::EnemyBehavior;
use pot_shared::types::{AiState, Direction};

use crate::app_state::AppState;
use crate::components::combat::{Damage, Hitbox, Hurtbox, Projectile};
use crate::components::enemy::*;
use crate::components::player::{AnimationTimer, Health, Player};
use crate::content::CombatFeelConfig;
use crate::plugins::combat::HitboxLifetime;
use crate::plugins::run::{ArenaCollision, resolve_world_collision};
use crate::plugins::vfx::gameplay_unfrozen;
use crate::plugins::vfx::{KillSlowMoMsg, ParticleBurstMsg};
use crate::rendering::isometric::WorldPosition;
use crate::rendering::sprites::{SpriteAssets, CharacterAtlasLayout};

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnEnemyMsg>()
            .add_message::<EnemyDeathMsg>()
            .add_systems(Update, (
                enemy_spawn_system,
                enemy_ai_system,
                enemy_attack_hitbox_system,
                enemy_telegraph_system,
                crowd_management_system,
                stagger_system,
                enemy_death_system,
                dying_dissolution_system,
            ).chain().run_if(gameplay_unfrozen).run_if(in_state(AppState::Run)))
            .add_systems(Update, enemy_animation_system.run_if(gameplay_unfrozen).run_if(in_state(AppState::Run)));
    }
}

/// Message to request spawning an enemy.
#[derive(Message, Clone, Debug)]
pub struct SpawnEnemyMsg {
    pub enemy_type: String,
    pub position: Vec2,
    pub hp: f32,
    pub damage: f32,
    pub speed: f32,
    pub aggro_range: f32,
    pub attack_range: f32,
    pub attack_cooldown_frames: u32,
    pub windup_frames: u32,
    pub behavior: EnemyBehavior,
}

/// Message fired when an enemy dies.
#[derive(Message, Clone, Debug)]
pub struct EnemyDeathMsg {
    pub entity: Entity,
    pub position: Vec2,
}

/// Spawn enemies from messages, alternating between sprite variants.
fn enemy_spawn_system(
    mut commands: Commands,
    mut spawn_msgs: MessageReader<SpawnEnemyMsg>,
    sprite_assets: Res<SpriteAssets>,
    atlas_layout: Res<CharacterAtlasLayout>,
    existing_enemies: Query<Entity, With<Enemy>>,
) {
    let base_count = existing_enemies.iter().count();
    for (i, msg) in spawn_msgs.read().enumerate() {
        // Alternate between the two enemy sprite types.
        let is_accountant = (base_count + i) % 2 == 1;
        let sprite_handle = if is_accountant {
            sprite_assets.enemy_undead_accountant.clone()
        } else {
            sprite_assets.enemy_tax_collector.clone()
        };

        commands.spawn((
            // Identity and AI.
            Enemy,
            EnemyType(msg.enemy_type.clone()),
            BehaviorType(msg.behavior),
            MoveSpeed(msg.speed),
            EnemyDamage(msg.damage),
            AiBehavior::default(),
            AggroRange(msg.aggro_range),
            AttackCooldown {
                current_frames: 0,
                max_frames: msg.attack_cooldown_frames,
            },
            AttackWindupFrames(msg.windup_frames),
            AttackRange(msg.attack_range),
            Variant::default(),
            // Combat, animation, and rendering.
            (
                AnimationTimer::default(),
                Health { current: msg.hp, max: msg.hp },
                WorldPosition::new(msg.position.x, msg.position.y),
                Hurtbox { radius: 14.0, faction: pot_shared::types::Faction::Enemy },
                Sprite {
                    image: sprite_handle,
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
}

/// Enemy AI: behavior-specific movement and attack patterns.
fn enemy_ai_system(
    mut enemy_query: Query<(
        Entity,
        &mut AiBehavior,
        &BehaviorType,
        &MoveSpeed,
        &AggroRange,
        &AttackRange,
        &AttackWindupFrames,
        &mut AttackCooldown,
        &mut WorldPosition,
        Option<&Staggered>,
        Option<&ActiveAttacker>,
    ), (With<Enemy>, Without<Dying>, Without<Player>)>,
    player_query: Query<&WorldPosition, With<Player>>,
    arena: Option<Res<ArenaCollision>>,
    mut commands: Commands,
    time: Res<Time>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    for (
        entity,
        mut ai,
        behavior,
        move_speed,
        aggro,
        attack_range,
        windup,
        mut attack_cd,
        mut world_pos,
        staggered,
        active_attacker,
    ) in &mut enemy_query
    {
        // Skip if staggered.
        if staggered.is_some_and(|s| s.frames_remaining > 0) {
            continue;
        }

        attack_cd.tick();
        ai.state_timer_frames += 1;

        let dx = player_pos.x - world_pos.x;
        let dy = player_pos.y - world_pos.y;
        let dist = (dx * dx + dy * dy).sqrt();

        match behavior.0 {
            EnemyBehavior::Chase => {
                ai_chase(&mut ai, &mut world_pos, &mut attack_cd, &mut commands,
                    entity, aggro, attack_range, move_speed.0, dist, dx, dy,
                    active_attacker.is_some(), time.delta_secs(), windup.0);
            }
            EnemyBehavior::Shamble => {
                let shamble_speed = move_speed.0 * 0.6;
                ai_chase(&mut ai, &mut world_pos, &mut attack_cd, &mut commands,
                    entity, aggro, attack_range, shamble_speed, dist, dx, dy,
                    active_attacker.is_some(), time.delta_secs(), windup.0);
            }
            EnemyBehavior::Swarm => {
                // Faster (130% speed), flanking approach angle offset by entity index.
                let swarm_speed = move_speed.0 * 1.3;
                let flank_offset = swarm_flank_angle(entity);
                ai_swarm(&mut ai, &mut world_pos, &mut attack_cd, &mut commands,
                    entity, aggro, attack_range, swarm_speed, dist, dx, dy,
                    active_attacker.is_some(), time.delta_secs(), flank_offset, windup.0);
            }
            EnemyBehavior::Ranged => {
                ai_ranged(&mut ai, &mut world_pos, &mut attack_cd, &mut commands,
                    entity, aggro, attack_range, move_speed.0, dist, dx, dy,
                    active_attacker.is_some(), time.delta_secs(), windup.0);
            }
            EnemyBehavior::Kiter => {
                ai_kiter(&mut ai, &mut world_pos, &mut attack_cd, &mut commands,
                    entity, aggro, attack_range, move_speed.0, dist, dx, dy,
                    player_pos, active_attacker.is_some(), time.delta_secs(), windup.0);
            }
            EnemyBehavior::Debuffer => {
                // Debuffers use ranged AI (fires projectiles from range).
                // Full debuff effects (slow, silence) are a future task.
                ai_ranged(&mut ai, &mut world_pos, &mut attack_cd, &mut commands,
                    entity, aggro, attack_range, move_speed.0, dist, dx, dy,
                    active_attacker.is_some(), time.delta_secs(), windup.0);
            }
            EnemyBehavior::Stationary => {
                // Stationary enemies don't move; they attack when player is in range.
                ai_stationary(&mut ai, &mut attack_cd, &mut commands,
                    entity, aggro, attack_range, dist, windup.0);
            }
        }

        if let Some(arena) = &arena {
            let resolved =
                resolve_world_collision(Vec2::new(world_pos.x, world_pos.y), 14.0, arena);
            world_pos.x = resolved.x;
            world_pos.y = resolved.y;
        }
    }
}

/// Chase AI: move toward player, windup, strike.
fn ai_chase(
    ai: &mut AiBehavior,
    world_pos: &mut WorldPosition,
    attack_cd: &mut AttackCooldown,
    commands: &mut Commands,
    entity: Entity,
    aggro: &AggroRange,
    attack_range: &AttackRange,
    speed: f32,
    dist: f32,
    dx: f32,
    dy: f32,
    is_active_attacker: bool,
    delta_secs: f32,
    windup_frames: u32,
) {
    match ai.state {
        AiState::Idle | AiState::Patrol => {
            if dist <= aggro.0 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Chase => {
            if is_active_attacker {
                if dist > attack_range.0 {
                    if dist > 0.01 {
                        let dir_x = dx / dist;
                        let dir_y = dy / dist;
                        world_pos.x += dir_x * speed * delta_secs;
                        world_pos.y += dir_y * speed * delta_secs;
                    }
                } else if attack_cd.is_ready() {
                    ai.state = AiState::Windup;
                    ai.state_timer_frames = 0;
                }
            } else {
                let orbit_dist = aggro.0 * 0.6;
                if dist < orbit_dist && dist > 0.01 {
                    let orbit_speed = speed * 0.5;
                    world_pos.x -= (dx / dist) * orbit_speed * delta_secs;
                    world_pos.y -= (dy / dist) * orbit_speed * delta_secs;
                }
            }
        }
        AiState::Windup => {
            if ai.state_timer_frames >= windup_frames {
                ai.state = AiState::Attack;
                ai.state_timer_frames = 0;
                commands.entity(entity).remove::<AttackFired>();
            }
        }
        AiState::Attack => {
            if ai.state_timer_frames >= 4 {
                ai.state = AiState::Recover;
                ai.state_timer_frames = 0;
                attack_cd.trigger();
            }
        }
        AiState::Recover => {
            if ai.state_timer_frames >= 12 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Staggered => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
        AiState::Flee => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
    }
}

/// Compute a flanking angle offset from the entity's ID bits.
fn swarm_flank_angle(entity: Entity) -> f32 {
    // Use the low bits of the entity ID for a deterministic per-entity offset.
    let idx = (entity.to_bits() & 0xFF) as f32;
    idx * std::f32::consts::TAU / 5.0
}

/// Swarm AI: fast approach with flanking offset, melee attack.
fn ai_swarm(
    ai: &mut AiBehavior,
    world_pos: &mut WorldPosition,
    attack_cd: &mut AttackCooldown,
    commands: &mut Commands,
    entity: Entity,
    aggro: &AggroRange,
    attack_range: &AttackRange,
    speed: f32,
    dist: f32,
    dx: f32,
    dy: f32,
    is_active_attacker: bool,
    delta_secs: f32,
    flank_offset: f32,
    windup_frames: u32,
) {
    match ai.state {
        AiState::Idle | AiState::Patrol => {
            if dist <= aggro.0 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Chase => {
            if is_active_attacker {
                if dist > attack_range.0 {
                    if dist > 0.01 {
                        // Offset the approach direction by flank_offset radians.
                        let base_angle = dy.atan2(dx);
                        let flanked_angle = base_angle + flank_offset;
                        let dir_x = flanked_angle.cos();
                        let dir_y = flanked_angle.sin();
                        world_pos.x += dir_x * speed * delta_secs;
                        world_pos.y += dir_y * speed * delta_secs;
                    }
                } else if attack_cd.is_ready() {
                    ai.state = AiState::Windup;
                    ai.state_timer_frames = 0;
                }
            } else {
                // Swarm enemies still approach even when not active, just slower.
                if dist > attack_range.0 * 2.0 && dist > 0.01 {
                    let base_angle = dy.atan2(dx);
                    let flanked_angle = base_angle + flank_offset;
                    let slow_speed = speed * 0.4;
                    world_pos.x += flanked_angle.cos() * slow_speed * delta_secs;
                    world_pos.y += flanked_angle.sin() * slow_speed * delta_secs;
                }
            }
        }
        AiState::Windup => {
            if ai.state_timer_frames >= windup_frames {
                ai.state = AiState::Attack;
                ai.state_timer_frames = 0;
                commands.entity(entity).remove::<AttackFired>();
            }
        }
        AiState::Attack => {
            if ai.state_timer_frames >= 4 {
                ai.state = AiState::Recover;
                ai.state_timer_frames = 0;
                attack_cd.trigger();
            }
        }
        AiState::Recover => {
            if ai.state_timer_frames >= 8 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Staggered => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
        AiState::Flee => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
    }
}

/// Ranged AI: maintain distance at aggro_range * 0.7, fire projectile when in range.
fn ai_ranged(
    ai: &mut AiBehavior,
    world_pos: &mut WorldPosition,
    attack_cd: &mut AttackCooldown,
    commands: &mut Commands,
    entity: Entity,
    aggro: &AggroRange,
    attack_range: &AttackRange,
    speed: f32,
    dist: f32,
    dx: f32,
    dy: f32,
    is_active_attacker: bool,
    delta_secs: f32,
    windup_frames: u32,
) {
    let preferred_dist = aggro.0 * 0.7;

    match ai.state {
        AiState::Idle | AiState::Patrol => {
            if dist <= aggro.0 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Chase => {
            if is_active_attacker {
                if dist > 0.01 {
                    if dist > attack_range.0 {
                        // Too far -- move closer.
                        world_pos.x += (dx / dist) * speed * delta_secs;
                        world_pos.y += (dy / dist) * speed * delta_secs;
                    } else if dist < preferred_dist * 0.8 {
                        // Too close -- back away.
                        world_pos.x -= (dx / dist) * speed * delta_secs;
                        world_pos.y -= (dy / dist) * speed * delta_secs;
                    }
                }

                if dist <= attack_range.0 && attack_cd.is_ready() {
                    ai.state = AiState::Windup;
                    ai.state_timer_frames = 0;
                }
            } else {
                // Orbit at preferred distance.
                if dist < preferred_dist * 0.6 && dist > 0.01 {
                    world_pos.x -= (dx / dist) * speed * 0.5 * delta_secs;
                    world_pos.y -= (dy / dist) * speed * 0.5 * delta_secs;
                }
            }
        }
        AiState::Windup => {
            if ai.state_timer_frames >= windup_frames {
                ai.state = AiState::Attack;
                ai.state_timer_frames = 0;
                commands.entity(entity).remove::<AttackFired>();
            }
        }
        AiState::Attack => {
            if ai.state_timer_frames >= 4 {
                ai.state = AiState::Recover;
                ai.state_timer_frames = 0;
                attack_cd.trigger();
            }
        }
        AiState::Recover => {
            if ai.state_timer_frames >= 12 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Staggered => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
        AiState::Flee => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
    }
}

/// Kiter AI: attack then strafe perpendicular to player, maintain distance.
fn ai_kiter(
    ai: &mut AiBehavior,
    world_pos: &mut WorldPosition,
    attack_cd: &mut AttackCooldown,
    commands: &mut Commands,
    entity: Entity,
    aggro: &AggroRange,
    attack_range: &AttackRange,
    speed: f32,
    dist: f32,
    dx: f32,
    dy: f32,
    _player_pos: &WorldPosition,
    is_active_attacker: bool,
    delta_secs: f32,
    windup_frames: u32,
) {
    let preferred_dist = aggro.0 * 0.7;

    match ai.state {
        AiState::Idle | AiState::Patrol => {
            if dist <= aggro.0 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Chase => {
            if is_active_attacker {
                if dist > 0.01 {
                    if dist > attack_range.0 {
                        world_pos.x += (dx / dist) * speed * delta_secs;
                        world_pos.y += (dy / dist) * speed * delta_secs;
                    } else if dist < preferred_dist * 0.6 {
                        world_pos.x -= (dx / dist) * speed * delta_secs;
                        world_pos.y -= (dy / dist) * speed * delta_secs;
                    }
                }

                if dist <= attack_range.0 && attack_cd.is_ready() {
                    ai.state = AiState::Windup;
                    ai.state_timer_frames = 0;
                }
            } else {
                if dist < preferred_dist * 0.5 && dist > 0.01 {
                    world_pos.x -= (dx / dist) * speed * 0.5 * delta_secs;
                    world_pos.y -= (dy / dist) * speed * 0.5 * delta_secs;
                }
            }
        }
        AiState::Windup => {
            if ai.state_timer_frames >= windup_frames {
                ai.state = AiState::Attack;
                ai.state_timer_frames = 0;
                commands.entity(entity).remove::<AttackFired>();
            }
        }
        AiState::Attack => {
            if ai.state_timer_frames >= 4 {
                ai.state = AiState::Recover;
                ai.state_timer_frames = 0;
                attack_cd.trigger();
            }
        }
        AiState::Recover => {
            // Strafe perpendicular to player direction during recovery.
            if dist > 0.01 {
                let perp_x = -dy / dist;
                let perp_y = dx / dist;
                // Also move away slightly to maintain distance.
                let away_x = -(dx / dist);
                let away_y = -(dy / dist);
                let strafe_speed = speed * 1.2;
                world_pos.x += (perp_x * 0.7 + away_x * 0.3) * strafe_speed * delta_secs;
                world_pos.y += (perp_y * 0.7 + away_y * 0.3) * strafe_speed * delta_secs;
            }

            if ai.state_timer_frames >= 18 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Staggered => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
        AiState::Flee => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
    }
}

/// Stationary AI: don't move, attack when player is within range.
fn ai_stationary(
    ai: &mut AiBehavior,
    attack_cd: &mut AttackCooldown,
    commands: &mut Commands,
    entity: Entity,
    aggro: &AggroRange,
    attack_range: &AttackRange,
    dist: f32,
    windup_frames: u32,
) {
    match ai.state {
        AiState::Idle | AiState::Patrol => {
            if dist <= aggro.0 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Chase => {
            if dist <= attack_range.0 && attack_cd.is_ready() {
                ai.state = AiState::Windup;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Windup => {
            if ai.state_timer_frames >= windup_frames {
                ai.state = AiState::Attack;
                ai.state_timer_frames = 0;
                commands.entity(entity).remove::<AttackFired>();
            }
        }
        AiState::Attack => {
            if ai.state_timer_frames >= 4 {
                ai.state = AiState::Recover;
                ai.state_timer_frames = 0;
                attack_cd.trigger();
            }
        }
        AiState::Recover => {
            if ai.state_timer_frames >= 12 {
                ai.state = AiState::Chase;
                ai.state_timer_frames = 0;
            }
        }
        AiState::Staggered => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
        AiState::Flee => {
            ai.state = AiState::Chase;
            ai.state_timer_frames = 0;
        }
    }
}

/// Spawn enemy attack hitboxes (melee) or projectiles (ranged) during Attack state.
/// Only fires once per attack cycle (guarded by AttackFired marker).
fn enemy_attack_hitbox_system(
    mut enemy_query: Query<(
        Entity,
        &AiBehavior,
        &BehaviorType,
        &AttackRange,
        &EnemyDamage,
        &WorldPosition,
        Option<&AttackFired>,
    ), (With<Enemy>, Without<Dying>)>,
    player_query: Query<&WorldPosition, With<Player>>,
    mut commands: Commands,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    for (entity, ai, behavior, attack_range, enemy_dmg, world_pos, attack_fired) in &mut enemy_query {
        if ai.state != AiState::Attack || attack_fired.is_some() {
            continue;
        }

        // Mark this attack cycle as fired so we don't spawn again.
        commands.entity(entity).insert(AttackFired);

        // Direction from enemy toward player.
        let dx = player_pos.x - world_pos.x;
        let dy = player_pos.y - world_pos.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let dir = if dist > 0.01 {
            Vec2::new(dx / dist, dy / dist)
        } else {
            Vec2::new(0.0, -1.0)
        };

        let is_ranged = matches!(
            behavior.0,
            EnemyBehavior::Ranged | EnemyBehavior::Kiter | EnemyBehavior::Debuffer
        );

        if is_ranged {
            // Spawn a projectile aimed at the player.
            let proj_pos = WorldPosition::new(
                world_pos.x + dir.x * 15.0,
                world_pos.y + dir.y * 15.0,
            );

            // Dark red/purple for enemy projectiles.
            let proj_color = match behavior.0 {
                EnemyBehavior::Debuffer => Color::srgba(0.6, 0.1, 0.8, 0.8), // purple
                _ => Color::srgba(0.7, 0.1, 0.2, 0.8),                       // dark red
            };

            commands.spawn((
                Projectile {
                    speed: 200.0,
                    direction: dir,
                    lifetime_frames: 120,
                    elapsed_frames: 0,
                    pierce_remaining: 0,
                    radius: 10.0,
                },
                Hitbox {
                    radius: 10.0,
                    faction: pot_shared::types::Faction::Enemy,
                    already_hit: Vec::new(),
                },
                Damage {
                    amount: enemy_dmg.0,
                    damage_type: pot_shared::types::DamageType::Penalty,
                    knockback_force: 60.0,
                    knockback_dir: dir,
                    is_critical: false,
                },
                proj_pos,
                Sprite {
                    color: proj_color,
                    custom_size: Some(Vec2::new(20.0, 20.0)),
                    ..default()
                },
                Transform::default(),
                HitboxLifetime { frames_remaining: 120 },
            ));
        } else {
            // Melee hitbox (Chase, Shamble, Swarm, Stationary).
            let strike_distance = attack_range.0.clamp(24.0, 72.0) * 0.58;
            let strike_radius = (attack_range.0 * 0.52).clamp(18.0, 46.0);
            let hitbox_pos = WorldPosition::new(
                world_pos.x + dir.x * strike_distance,
                world_pos.y + dir.y * strike_distance,
            );

            commands.spawn((
                Hitbox {
                    radius: strike_radius,
                    faction: pot_shared::types::Faction::Enemy,
                    already_hit: Vec::new(),
                },
                Damage {
                    amount: enemy_dmg.0,
                    damage_type: pot_shared::types::DamageType::Penalty,
                    knockback_force: 100.0,
                    knockback_dir: dir,
                    is_critical: false,
                },
                hitbox_pos,
                Sprite {
                    color: Color::srgba(1.0, 0.0, 0.0, 0.4),
                    custom_size: Some(Vec2::splat(strike_radius * 2.0)),
                    ..default()
                },
                Transform::default(),
                HitboxLifetime { frames_remaining: 4 },
            ));
        }
    }
}

/// Manage attack telegraph ground indicators.
/// Spawns a red circle during Windup, despawns it when leaving Windup.
fn enemy_telegraph_system(
    enemy_query: Query<(Entity, &AiBehavior, &BehaviorType, &AttackRange, &WorldPosition), (With<Enemy>, Without<Dying>)>,
    player_query: Query<&WorldPosition, With<Player>>,
    telegraph_query: Query<(Entity, &AttackTelegraph)>,
    mut commands: Commands,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    // Collect which enemies currently have telegraphs.
    let telegraphed_owners: Vec<(Entity, Entity)> = telegraph_query
        .iter()
        .map(|(te, at)| (at.owner, te))
        .collect();

    for (entity, ai, behavior, attack_range, world_pos) in &enemy_query {
        let has_telegraph = telegraphed_owners.iter().find(|(owner, _)| *owner == entity);

        if ai.state == AiState::Windup {
            if has_telegraph.is_none() {
                // Spawn telegraph at a position between enemy and player.
                let dx = player_pos.x - world_pos.x;
                let dy = player_pos.y - world_pos.y;
                let dist = (dx * dx + dy * dy).sqrt();
                let dir = if dist > 0.01 {
                    Vec2::new(dx / dist, dy / dist)
                } else {
                    Vec2::new(0.0, -1.0)
                };

                let telegraph_distance = if matches!(
                    behavior.0,
                    EnemyBehavior::Ranged | EnemyBehavior::Kiter | EnemyBehavior::Debuffer
                ) {
                    attack_range.0.clamp(32.0, 120.0) * 0.35
                } else {
                    attack_range.0.clamp(24.0, 72.0) * 0.58
                };
                let telegraph_size = if matches!(
                    behavior.0,
                    EnemyBehavior::Ranged | EnemyBehavior::Kiter | EnemyBehavior::Debuffer
                ) {
                    Vec2::new(28.0, 28.0)
                } else {
                    Vec2::splat((attack_range.0 * 1.05).clamp(36.0, 92.0))
                };
                let telegraph_pos = WorldPosition::new(
                    world_pos.x + dir.x * telegraph_distance,
                    world_pos.y + dir.y * telegraph_distance,
                );

                commands.spawn((
                    AttackTelegraph { owner: entity },
                    telegraph_pos,
                    Sprite {
                        color: Color::srgba(1.0, 0.0, 0.0, 0.25),
                        custom_size: Some(telegraph_size),
                        ..default()
                    },
                    Transform::default(),
                ));
            }
        } else {
            // Not in Windup -- remove any existing telegraph for this enemy.
            if let Some(&(_, telegraph_entity)) = has_telegraph {
                commands.entity(telegraph_entity).despawn();
            }
        }
    }
}

/// Manage the active attacker pool: max MAX_ACTIVE_ATTACKERS enemies can attack at once.
fn crowd_management_system(
    mut commands: Commands,
    enemy_query: Query<(Entity, &WorldPosition), (With<Enemy>, Without<Dying>)>,
    active_query: Query<Entity, With<ActiveAttacker>>,
    player_query: Query<&WorldPosition, With<Player>>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    let mut candidates: Vec<(Entity, f32)> = enemy_query
        .iter()
        .map(|(e, pos)| {
            let dist = pos.distance_to(player_pos);
            (e, dist)
        })
        .collect();

    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let selected: std::collections::HashSet<Entity> = candidates
        .iter()
        .take(MAX_ACTIVE_ATTACKERS)
        .map(|(entity, _)| *entity)
        .collect();

    for entity in active_query.iter() {
        if !selected.contains(&entity) {
            commands.entity(entity).remove::<ActiveAttacker>();
        }
    }

    for entity in selected {
        commands.entity(entity).insert(ActiveAttacker);
    }
}

/// Tick stagger timers and remove stagger component when done.
fn stagger_system(
    mut query: Query<(Entity, &mut Staggered)>,
    mut commands: Commands,
) {
    for (entity, mut stagger) in &mut query {
        stagger.frames_remaining = stagger.frames_remaining.saturating_sub(1);
        if stagger.frames_remaining == 0 {
            commands.entity(entity).remove::<Staggered>();
        }
    }
}

/// Check for dead enemies and begin death dissolution.
/// Triggers slow-motion when the last enemy dies.
fn enemy_death_system(
    query: Query<(Entity, &Health, &WorldPosition), (With<Enemy>, Without<Dying>)>,
    feel: Res<CombatFeelConfig>,
    mut death_msgs: MessageWriter<EnemyDeathMsg>,
    mut particle_msgs: MessageWriter<ParticleBurstMsg>,
    mut slow_mo_msgs: MessageWriter<KillSlowMoMsg>,
    mut run_state: Option<ResMut<crate::plugins::run::RunStateRes>>,
    mut commands: Commands,
) {
    let alive_count = query.iter().filter(|(_, h, _)| !h.is_dead()).count();
    let dying_this_frame: Vec<_> = query.iter().filter(|(_, h, _)| h.is_dead()).collect();

    for (entity, _health, pos) in &dying_this_frame {
        // Increment per-enemy kill counter in RunStateRes.
        if let Some(ref mut run) = run_state {
            run.enemies_killed += 1;
        }

        death_msgs.write(EnemyDeathMsg {
            entity: *entity,
            position: Vec2::new(pos.x, pos.y),
        });

        // Death particle burst -- bigger than hit particles.
        particle_msgs.write(ParticleBurstMsg {
            position: Vec2::new(pos.x, pos.y),
            direction: Vec2::ZERO,
            count: 20,
            color: Color::srgb(0.6, 0.1, 0.1),
        });

        // Remove combat components and add Dying marker.
        commands.entity(*entity)
            .remove::<ActiveAttacker>()
            .remove::<Hurtbox>()
            .remove::<AttackFired>()
            .insert(Dying {
                frames_remaining: feel.death_dissolve_frames,
            });

        // Also despawn any telegraphs owned by this enemy (handled in telegraph_system
        // next frame, but clean up immediately for responsiveness).
    }

    // If all remaining alive enemies are dying this frame, trigger slow-mo.
    if !dying_this_frame.is_empty() && alive_count == 0 {
        slow_mo_msgs.write(KillSlowMoMsg {
            frames: feel.last_enemy_slowmo_frames,
            time_scale: feel.last_enemy_slowmo_speed,
        });
    }
}

/// Fade out and despawn dying enemies.
fn dying_dissolution_system(
    mut query: Query<(Entity, &mut Dying, &mut Sprite)>,
    mut commands: Commands,
) {
    for (entity, mut dying, mut sprite) in &mut query {
        dying.frames_remaining = dying.frames_remaining.saturating_sub(1);

        // Fade alpha.
        let alpha = dying.frames_remaining as f32 / 20.0;
        sprite.color = sprite.color.with_alpha(alpha);

        if dying.frames_remaining == 0 {
            commands.entity(entity).despawn();
        }
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

/// Compute a facing Direction from a movement vector.
fn direction_from_vec2(v: Vec2) -> Direction {
    if v.length_squared() < 0.001 {
        return Direction::S;
    }
    let angle = v.y.atan2(v.x);
    Direction::from_angle(angle)
}

/// Update enemy TextureAtlas indices based on AI state and facing toward the player.
fn enemy_animation_system(
    mut enemy_query: Query<(
        &AiBehavior,
        &WorldPosition,
        &mut AnimationTimer,
        &mut Sprite,
    ), (With<Enemy>, Without<Dying>, Without<Player>)>,
    player_query: Query<&WorldPosition, With<Player>>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    for (ai, world_pos, mut anim, mut sprite) in &mut enemy_query {
        // Compute facing direction toward player.
        let dx = player_pos.x - world_pos.x;
        let dy = player_pos.y - world_pos.y;
        let facing_dir = direction_from_vec2(Vec2::new(dx, dy));
        let row = direction_to_flare_row(facing_dir);

        match ai.state {
            AiState::Chase => {
                // Walk animation: cycle columns 0-7.
                anim.frame_counter += 1;
                if anim.frame_counter >= anim.frames_per_step {
                    anim.frame_counter = 0;
                    anim.current_column = (anim.current_column + 1) % 8;
                }
            }
            AiState::Windup | AiState::Attack => {
                // Attack animation: cycle columns 0-3 faster.
                anim.frame_counter += 1;
                if anim.frame_counter >= (anim.frames_per_step / 2).max(1) {
                    anim.frame_counter = 0;
                    anim.current_column = (anim.current_column + 1) % 4;
                }
            }
            _ => {
                // Idle / Patrol / Recover / etc: static first frame.
                anim.frame_counter = 0;
                anim.current_column = 0;
            }
        }

        if let Some(ref mut atlas) = sprite.texture_atlas {
            atlas.index = (row * 8 + anim.current_column) as usize;
        }
    }
}
