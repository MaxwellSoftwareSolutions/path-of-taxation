use bevy::prelude::*;

use crate::app_state::AppState;
use crate::components::combat::Cooldowns;
use crate::components::enemy::{Enemy, EnemyType};
use crate::components::player::*;
use crate::plugins::run::RoomClearUI;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComplianceMeter>()
            .add_systems(OnEnter(AppState::Run), setup_hud)
            .add_systems(OnExit(AppState::Run), cleanup_hud)
            .add_systems(Update, (
                update_health_bar,
                update_mana_bar,
                update_cooldown_indicators,
                update_enemy_health_bars,
                update_room_title,
                update_deductions_text,
                update_compliance_bar,
                death_screen_system,
                death_screen_button_system,
            ).run_if(in_state(AppState::Run)));
    }
}

/// Compliance Meter: tracks how "compliant" the player is.
/// Using abilities: +1 per use. Taking damage: -2.
/// High (>75): enemies +15% HP. Low (<25): +20% drop rate.
#[derive(Resource, Clone, Debug)]
pub struct ComplianceMeter {
    pub value: f32,
}

impl Default for ComplianceMeter {
    fn default() -> Self {
        Self { value: 50.0 }
    }
}

impl ComplianceMeter {
    pub fn add(&mut self, amount: f32) {
        self.value = (self.value + amount).clamp(0.0, 100.0);
    }

    pub fn fraction(&self) -> f32 {
        self.value / 100.0
    }

    pub fn is_compliant(&self) -> bool {
        self.value > 75.0
    }

    pub fn is_non_compliant(&self) -> bool {
        self.value < 25.0
    }

    pub fn label(&self) -> &'static str {
        if self.is_compliant() {
            "COMPLIANT"
        } else if self.is_non_compliant() {
            "NON-COMPLIANT"
        } else {
            "FILING"
        }
    }
}

/// Marker for the HUD root node.
#[derive(Component)]
pub struct HudRoot;

/// Marker for the player health bar fill.
#[derive(Component)]
pub struct HealthBarFill;

/// Marker for the player mana bar fill.
#[derive(Component)]
pub struct ManaBarFill;

/// Marker for cooldown slot indicators.
#[derive(Component)]
pub struct CooldownSlot(pub usize);

/// Marker for room title text.
#[derive(Component)]
pub struct RoomTitleText;

/// Marker for the deductions counter text.
#[derive(Component)]
pub struct DeductionsText;

/// Marker for the minimap placeholder.
#[derive(Component)]
pub struct MinimapPlaceholder;

/// Marker for the death screen overlay.
#[derive(Component)]
pub struct DeathScreen;

/// Marker for the compliance bar fill.
#[derive(Component)]
pub struct ComplianceBarFill;

/// Marker for the compliance label text.
#[derive(Component)]
pub struct ComplianceLabelText;

/// Marker for death screen "FILE AGAIN" button.
#[derive(Component)]
pub struct DeathFileAgainButton;

/// Marker for death screen "RETURN TO OFFICE" button.
#[derive(Component)]
pub struct DeathReturnButton;

/// Marker for enemy health bar (world-space).
#[derive(Component)]
pub struct EnemyHealthBar {
    pub owner: Entity,
}

