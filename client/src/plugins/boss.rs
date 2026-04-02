use bevy::prelude::*;

use crate::app_state::{AppState, CombatPhase};
use crate::components::boss::{Boss, BossState};
use crate::components::combat::{Damage, Hitbox, Hurtbox, Projectile};
use crate::components::player::{Health, Player};
use crate::plugins::combat::HitboxLifetime;
use crate::plugins::run::{ArenaEntity, RunStateRes};
use crate::plugins::vfx::{KillSlowMoMsg, ParticleBurstMsg, ScreenFlashMsg};
use crate::rendering::isometric::WorldPosition;
use crate::rendering::sprites::{CharacterAtlasLayout, SpriteAssets};

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(CombatPhase::BossIntro), spawn_boss)
            .add_systems(
                Update,
                boss_intro_system.run_if(in_state(CombatPhase::BossIntro)),
            )
            .add_systems(
                Update,
                (
                    boss_ai_system,
                    boss_stamp_slam_system,
                    boss_phase_transition_system,
                    boss_death_system,
                    boss_hp_bar_system,
                    boss_bark_system,
                )
                    .chain()
                    .run_if(in_state(CombatPhase::BossFight)),
            )
            .add_systems(OnExit(AppState::Run), cleanup_boss_ui);
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Countdown timer for the boss intro sequence.
#[derive(Resource)]
pub struct BossIntroTimer {
    pub frames_remaining: u32,
}

/// Tracks a pending Stamp Slam telegraph before the AoE fires.
#[derive(Resource)]
pub struct StampSlamTelegraph {
    /// World position where the slam will land.
    pub target: Vec2,
    /// Frames remaining before the slam fires.
    pub frames_remaining: u32,
}

/// Tracks the Audit Charge dash in progress.
#[derive(Resource)]
pub struct AuditCharge {
    pub start: Vec2,
    pub end: Vec2,
    pub frames_remaining: u32,
    pub total_frames: u32,
}

/// Bark text displayed during phase transitions.
#[derive(Resource)]
pub struct BossBark {
    pub frames_remaining: u32,
}

// ---------------------------------------------------------------------------
// UI markers
// ---------------------------------------------------------------------------

/// Marker for the boss intro title text entity.
#[derive(Component)]
pub struct BossIntroText;

/// Marker for the boss HP bar container (UI node).
#[derive(Component)]
pub struct BossHpBarRoot;

/// Marker for the boss HP bar fill.
#[derive(Component)]
pub struct BossHpBarFill;

/// Marker for boss bark text.
#[derive(Component)]
pub struct BossBarkText;

/// Marker for the boss death "AUDIT COMPLETE" text.
#[derive(Component)]
pub struct BossDeathText;

/// Timer resource for the boss death sequence.
#[derive(Resource)]
pub struct BossDeathTimer {
    pub frames_remaining: u32,
}

// ---------------------------------------------------------------------------
// Boss Intro
// ---------------------------------------------------------------------------

