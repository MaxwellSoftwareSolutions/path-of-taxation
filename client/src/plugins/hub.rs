use bevy::prelude::*;

use crate::app_state::AppState;
use crate::rendering::isometric::{world_to_screen, WorldPosition};
use crate::rendering::sprites::{CharacterAtlasLayout, SpriteAssets};

pub struct HubPlugin;

/// Marker for the hub player entity (separate from combat Player).
#[derive(Component)]
pub struct HubPlayer;

impl Plugin for HubPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Hub), setup_hub)
            .add_systems(OnExit(AppState::Hub), cleanup_hub)
            .add_systems(
                Update,
                (
                    hub_player_movement_system,
                    npc_interact_prompt_system,
                    npc_interact_system,
                    dialogue_continue_system,
                    filing_cabinet_ui_system,
                    filing_cabinet_buy_system,
                )
                    .chain()
                    .run_if(in_state(AppState::Hub)),
            )
            .add_systems(OnEnter(AppState::Results), setup_results)
            .add_systems(OnExit(AppState::Results), cleanup_results);
    }
}

/// Move the hub player with WASD.
fn hub_player_movement_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut WorldPosition, With<HubPlayer>>,
    time: Res<Time>,
) {
    let Ok(mut pos) = query.single_mut() else { return };
    let speed = 200.0;
    let mut dir = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { dir.y += 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { dir.y -= 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { dir.x += 1.0; }
    if dir != Vec2::ZERO {
        dir = dir.normalize();
        pos.x += dir.x * speed * time.delta_secs();
        pos.y += dir.y * speed * time.delta_secs();
    }
}

// ---------------------------------------------------------------------------
// Components and enums
// ---------------------------------------------------------------------------

/// Marker for hub scene entities.
#[derive(Component)]
pub struct HubEntity;

/// Marker for the results screen UI.
#[derive(Component)]
pub struct ResultsUI;

/// NPC type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Renly,
    Una,
    FilingCabinet,
}

/// Marker for NPC entities.
#[derive(Component)]
pub struct NpcEntity {
    pub npc_type: NpcType,
    pub interaction_radius: f32,
}

/// Interaction prompt shown above an NPC when the player is nearby.
#[derive(Component)]
pub struct InteractPrompt {
    pub npc_entity: Entity,
}

/// Active dialogue box state.
#[derive(Component)]
pub struct DialogueBox;

/// Tracks dialogue progression for the currently open dialogue.
#[derive(Resource)]
pub struct ActiveDialogue {
    pub npc_type: NpcType,
    pub lines: Vec<&'static str>,
    pub current_line: usize,
}

/// Filing cabinet upgrade menu root.
#[derive(Component)]
pub struct FilingCabinetUI;

/// A single upgrade button in the filing cabinet menu.
#[derive(Component)]
pub struct UpgradeButton(pub usize);

/// Meta-progression resource -- persists across runs.
#[derive(Resource)]
pub struct MetaProgression {
    pub compliance_credits: u32,
    pub filing_cabinet_ranks: [u32; 5],
}

impl Default for MetaProgression {
    fn default() -> Self {
        Self {
            compliance_credits: 0,
            filing_cabinet_ranks: [0; 5],
        }
    }
}

/// Upgrade definitions for the filing cabinet.
pub const UPGRADE_NAMES: [&str; 5] = [
    "HP +5%/rank",
    "Speed +3%/rank",
    "Mana +10%/rank",
    "Damage +5%/rank",
    "Crit +2%/rank",
];

/// Cost to purchase the next rank: (current_rank + 1) * 50.
fn upgrade_cost(current_rank: u32) -> u32 {
    (current_rank + 1) * 50
}

/// Summary of a completed run, captured before RunStateRes is cleaned up.
#[derive(Resource, Clone, Debug)]
pub struct RunSummary {
    pub run_complete: bool,
    pub rooms_cleared: u32,
    pub enemies_killed: u32,
    pub deductions_earned: u32,
}

impl Default for RunSummary {
    fn default() -> Self {
        Self {
            run_complete: false,
            rooms_cleared: 0,
            enemies_killed: 0,
            deductions_earned: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Dialogue content
// ---------------------------------------------------------------------------

fn dialogue_lines(npc_type: NpcType) -> Vec<&'static str> {
    match npc_type {
        NpcType::Renly => vec![
            "Welcome to Clearfile Tax Office. Your death and taxes await.",
            "I can sell you equipment... for the right filing fee.",
            "Remember: every deduction you earn is a deduction the IRS didn't get.",
            "Good luck out there. The auditors show no mercy.",
        ],
        NpcType::Una => vec![
            "I audit your combat performance. It's... concerning.",
            "Have you filed your Form 1040 Barrage correctly?",
            "Your damage output is a write-off. Literally.",
            "Come back when your compliance rating improves.",
        ],
        NpcType::FilingCabinet => vec![],
    }
}

// ---------------------------------------------------------------------------
// Hub setup
// ---------------------------------------------------------------------------

fn setup_hub(
    mut commands: Commands,
    sprite_assets: Res<SpriteAssets>,
    atlas_layout: Res<CharacterAtlasLayout>,
) {
    // Title text and instruction.
    commands.spawn((
        HubEntity,
        Text2d::new("CLEARFILE TAX OFFICE"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.85, 0.6)),
        Transform::from_translation(Vec3::new(0.0, 120.0, 10.0)),
    ));

    commands.spawn((
        HubEntity,
        Text2d::new("[Press ENTER to start a run]"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.6, 0.6)),
        Transform::from_translation(Vec3::new(0.0, 85.0, 10.0)),
    ));

    // Hub floor: isometric wood tile grid to look like an office.
    let tile_w: f32 = 64.0;
    let tile_h: f32 = 32.0;
    let hub_radius: i32 = 5;
    for row in -hub_radius..=hub_radius {
        for col in -hub_radius..=hub_radius {
            if (row.abs() + col.abs()) > hub_radius {
                continue;
            }
            let world_x = col as f32 * tile_w * 0.5;
            let world_y = row as f32 * tile_w * 0.5;
            let screen = world_to_screen(world_x, world_y);

            let dark = (row + col).rem_euclid(2) == 0;
            let tint = if dark {
                Color::srgb(0.9, 0.9, 0.9)
            } else {
                Color::WHITE
            };

            commands.spawn((
                HubEntity,
                Sprite {
                    image: sprite_assets.floor_wood.clone(),
                    color: tint,
                    custom_size: Some(Vec2::new(tile_w, tile_h)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(screen.x, screen.y, -10.0)),
            ));
        }
    }

    // Place some wall/pillar props around the edges for an office feel.
    let pillar_positions = [
        (-3.0_f32, -3.0_f32),
        (3.0, -3.0),
        (-3.0, 3.0),
        (3.0, 3.0),
    ];
    for (wx, wy) in pillar_positions {
        let world_x = wx * tile_w * 0.5;
        let world_y = wy * tile_w * 0.5;
        let screen = world_to_screen(world_x, world_y);
        commands.spawn((
            HubEntity,
            Sprite {
                image: sprite_assets.wall.clone(),
                custom_size: Some(Vec2::new(48.0, 48.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(screen.x, screen.y, -5.0)),
        ));
    }

    // --- Hub Player (movable) ---
    commands.spawn((
        HubEntity,
        HubPlayer,
        WorldPosition::new(0.0, 0.0),
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
    ));

    // --- NPC: Renly the Accountant ---
    commands.spawn((
        HubEntity,
        NpcEntity {
            npc_type: NpcType::Renly,
            interaction_radius: 40.0,
        },
        WorldPosition::new(-100.0, -50.0),
        Sprite {
            image: sprite_assets.player.clone(),
            color: Color::srgb(0.4, 0.5, 1.0), // blue tint
            custom_size: Some(Vec2::new(256.0, 256.0)),
            texture_atlas: Some(TextureAtlas {
                layout: atlas_layout.layout.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::default(),
    ));

    // Renly name label.
    let renly_screen = world_to_screen(-100.0, -50.0);
    commands.spawn((
        HubEntity,
        Text2d::new("Renly the Accountant"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.75, 1.0)),
        Transform::from_translation(Vec3::new(renly_screen.x, renly_screen.y + 50.0, 91.0)),
    ));

    // --- NPC: Una the Auditor ---
    commands.spawn((
        HubEntity,
        NpcEntity {
            npc_type: NpcType::Una,
            interaction_radius: 40.0,
        },
        WorldPosition::new(100.0, -50.0),
        Sprite {
            image: sprite_assets.enemy_undead_accountant.clone(),
            color: Color::srgb(0.4, 1.0, 0.5), // green tint
            custom_size: Some(Vec2::new(256.0, 256.0)),
            texture_atlas: Some(TextureAtlas {
                layout: atlas_layout.layout.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::default(),
    ));

    // Una name label.
    let una_screen = world_to_screen(100.0, -50.0);
    commands.spawn((
        HubEntity,
        Text2d::new("Una the Auditor"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 1.0, 0.6)),
        Transform::from_translation(Vec3::new(una_screen.x, una_screen.y + 50.0, 91.0)),
    ));

    // --- NPC: Filing Cabinet ---
    commands.spawn((
        HubEntity,
        NpcEntity {
            npc_type: NpcType::FilingCabinet,
            interaction_radius: 40.0,
        },
        WorldPosition::new(0.0, -120.0),
        Sprite {
            image: sprite_assets.wall.clone(),
            color: Color::srgb(0.7, 0.6, 0.4),
            custom_size: Some(Vec2::new(48.0, 48.0)),
            ..default()
        },
        Transform::default(),
    ));

    // Filing Cabinet label.
    let cabinet_screen = world_to_screen(0.0, -120.0);
    commands.spawn((
        HubEntity,
        Text2d::new("Filing Cabinet"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.7, 0.5)),
        Transform::from_translation(Vec3::new(
            cabinet_screen.x,
            cabinet_screen.y + 35.0,
            91.0,
        )),
    ));
}

fn cleanup_hub(
    mut commands: Commands,
    query: Query<Entity, With<HubEntity>>,
    prompt_query: Query<Entity, With<InteractPrompt>>,
    dialogue_query: Query<Entity, With<DialogueBox>>,
    cabinet_query: Query<Entity, With<FilingCabinetUI>>,
) {
    for entity in query
        .iter()
        .chain(prompt_query.iter())
        .chain(dialogue_query.iter())
        .chain(cabinet_query.iter())
    {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<ActiveDialogue>();
}

// ---------------------------------------------------------------------------
// NPC interaction prompt system
// ---------------------------------------------------------------------------

/// Show/hide "[E] Talk" prompts based on the hub player's position.
fn npc_interact_prompt_system(
    mut commands: Commands,
    npc_query: Query<(Entity, &NpcEntity, &WorldPosition), Without<HubPlayer>>,
    prompt_query: Query<(Entity, &InteractPrompt)>,
    hub_player_query: Query<&WorldPosition, With<HubPlayer>>,
    dialogue_res: Option<Res<ActiveDialogue>>,
    cabinet_query: Query<Entity, With<FilingCabinetUI>>,
) {
    // Don't show prompts while dialogue or cabinet UI is open.
    if dialogue_res.is_some() || !cabinet_query.is_empty() {
        for (prompt_entity, _) in &prompt_query {
            commands.entity(prompt_entity).despawn();
        }
        return;
    }

    let player_pos = hub_player_query.single().cloned().unwrap_or(WorldPosition::new(0.0, 0.0));

    for (npc_entity, npc, npc_pos) in &npc_query {
        let dist = player_pos.distance_to(npc_pos);
        let has_prompt = prompt_query.iter().any(|(_, p)| p.npc_entity == npc_entity);

        if dist <= npc.interaction_radius && !has_prompt {
            // Spawn prompt above NPC.
            let screen = world_to_screen(npc_pos.x, npc_pos.y);
            let label = match npc.npc_type {
                NpcType::FilingCabinet => "[E] Open",
                _ => "[E] Talk",
            };
            commands.spawn((
                HubEntity,
                InteractPrompt {
                    npc_entity,
                },
                Text2d::new(label),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.7)),
                Transform::from_translation(Vec3::new(screen.x, screen.y + 70.0, 92.0)),
            ));
        } else if dist > npc.interaction_radius && has_prompt {
            // Remove prompt.
            for (prompt_entity, p) in &prompt_query {
                if p.npc_entity == npc_entity {
                    commands.entity(prompt_entity).despawn();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NPC interaction (E key) -- open dialogue or filing cabinet
// ---------------------------------------------------------------------------

fn npc_interact_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    npc_query: Query<(&NpcEntity, &WorldPosition)>,
    dialogue_res: Option<Res<ActiveDialogue>>,
    cabinet_query: Query<Entity, With<FilingCabinetUI>>,
    meta: Res<MetaProgression>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    // Don't open a new interaction while one is already active.
    if dialogue_res.is_some() || !cabinet_query.is_empty() {
        return;
    }

    let player_pos = WorldPosition::new(0.0, 0.0);

    for (npc, npc_pos) in &npc_query {
        if player_pos.distance_to(npc_pos) > npc.interaction_radius {
            continue;
        }

        match npc.npc_type {
            NpcType::Renly | NpcType::Una => {
                let lines = dialogue_lines(npc.npc_type);
                if lines.is_empty() {
                    continue;
                }
                commands.insert_resource(ActiveDialogue {
                    npc_type: npc.npc_type,
                    lines: lines.clone(),
                    current_line: 0,
                });
                spawn_dialogue_box(&mut commands, lines[0]);
            }
            NpcType::FilingCabinet => {
                spawn_filing_cabinet_ui(&mut commands, &meta);
            }
        }
        break;
    }
}

// ---------------------------------------------------------------------------
// Dialogue box
// ---------------------------------------------------------------------------

fn spawn_dialogue_box(commands: &mut Commands, text: &str) {
    commands
        .spawn((
            DialogueBox,
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.92)),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(40.0),
                left: Val::Percent(10.0),
                width: Val::Percent(80.0),
                height: Val::Px(120.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.88, 0.8)),
            ));
            parent.spawn((
                Text::new("[E] Continue"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.45)),
                Node {
                    align_self: AlignSelf::FlexEnd,
                    ..default()
                },
            ));
        });
}

/// Advance or close dialogue on E press.
fn dialogue_continue_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut dialogue_res: Option<ResMut<ActiveDialogue>>,
    dialogue_query: Query<Entity, With<DialogueBox>>,
) {
    let Some(ref mut dialogue) = dialogue_res else {
        return;
    };

    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }

    // Despawn old dialogue box.
    for entity in &dialogue_query {
        commands.entity(entity).despawn();
    }

    dialogue.current_line += 1;
    if dialogue.current_line >= dialogue.lines.len() {
        // End of dialogue.
        commands.remove_resource::<ActiveDialogue>();
    } else {
        // Show next line.
        let line = dialogue.lines[dialogue.current_line];
        spawn_dialogue_box(&mut commands, line);
    }
}

// ---------------------------------------------------------------------------
// Filing Cabinet UI
// ---------------------------------------------------------------------------

fn spawn_filing_cabinet_ui(commands: &mut Commands, meta: &MetaProgression) {
    commands
        .spawn((
            FilingCabinetUI,
            BackgroundColor(Color::srgba(0.04, 0.04, 0.06, 0.95)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(15.0),
                left: Val::Percent(25.0),
                width: Val::Percent(50.0),
                height: Val::Percent(70.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(12.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title.
            parent.spawn((
                Text::new("FILING CABINET -- Permanent Upgrades"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.6)),
            ));

            // Credits display.
            parent.spawn((
                Text::new(format!(
                    "Compliance Credits: {}",
                    meta.compliance_credits
                )),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.8, 0.5)),
            ));

            // Upgrade buttons.
            for (i, name) in UPGRADE_NAMES.iter().enumerate() {
                let rank = meta.filing_cabinet_ranks[i];
                let cost = upgrade_cost(rank);
                let can_afford = meta.compliance_credits >= cost;

                let bg_color = if can_afford {
                    Color::srgb(0.15, 0.18, 0.12)
                } else {
                    Color::srgb(0.12, 0.10, 0.10)
                };

                parent
                    .spawn((
                        UpgradeButton(i),
                        Button,
                        BackgroundColor(bg_color),
                        Node {
                            width: Val::Percent(90.0),
                            height: Val::Px(50.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(16.0)),
                            ..default()
                        },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(format!("{} (Rank {})", name, rank)),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.85, 0.82, 0.75)),
                        ));
                        btn.spawn((
                            Text::new(format!("Cost: {}", cost)),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(if can_afford {
                                Color::srgb(0.5, 0.9, 0.4)
                            } else {
                                Color::srgb(0.7, 0.3, 0.3)
                            }),
                        ));
                    });
            }

            // Close hint.
            parent.spawn((
                Text::new("[Escape] Close"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.45)),
                Node {
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
            ));
        });
}