/// Set up the full HUD layout.
fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Top bar: health + mana.
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(60.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                })
                .with_children(|top| {
                    // Health bar: dark outline container -> background -> fill.
                    top.spawn((
                        BackgroundColor(Color::srgb(0.02, 0.02, 0.02)),
                        Node {
                            width: Val::Px(304.0),
                            height: Val::Px(24.0),
                            padding: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                    ))
                    .with_children(|outline| {
                        outline.spawn((
                            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                        ))
                        .with_children(|bar_bg| {
                            bar_bg.spawn((
                                HealthBarFill,
                                BackgroundColor(Color::srgb(0.8, 0.15, 0.15)),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                            ));
                        });
                    });

                    // Mana bar: dark outline container -> background -> fill.
                    top.spawn((
                        BackgroundColor(Color::srgb(0.02, 0.02, 0.02)),
                        Node {
                            width: Val::Px(304.0),
                            height: Val::Px(18.0),
                            padding: UiRect::all(Val::Px(2.0)),
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                    ))
                    .with_children(|outline| {
                        outline.spawn((
                            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                        ))
                        .with_children(|bar_bg| {
                            bar_bg.spawn((
                                ManaBarFill,
                                BackgroundColor(Color::srgb(0.15, 0.3, 0.8)),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                            ));
                        });
                    });
                });

            // Bottom section: compliance bar + ability cooldowns.
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|bottom_section| {
                    // Compliance meter: small horizontal bar above ability slots.
                    bottom_section
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(2.0),
                            ..default()
                        })
                        .with_children(|compliance_container| {
                            // Label: "COMPLIANT" / "NON-COMPLIANT" / "FILING".
                            compliance_container.spawn((
                                ComplianceLabelText,
                                Text::new("FILING"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.65, 0.5)),
                            ));
                            // Bar outline.
                            compliance_container
                                .spawn((
                                    BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                                    Node {
                                        width: Val::Px(200.0),
                                        height: Val::Px(10.0),
                                        padding: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                ))
                                .with_children(|outline| {
                                    outline
                                        .spawn((
                                            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                        ))
                                        .with_children(|bar_bg| {
                                            bar_bg.spawn((
                                                ComplianceBarFill,
                                                BackgroundColor(Color::srgb(0.3, 0.7, 0.3)),
                                                Node {
                                                    width: Val::Percent(50.0),
                                                    height: Val::Percent(100.0),
                                                    ..default()
                                                },
                                            ));
                                        });
                                });
                        });

                    // Ability cooldown slots.
                    bottom_section
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(60.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(8.0),
                            ..default()
                        })
                        .with_children(|bottom| {
                            for i in 0..6 {
                                bottom.spawn((
                                    CooldownSlot(i),
                                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                                    Node {
                                        width: Val::Px(44.0),
                                        height: Val::Px(44.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                ))
                                .with_children(|slot| {
                                    slot.spawn((
                                        Text::new(format!("{}", i + 1)),
                                        TextFont {
                                            font_size: 16.0,
                                            ..default()
                                        },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            }
                        });
                });
        });

    // Room title (top center).
    commands.spawn((
        RoomTitleText,
        Text::new("Room 1"),
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Percent(50.0),
            ..default()
        },
    ));

    // Deductions counter (below room title, top center).
    commands.spawn((
        DeductionsText,
        Text::new("Deductions: 0"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.75, 0.5, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Percent(50.0),
            ..default()
        },
    ));

    // Minimap placeholder (top-right corner).
    commands.spawn((
        MinimapPlaceholder,
        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            width: Val::Px(120.0),
            height: Val::Px(120.0),
            ..default()
        },
    ));
}

fn cleanup_hud(
    mut commands: Commands,
    hud_query: Query<Entity, With<HudRoot>>,
    title_query: Query<Entity, With<RoomTitleText>>,
    deductions_query: Query<Entity, With<DeductionsText>>,
    minimap_query: Query<Entity, With<MinimapPlaceholder>>,
    death_query: Query<Entity, With<DeathScreen>>,
    enemy_bar_query: Query<Entity, With<EnemyHealthBar>>,
) {
    for entity in hud_query.iter()
        .chain(title_query.iter())
        .chain(deductions_query.iter())
        .chain(minimap_query.iter())
        .chain(death_query.iter())
        .chain(enemy_bar_query.iter())
    {
        commands.entity(entity).despawn();
    }
    // Reset compliance meter on run exit.
    commands.insert_resource(ComplianceMeter::default());
}

/// Update the player health bar width.
fn update_health_bar(
    player_query: Query<&Health, With<Player>>,
    mut bar_query: Query<&mut Node, With<HealthBarFill>>,
) {
    let Ok(health) = player_query.single() else {
        return;
    };
    let Ok(mut node) = bar_query.single_mut() else {
        return;
    };
    node.width = Val::Percent(health.fraction() * 100.0);
}

/// Update the player mana bar width.
fn update_mana_bar(
    player_query: Query<&Mana, With<Player>>,
    mut bar_query: Query<&mut Node, With<ManaBarFill>>,
) {
    let Ok(mana) = player_query.single() else {
        return;
    };
    let Ok(mut node) = bar_query.single_mut() else {
        return;
    };
    node.width = Val::Percent(mana.fraction() * 100.0);
}