fn spawn_boss(
    mut commands: Commands,
    sprite_assets: Res<SpriteAssets>,
    atlas_layout: Res<CharacterAtlasLayout>,
) {
    // Spawn the boss entity.
    commands.spawn((
        Boss,
        BossState::default(),
        ArenaEntity,
        Health {
            current: 500.0,
            max: 500.0,
        },
        Hurtbox {
            radius: 30.0,
            faction: pot_shared::types::Faction::Enemy,
        },
        WorldPosition::new(0.0, 200.0),
        Sprite {
            image: sprite_assets.enemy_tax_collector.clone(), // enemy_skeleton sprite
            color: Color::srgb(0.6, 0.1, 0.1),               // dark red tint
            custom_size: Some(Vec2::new(384.0, 384.0)),
            texture_atlas: Some(TextureAtlas {
                layout: atlas_layout.layout.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::default(),
    ));

    // Intro title text (screen-space UI).
    commands.spawn((
        BossIntroText,
        Text::new("THE BLOATED FILER"),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.2, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(30.0),
            left: Val::Percent(50.0),
            ..default()
        },
    ));

    // Insert intro timer resource.
    commands.insert_resource(BossIntroTimer {
        frames_remaining: 120,
    });
}

fn boss_intro_system(
    mut timer: ResMut<BossIntroTimer>,
    mut next_phase: ResMut<NextState<CombatPhase>>,
    intro_text_query: Query<Entity, With<BossIntroText>>,
    mut commands: Commands,
) {
    timer.frames_remaining = timer.frames_remaining.saturating_sub(1);

    if timer.frames_remaining == 0 {
        // Clean up intro text.
        for entity in &intro_text_query {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<BossIntroTimer>();

        // Spawn boss HP bar UI.
        spawn_boss_hp_bar(&mut commands);

        next_phase.set(CombatPhase::BossFight);
    }
}

fn spawn_boss_hp_bar(commands: &mut Commands) {
    commands
        .spawn((
            BossHpBarRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(40.0),
                left: Val::Percent(30.0),
                width: Val::Percent(40.0),
                height: Val::Px(28.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Boss name label.
            parent.spawn((
                Text::new("THE BLOATED FILER"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.2, 0.2)),
            ));

            // Bar background.
            parent
                .spawn((
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(12.0),
                        margin: UiRect::top(Val::Px(2.0)),
                        ..default()
                    },
                ))
                .with_children(|bar_bg| {
                    // Fill.
                    bar_bg.spawn((
                        BossHpBarFill,
                        BackgroundColor(Color::srgb(0.8, 0.15, 0.15)),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));
                });
        });
}

// ---------------------------------------------------------------------------
// Boss AI
// ---------------------------------------------------------------------------

fn boss_ai_system(
    mut boss_query: Query<(&mut BossState, &mut WorldPosition, &Health), With<Boss>>,
    player_query: Query<&WorldPosition, (With<Player>, Without<Boss>)>,
    mut commands: Commands,
    slam_res: Option<Res<StampSlamTelegraph>>,
    charge_res: Option<ResMut<AuditCharge>>,
    time: Res<Time>,
) {
    let Ok((mut boss, mut boss_pos, _health)) = boss_query.single_mut() else {
        return;
    };
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    // Handle active Audit Charge dash.
    if let Some(mut charge) = charge_res {
        charge.frames_remaining = charge.frames_remaining.saturating_sub(1);
        let t = 1.0
            - (charge.frames_remaining as f32 / charge.total_frames as f32);
        boss_pos.x = charge.start.x + (charge.end.x - charge.start.x) * t;
        boss_pos.y = charge.start.y + (charge.end.y - charge.start.y) * t;
        if charge.frames_remaining == 0 {
            commands.remove_resource::<AuditCharge>();
        }
        return;
    }

    // Tick attack timer.
    boss.attack_timer += 1;

    let interval = boss.attack_interval();

    if boss.attack_timer >= interval {
        boss.attack_timer = 0;

        match boss.attack_index {
            0 => {
                // Stamp Slam: place telegraph at player position.
                if slam_res.is_none() {
                    commands.insert_resource(StampSlamTelegraph {
                        target: Vec2::new(player_pos.x, player_pos.y),
                        frames_remaining: 40,
                    });
                }
            }
            1 => {
                // Paper Barrage: spawn 8 projectiles in a circle.
                let bx = boss_pos.x;
                let by = boss_pos.y;
                for i in 0..8 {
                    let angle =
                        (i as f32 / 8.0) * std::f32::consts::TAU;
                    let dir = Vec2::new(angle.cos(), angle.sin());
                    let proj_pos = WorldPosition::new(
                        bx + dir.x * 20.0,
                        by + dir.y * 20.0,
                    );

                    commands.spawn((
                        Projectile {
                            speed: 150.0,
                            direction: dir,
                            lifetime_frames: 90,
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
                            amount: 15.0,
                            damage_type: pot_shared::types::DamageType::Penalty,
                            knockback_force: 80.0,
                            knockback_dir: dir,
                            is_critical: false,
                        },
                        proj_pos,
                        Sprite {
                            color: Color::srgba(0.8, 0.2, 0.1, 0.8),
                            custom_size: Some(Vec2::new(20.0, 20.0)),
                            ..default()
                        },
                        Transform::default(),
                        HitboxLifetime {
                            frames_remaining: 90,
                        },
                    ));
                }
            }
            2 => {
                // Audit Charge: dash toward player over 20 frames.
                commands.insert_resource(AuditCharge {
                    start: Vec2::new(boss_pos.x, boss_pos.y),
                    end: Vec2::new(player_pos.x, player_pos.y),
                    frames_remaining: 20,
                    total_frames: 20,
                });
            }
            _ => {}
        }

        boss.attack_index = (boss.attack_index + 1) % 3;
    } else {
        // Between attacks, slowly chase the player.
        let dx = player_pos.x - boss_pos.x;
        let dy = player_pos.y - boss_pos.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > 30.0 {
            let chase_speed = 40.0;
            boss_pos.x += (dx / dist) * chase_speed * time.delta_secs();
            boss_pos.y += (dy / dist) * chase_speed * time.delta_secs();
        }
    }
}

/// Tick the Stamp Slam telegraph and spawn the AoE hitbox when it expires.
fn boss_stamp_slam_system(
    mut commands: Commands,
    slam_res: Option<ResMut<StampSlamTelegraph>>,
) {
    let Some(mut slam) = slam_res else {
        return;
    };

    slam.frames_remaining = slam.frames_remaining.saturating_sub(1);

    if slam.frames_remaining == 0 {
        let pos = slam.target;

        // Spawn large AoE hitbox.
        commands.spawn((
            Hitbox {
                radius: 60.0,
                faction: pot_shared::types::Faction::Enemy,
                already_hit: Vec::new(),
            },
            Damage {
                amount: 25.0,
                damage_type: pot_shared::types::DamageType::Penalty,
                knockback_force: 150.0,
                knockback_dir: Vec2::new(0.0, -1.0),
                is_critical: false,
            },
            WorldPosition::new(pos.x, pos.y),
            Sprite {
                color: Color::srgba(1.0, 0.0, 0.0, 0.5),
                custom_size: Some(Vec2::splat(120.0)),
                ..default()
            },
            Transform::default(),
            HitboxLifetime {
                frames_remaining: 6,
            },
        ));

        commands.remove_resource::<StampSlamTelegraph>();
    }
}

// ---------------------------------------------------------------------------
// Phase Transitions
// ---------------------------------------------------------------------------

fn boss_phase_transition_system(
    mut boss_query: Query<(&mut BossState, &Health), With<Boss>>,
    mut slow_mo_msgs: MessageWriter<KillSlowMoMsg>,
    mut screen_flash_msgs: MessageWriter<ScreenFlashMsg>,
    mut commands: Commands,
) {
    let Ok((mut boss, health)) = boss_query.single_mut() else {
        return;
    };

    let fraction = health.fraction();

    // Phase 1 at 66% HP.
    if fraction <= 0.66 && !boss.phase_transitioned[1] {
        boss.phase_transitioned[1] = true;
        boss.current_phase = 1;
        boss.attack_timer = 0;

        slow_mo_msgs.write(KillSlowMoMsg {
            frames: 30,
            time_scale: 0.3,
        });
        screen_flash_msgs.write(ScreenFlashMsg {
            color: Color::WHITE,
        });

        // Spawn bark text.
        commands.insert_resource(BossBark {
            frames_remaining: 90,
        });
        commands.spawn((
            BossBarkText,
            Text::new("YOUR FILING IS INCOMPLETE!"),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.8, 0.2)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(45.0),
                left: Val::Percent(50.0),
                ..default()
            },
        ));
    }

    // Phase 2 at 33% HP.
    if fraction <= 0.33 && !boss.phase_transitioned[2] {
        boss.phase_transitioned[2] = true;
        boss.current_phase = 2;
        boss.attack_timer = 0;

        slow_mo_msgs.write(KillSlowMoMsg {
            frames: 30,
            time_scale: 0.3,
        });
        screen_flash_msgs.write(ScreenFlashMsg {
            color: Color::WHITE,
        });

        commands.insert_resource(BossBark {
            frames_remaining: 90,
        });
        commands.spawn((
            BossBarkText,
            Text::new("PREPARE FOR FULL AUDIT!"),
            TextFont {
                font_size: 36.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.3, 0.1)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(45.0),
                left: Val::Percent(50.0),
                ..default()
            },
        ));
    }
}