/// Handle buying upgrades and closing the filing cabinet.
fn filing_cabinet_ui_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    cabinet_query: Query<Entity, With<FilingCabinetUI>>,
) {
    if cabinet_query.is_empty() {
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        for entity in &cabinet_query {
            commands.entity(entity).despawn();
        }
    }
}

/// Handle click-to-buy on upgrade buttons.
fn filing_cabinet_buy_system(
    mut commands: Commands,
    mut meta: ResMut<MetaProgression>,
    mut interaction_query: Query<
        (&Interaction, &UpgradeButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    cabinet_query: Query<Entity, With<FilingCabinetUI>>,
) {
    if cabinet_query.is_empty() {
        return;
    }

    let mut rebuild = false;

    for (interaction, btn, mut bg) in &mut interaction_query {
        let idx = btn.0;
        let rank = meta.filing_cabinet_ranks[idx];
        let cost = upgrade_cost(rank);
        let can_afford = meta.compliance_credits >= cost;

        match *interaction {
            Interaction::Pressed => {
                if can_afford {
                    meta.compliance_credits -= cost;
                    meta.filing_cabinet_ranks[idx] += 1;
                    info!(
                        "Purchased upgrade {}: rank {} -> {} (credits remaining: {})",
                        UPGRADE_NAMES[idx],
                        rank,
                        rank + 1,
                        meta.compliance_credits
                    );
                    rebuild = true;
                }
            }
            Interaction::Hovered => {
                bg.0 = if can_afford {
                    Color::srgb(0.22, 0.26, 0.18)
                } else {
                    Color::srgb(0.16, 0.13, 0.13)
                };
            }
            Interaction::None => {
                bg.0 = if can_afford {
                    Color::srgb(0.15, 0.18, 0.12)
                } else {
                    Color::srgb(0.12, 0.10, 0.10)
                };
            }
        }
    }

    // Rebuild the entire cabinet UI to reflect updated ranks/costs.
    if rebuild {
        for entity in &cabinet_query {
            commands.entity(entity).despawn();
        }
        let meta_snapshot = MetaProgression {
            compliance_credits: meta.compliance_credits,
            filing_cabinet_ranks: meta.filing_cabinet_ranks,
        };
        spawn_filing_cabinet_ui(&mut commands, &meta_snapshot);
    }
}

// ---------------------------------------------------------------------------
// Results screen
// ---------------------------------------------------------------------------

fn setup_results(
    mut commands: Commands,
    run_summary: Option<Res<RunSummary>>,
    mut meta: ResMut<MetaProgression>,
) {
    let summary = run_summary
        .as_deref()
        .cloned()
        .unwrap_or_default();

    // Award compliance credits.
    let credits_earned = summary.rooms_cleared * 10;
    meta.compliance_credits += credits_earned;

    let header = if summary.run_complete {
        "RUN COMPLETE"
    } else {
        "RUN FAILED"
    };

    let header_color = if summary.run_complete {
        Color::srgb(0.2, 0.9, 0.3)
    } else {
        Color::srgb(0.9, 0.2, 0.2)
    };

    commands
        .spawn((
            ResultsUI,
            BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.92)),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Header.
            parent.spawn((
                Text::new(header),
                TextFont {
                    font_size: 52.0,
                    ..default()
                },
                TextColor(header_color),
            ));

            // Stats.
            let stats = [
                format!("Rooms cleared: {}", summary.rooms_cleared),
                format!("Enemies killed: {}", summary.enemies_killed),
                format!("Deductions earned: {}", summary.deductions_earned),
                format!("Compliance Credits earned: {}", credits_earned),
            ];

            for stat in &stats {
                parent.spawn((
                    Text::new(stat.clone()),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.78, 0.7)),
                ));
            }

            // Total credits.
            parent.spawn((
                Text::new(format!(
                    "Total Compliance Credits: {}",
                    meta.compliance_credits
                )),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.8, 0.5)),
                Node {
                    margin: UiRect::top(Val::Px(12.0)),
                    ..default()
                },
            ));

            // Continue prompt.
            parent.spawn((
                Text::new("[Press Enter to return to office]"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.45)),
                Node {
                    margin: UiRect::top(Val::Px(24.0)),
                    ..default()
                },
            ));
        });
}

fn cleanup_results(
    mut commands: Commands,
    query: Query<Entity, With<ResultsUI>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<RunSummary>();
}