/// Update cooldown slot visuals (darken when on cooldown).
fn update_cooldown_indicators(
    player_query: Query<&Cooldowns, With<Player>>,
    mut slot_query: Query<(&CooldownSlot, &mut BackgroundColor)>,
) {
    let Ok(cooldowns) = player_query.single() else {
        return;
    };

    for (slot, mut bg) in &mut slot_query {
        let fraction = cooldowns.fraction(slot.0);
        let brightness = 0.15 + fraction * 0.35;
        bg.0 = Color::srgb(brightness, brightness, brightness);
    }
}

/// Update enemy health bars (positioned above enemies in screen space).
fn update_enemy_health_bars(
    enemy_query: Query<(Entity, &Health, &Transform), With<Enemy>>,
    mut bar_query: Query<(Entity, &EnemyHealthBar, &mut Transform, &mut Sprite), Without<Enemy>>,
    mut commands: Commands,
) {
    // For each enemy that doesn't have a health bar, spawn one.
    // For simplicity, we use world-space sprites rather than UI nodes.
    let existing_owners: Vec<Entity> = bar_query.iter().map(|(_, bar, _, _)| bar.owner).collect();

    for (enemy_entity, _health, enemy_transform) in &enemy_query {
        if !existing_owners.contains(&enemy_entity) {
            // Spawn a health bar above the enemy.
            commands.spawn((
                EnemyHealthBar { owner: enemy_entity },
                Sprite {
                    color: Color::srgb(0.8, 0.15, 0.15),
                    custom_size: Some(Vec2::new(22.0, 3.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(
                    enemy_transform.translation.x,
                    enemy_transform.translation.y + 35.0,
                    50.0,
                )),
            ));
        }
    }

    // Update existing bars.
    for (bar_entity, bar, mut bar_transform, mut bar_sprite) in &mut bar_query {
        if let Ok((_, health, enemy_transform)) = enemy_query.get(bar.owner) {
            bar_transform.translation.x = enemy_transform.translation.x;
            bar_transform.translation.y = enemy_transform.translation.y + 20.0;
            let width = 22.0 * health.fraction();
            bar_sprite.custom_size = Some(Vec2::new(width.max(0.0), 3.0));
        } else {
            // Enemy no longer exists; despawn bar entity (not the missing enemy).
            commands.entity(bar_entity).despawn();
        }
    }
}

/// Update room title text.
fn update_room_title(
    run_state: Option<Res<crate::plugins::run::RunStateRes>>,
    mut title_query: Query<&mut Text, With<RoomTitleText>>,
) {
    let Some(run) = run_state else {
        return;
    };
    let Ok(mut text) = title_query.single_mut() else {
        return;
    };
    **text = format!("Room {}", run.current_room + 1);
}

/// Update deductions counter text.
fn update_deductions_text(
    run_state: Option<Res<crate::plugins::run::RunStateRes>>,
    mut text_query: Query<&mut Text, With<DeductionsText>>,
) {
    let Some(run) = run_state else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };
    **text = format!("Deductions: {}", run.deductions_earned);
}

/// Update the compliance bar fill and label.
fn update_compliance_bar(
    compliance: Res<ComplianceMeter>,
    mut bar_query: Query<(&mut Node, &mut BackgroundColor), With<ComplianceBarFill>>,
    mut label_query: Query<(&mut Text, &mut TextColor), With<ComplianceLabelText>>,
) {
    if let Ok((mut node, mut bg)) = bar_query.single_mut() {
        node.width = Val::Percent(compliance.fraction() * 100.0);
        // Color shifts: green when compliant, red when non-compliant, yellow otherwise.
        bg.0 = if compliance.is_compliant() {
            Color::srgb(0.2, 0.8, 0.2)
        } else if compliance.is_non_compliant() {
            Color::srgb(0.8, 0.2, 0.2)
        } else {
            Color::srgb(0.7, 0.65, 0.2)
        };
    }

    if let Ok((mut text, mut color)) = label_query.single_mut() {
        **text = compliance.label().to_string();
        color.0 = if compliance.is_compliant() {
            Color::srgb(0.3, 0.9, 0.3)
        } else if compliance.is_non_compliant() {
            Color::srgb(0.9, 0.3, 0.3)
        } else {
            Color::srgb(0.7, 0.65, 0.5)
        };
    }
}

/// Show expanded death screen when player dies.
fn death_screen_system(
    player_query: Query<&Health, With<Player>>,
    death_query: Query<Entity, With<DeathScreen>>,
    room_clear_query: Query<Entity, With<RoomClearUI>>,
    run_state: Option<Res<crate::plugins::run::RunStateRes>>,
    enemy_query: Query<&EnemyType, With<Enemy>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let Ok(health) = player_query.single() else {
        return;
    };

    if health.is_dead() && death_query.is_empty() {
        // Despawn any lingering room clear UI so it doesn't overlap death screen.
        for entity in &room_clear_query {
            commands.entity(entity).despawn();
        }
        // Gather run stats for the death screen.
        let rooms_completed = run_state.as_ref().map(|r| r.rooms_cleared).unwrap_or(0);
        let deductions = run_state.as_ref().map(|r| r.deductions_earned).unwrap_or(0);
        let time_elapsed = time.elapsed_secs() as u32;

        // Try to get the last enemy type name as the "killer".
        let killer = enemy_query
            .iter()
            .next()
            .map(|et| et.0.replace('_', " "))
            .unwrap_or_else(|| "Unknown Entity".to_string());

        commands
            .spawn((
                DeathScreen,
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                ZIndex(50),
            ))
            .with_children(|parent| {
                // Title with shadow.
                parent
                    .spawn(Node {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    })
                    .with_children(|text_container| {
                        // Shadow text.
                        text_container.spawn((
                            Text::new("YOUR TAX RETURN HAS BEEN REJECTED"),
                            TextFont {
                                font_size: 48.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(2.0),
                                top: Val::Px(2.0),
                                ..default()
                            },
                        ));
                        // Foreground text.
                        text_container.spawn((
                            Text::new("YOUR TAX RETURN HAS BEEN REJECTED"),
                            TextFont {
                                font_size: 48.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.2, 0.2)),
                        ));
                    });

                // Cause of death.
                parent.spawn((
                    Text::new(format!("Terminated by: {}", killer)),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.7, 0.6)),
                ));

                // Time filed.
                let mins = time_elapsed / 60;
                let secs = time_elapsed % 60;
                parent.spawn((
                    Text::new(format!("Time filed: {}:{:02}", mins, secs)),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.65, 0.55)),
                ));

                // Rooms completed.
                parent.spawn((
                    Text::new(format!("Rooms completed: {}", rooms_completed)),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.65, 0.55)),
                ));

                // Deductions claimed.
                parent.spawn((
                    Text::new(format!("Deductions claimed: {}", deductions)),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.65, 0.55)),
                ));

                // Buttons row.
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(24.0),
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    })
                    .with_children(|row| {
                        // "FILE AGAIN" button (restart run).
                        row.spawn((
                            DeathFileAgainButton,
                            Button,
                            BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                            Node {
                                padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("FILE AGAIN"),
                                TextFont {
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.95, 0.85)),
                            ));
                        });

                        // "RETURN TO OFFICE" button (go to Hub).
                        row.spawn((
                            DeathReturnButton,
                            Button,
                            BackgroundColor(Color::srgb(0.5, 0.15, 0.15)),
                            Node {
                                padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("RETURN TO OFFICE"),
                                TextFont {
                                    font_size: 22.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.95, 0.85)),
                            ));
                        });
                    });
            });
    }
}

/// Handle clicks on death screen buttons.
fn death_screen_button_system(
    mut next_state: ResMut<NextState<AppState>>,
    file_again_query: Query<
        &Interaction,
        (Changed<Interaction>, With<DeathFileAgainButton>),
    >,
    return_query: Query<
        &Interaction,
        (Changed<Interaction>, With<DeathReturnButton>),
    >,
) {
    for interaction in &file_again_query {
        if *interaction == Interaction::Pressed {
            // Restart: transition back to Run (will re-init everything).
            next_state.set(AppState::Run);
        }
    }

    for interaction in &return_query {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Hub);
        }
    }
}