// ---------------------------------------------------------------------------
// Boss Death
// ---------------------------------------------------------------------------

fn boss_death_system(
    boss_query: Query<(Entity, &Health, &WorldPosition), With<Boss>>,
    death_timer: Option<ResMut<BossDeathTimer>>,
    mut slow_mo_msgs: MessageWriter<KillSlowMoMsg>,
    mut particle_msgs: MessageWriter<ParticleBurstMsg>,
    mut run_state: Option<ResMut<RunStateRes>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    // If death timer is ticking, count down and finish the run.
    if let Some(mut timer) = death_timer {
        timer.frames_remaining = timer.frames_remaining.saturating_sub(1);
        if timer.frames_remaining == 0 {
            commands.remove_resource::<BossDeathTimer>();
            if let Some(ref mut run) = run_state {
                run.run_complete = true;
            }
            next_app_state.set(AppState::Results);
        }
        return;
    }

    let Ok((entity, health, pos)) = boss_query.single() else {
        return;
    };

    if !health.is_dead() {
        return;
    }

    // Extended slow-mo.
    slow_mo_msgs.write(KillSlowMoMsg {
        frames: 60,
        time_scale: 0.2,
    });

    // Big particle burst.
    particle_msgs.write(ParticleBurstMsg {
        position: Vec2::new(pos.x, pos.y),
        direction: Vec2::ZERO,
        count: 40,
        color: Color::srgb(0.8, 0.2, 0.1),
    });

    // Despawn boss entity.
    commands.entity(entity).despawn();

    // "AUDIT COMPLETE" text.
    commands.spawn((
        BossDeathText,
        Text::new("AUDIT COMPLETE"),
        TextFont {
            font_size: 52.0,
            ..default()
        },
        TextColor(Color::srgb(0.2, 0.9, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(40.0),
            left: Val::Percent(50.0),
            ..default()
        },
    ));

    // Start death timer.
    commands.insert_resource(BossDeathTimer {
        frames_remaining: 120,
    });
}

// ---------------------------------------------------------------------------
// Boss HP Bar
// ---------------------------------------------------------------------------

fn boss_hp_bar_system(
    boss_query: Query<&Health, With<Boss>>,
    mut bar_query: Query<&mut Node, With<BossHpBarFill>>,
) {
    let Ok(health) = boss_query.single() else {
        return;
    };
    let Ok(mut node) = bar_query.single_mut() else {
        return;
    };
    node.width = Val::Percent(health.fraction() * 100.0);
}

// ---------------------------------------------------------------------------
// Bark Text Timeout
// ---------------------------------------------------------------------------

fn boss_bark_system(
    bark_res: Option<ResMut<BossBark>>,
    bark_text_query: Query<Entity, With<BossBarkText>>,
    mut commands: Commands,
) {
    let Some(mut bark) = bark_res else {
        return;
    };
    bark.frames_remaining = bark.frames_remaining.saturating_sub(1);
    if bark.frames_remaining == 0 {
        for entity in &bark_text_query {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<BossBark>();
    }
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

fn cleanup_boss_ui(
    mut commands: Commands,
    hp_bar_query: Query<Entity, With<BossHpBarRoot>>,
    bark_query: Query<Entity, With<BossBarkText>>,
    intro_query: Query<Entity, With<BossIntroText>>,
    death_query: Query<Entity, With<BossDeathText>>,
) {
    for entity in hp_bar_query
        .iter()
        .chain(bark_query.iter())
        .chain(intro_query.iter())
        .chain(death_query.iter())
    {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<BossIntroTimer>();
    commands.remove_resource::<StampSlamTelegraph>();
    commands.remove_resource::<AuditCharge>();
    commands.remove_resource::<BossBark>();
    commands.remove_resource::<BossDeathTimer>();
}
