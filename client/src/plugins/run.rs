use bevy::prelude::*;

use pot_shared::enemy_defs::EnemyBehavior;

use crate::app_state::{AppState, CombatPhase};
use crate::components::combat::Hurtbox;
use crate::components::enemy::Enemy;
use crate::components::player::*;
use crate::content::{CombatFeelConfig, EnemyDefs, LoadingTipsDefs};
use crate::plugins::camera::ShakeQueue;
use crate::plugins::enemies::SpawnEnemyMsg;
use crate::plugins::vfx::ScreenFlashMsg;
use crate::rendering::isometric::{WorldPosition, world_to_screen, z_layers};

pub struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Run), setup_run)
            .add_systems(OnExit(AppState::Run), cleanup_run)
            // TEMP: terrain testing mode -- only fog + vegetation animation
            .add_systems(Update, (
                fog_drift_system,
                vegetation_sway_system,
            ).chain().run_if(in_state(AppState::Run)));
    }
}

/// Tracks run progression.
#[derive(Resource, Clone, Debug)]
pub struct RunStateRes {
    pub current_room: u32,
    pub total_rooms: u32,
    pub rooms_cleared: u32,
    pub is_boss_room: bool,
    pub run_complete: bool,
    pub run_failed: bool,
    pub enemies_killed: u32,
    pub deductions_earned: u32,
    /// Stores the selected room type for the current room.
    pub selected_room_type: Option<RoomType>,
    /// Number of enemies spawned in the current room (for kill tracking).
    pub enemies_this_room: u32,
}

impl Default for RunStateRes {
    fn default() -> Self {
        Self {
            current_room: 0,
            total_rooms: 5,
            rooms_cleared: 0,
            is_boss_room: false,
            run_complete: false,
            run_failed: false,
            enemies_killed: 0,
            deductions_earned: 0,
            selected_room_type: None,
            enemies_this_room: 0,
        }
    }
}

/// Types of rooms the player can choose from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomType {
    Combat,
    HardCombat,
    Treasure,
    Rest,
}

impl RoomType {
    /// Display name for the room type.
    pub fn label(&self) -> &'static str {
        match self {
            RoomType::Combat => "Combat",
            RoomType::HardCombat => "Hard Combat",
            RoomType::Treasure => "Treasure",
            RoomType::Rest => "Rest",
        }
    }

    /// Enemy count for this room type at the given room number.
    pub fn enemy_count(&self, room: u32) -> u32 {
        match self {
            RoomType::Combat => 3 + room,
            RoomType::HardCombat => 5 + room * 2,
            RoomType::Treasure => 1 + room / 2,
            RoomType::Rest => 0,
        }
    }
}

/// Generate 2-3 room options deterministically from the current room number.
fn generate_room_options(room: u32) -> Vec<RoomType> {
    // Simple deterministic hash from room number.
    let h = room.wrapping_mul(2654435761);
    let all_types = [RoomType::Combat, RoomType::HardCombat, RoomType::Treasure, RoomType::Rest];

    let count = if h % 3 == 0 { 2 } else { 3 };
    let mut options = Vec::with_capacity(count);

    for i in 0..count {
        let idx = ((h >> (i * 4)) as usize + i) % all_types.len();
        let room_type = all_types[idx];
        // Avoid duplicates.
        if !options.contains(&room_type) {
            options.push(room_type);
        } else {
            // Pick next available type.
            for t in &all_types {
                if !options.contains(t) {
                    options.push(*t);
                    break;
                }
            }
        }
    }

    options
}

/// Marker for room selection UI entities.
#[derive(Component)]
pub struct RoomSelectUI;

/// Marker for a room door button, storing which option index it represents.
#[derive(Component)]
pub struct RoomDoorButton(pub usize);

/// Marker for arena floor entities.
#[derive(Component)]
pub struct ArenaEntity;

#[derive(Resource, Clone, Debug, Default)]
pub struct ArenaCollision {
    pub half_extents: Vec2,
    pub blockers: Vec<ArenaBlocker>,
}

#[derive(Clone, Copy, Debug)]
pub struct ArenaBlocker {
    pub center: Vec2,
    pub radius: f32,
}

pub fn resolve_world_collision(position: Vec2, body_radius: f32, arena: &ArenaCollision) -> Vec2 {
    let mut resolved = Vec2::new(
        position
            .x
            .clamp(-arena.half_extents.x + body_radius, arena.half_extents.x - body_radius),
        position
            .y
            .clamp(-arena.half_extents.y + body_radius, arena.half_extents.y - body_radius),
    );

    for blocker in &arena.blockers {
        let delta = resolved - blocker.center;
        let distance = delta.length();
        let min_distance = blocker.radius + body_radius;
        if distance < min_distance {
            let normal = if distance <= f32::EPSILON {
                Vec2::X
            } else {
                delta / distance
            };
            resolved = blocker.center + normal * min_distance;
        }
    }

    Vec2::new(
        resolved
            .x
            .clamp(-arena.half_extents.x + body_radius, arena.half_extents.x - body_radius),
        resolved
            .y
            .clamp(-arena.half_extents.y + body_radius, arena.half_extents.y - body_radius),
    )
}

/// Fog drift component for atmospheric fog sprites.
#[derive(Component)]
pub struct FogDrift {
    pub speed: f32,
    pub phase: f32,
    pub amplitude: Vec2,
    pub base_pos: Vec2,
}

/// Gentle sinusoidal sway for vegetation sprites.
#[derive(Component)]
pub struct VegetationSway {
    pub speed: f32,
    pub phase: f32,
    pub amplitude: f32,
    pub base_x: f32,
}

/// Deterministic hash for pseudo-random tile variation.
fn tile_hash(r: i32, c: i32) -> u32 {
    let a = (r.wrapping_mul(2654435761_u32 as i32)) as u32;
    let b = (c.wrapping_mul(2246822519_u32 as i32)) as u32;
    a ^ b
}

fn setup_run(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RunStateRes::default());
    commands.insert_resource(ClearColor(Color::srgb(0.012, 0.008, 0.006)));

    // === GROUND: Single pre-generated hex terrain image ===
    // Generated by tools/hex_map_builder -- 4096x3072 with hex tiles blended together.
    let ground_handle: Handle<Image> = asset_server.load("sprites/terrain/generated_map_01.png");
    commands.spawn((
        ArenaEntity,
        Sprite {
            image: ground_handle,
            custom_size: Some(Vec2::new(2800.0, 2100.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, z_layers::TERRAIN_BASE)),
    ));

    let tile_spacing: f32 = 72.0;
    let arena_radius: i32 = 10;
    let arena_half_extents = Vec2::splat(arena_radius as f32 * tile_spacing - 80.0);
    let blockers = Vec::new();

    // === SCATTER PROPS on top of the pre-generated ground ===
    let rock_handle: Handle<Image> = asset_server.load("sprites/terrain/rock.png");
    let pillar_handle: Handle<Image> = asset_server.load("sprites/terrain/ruined_pillar.png");
    let dead_tree_handle: Handle<Image> = asset_server.load("sprites/terrain/dead_tree.png");

    // Rocks
    for i in 0..15 {
        let h = tile_hash(i * 31 + 5, i * 47 + 11);
        let angle = (i as f32 / 15.0) * std::f32::consts::TAU + (h % 100) as f32 * 0.06;
        let dist = 100.0 + ((h % 100) as f32 / 100.0) * 500.0;
        let wx = angle.cos() * dist;
        let wy = angle.sin() * dist;
        let screen = world_to_screen(wx, wy);
        let scale = 0.6 + ((h % 6) as f32 * 0.12);
        commands.spawn((
            ArenaEntity,
            Sprite {
                image: rock_handle.clone(),
                color: Color::srgb(0.65, 0.62, 0.58),
                custom_size: Some(Vec2::new(50.0 * scale, 50.0 * scale)),
                ..default()
            },
            Transform::from_translation(Vec3::new(screen.x, screen.y, z_layers::GROUND_PROPS + 4.0)),
        ));
    }

    // Ruined pillars
    for i in 0..5 {
        let h = tile_hash(i * 67 + 3, i * 89 + 7);
        let angle = (i as f32 / 5.0) * std::f32::consts::TAU + 0.5;
        let dist = 250.0 + ((h % 60) as f32 * 3.0);
        let wx = angle.cos() * dist;
        let wy = angle.sin() * dist;
        let screen = world_to_screen(wx, wy);
        commands.spawn((
            ArenaEntity,
            Sprite {
                image: pillar_handle.clone(),
                color: Color::srgb(0.55, 0.52, 0.48),
                custom_size: Some(Vec2::new(55.0, 90.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(screen.x, screen.y, z_layers::GROUND_PROPS + 5.0)),
        ));
    }

    // Dead trees at edges
    for i in 0..8 {
        let h = tile_hash(i * 41 + 9, i * 53 + 13);
        let angle = (i as f32 / 8.0) * std::f32::consts::TAU + 0.3;
        let dist = 450.0 + ((h % 40) as f32 * 4.0);
        let wx = angle.cos() * dist;
        let wy = angle.sin() * dist;
        let screen = world_to_screen(wx, wy);
        commands.spawn((
            ArenaEntity,
            Sprite {
                image: dead_tree_handle.clone(),
                color: Color::srgb(0.45, 0.42, 0.38),
                custom_size: Some(Vec2::new(90.0, 200.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(screen.x, screen.y, z_layers::GROUND_PROPS + 6.0)),
        ));
    }

    // === FOG ===
    let fog_texture_handle: Handle<Image> = asset_server.load("sprites/terrain/fog_cloud.png");
    for i in 0..8 {
        let h = tile_hash(i * 19, i * 37);
        let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
        let dist = 150.0 + ((h % 80) as f32 * 4.0);
        let wx = angle.cos() * dist;
        let wy = angle.sin() * dist;
        let screen = world_to_screen(wx, wy);
        commands.spawn((
            ArenaEntity,
            FogDrift {
                speed: 0.2 + ((h % 10) as f32 * 0.02),
                phase: (h % 628) as f32 * 0.01,
                amplitude: Vec2::new(25.0, 10.0),
                base_pos: screen,
            },
            Sprite {
                image: fog_texture_handle.clone(),
                color: Color::srgba(0.25, 0.23, 0.20, 0.08),
                custom_size: Some(Vec2::new(350.0, 200.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(screen.x, screen.y, z_layers::FOG)),
        ));
    }

    // === LAYER 6: VIGNETTE (edge darkness overlay) ===
    // Four gradient bars around the edges to darken the periphery.
    let vig_size = 600.0;
    let vig_offset = 500.0;
    let vig_alpha = 0.5;
    for (ox, oy, w, h) in [
        (0.0, vig_offset, 1600.0, vig_size),    // top
        (0.0, -vig_offset, 1600.0, vig_size),   // bottom
        (-vig_offset - 200.0, 0.0, vig_size, 1200.0), // left
        (vig_offset + 200.0, 0.0, vig_size, 1200.0),  // right
    ] {
        commands.spawn((
            ArenaEntity,
            Sprite {
                color: Color::srgba(0.01, 0.005, 0.005, vig_alpha),
                custom_size: Some(Vec2::new(w, h)),
                ..default()
            },
            Transform::from_translation(Vec3::new(ox, oy, z_layers::VIGNETTE)),
        ));
    }

    commands.insert_resource(ArenaCollision {
        half_extents: arena_half_extents,
        blockers,
    });
}

/// Animate fog sprites with gentle sinusoidal drift.
fn fog_drift_system(
    time: Res<Time>,
    mut query: Query<(&FogDrift, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (fog, mut transform) in &mut query {
        let offset_x = (t * fog.speed + fog.phase).sin() * fog.amplitude.x;
        let offset_y = (t * fog.speed * 0.7 + fog.phase + 1.5).cos() * fog.amplitude.y;
        transform.translation.x = fog.base_pos.x + offset_x;
        transform.translation.y = fog.base_pos.y + offset_y;
    }
}

fn resolve_actor_collisions_system(
    arena: Option<Res<ArenaCollision>>,
    mut query: Query<(&mut WorldPosition, &Hurtbox)>,
) {
    let Some(arena) = arena else {
        return;
    };

    let mut combinations = query.iter_combinations_mut();
    while let Some([(mut a_pos, a_hurtbox), (mut b_pos, b_hurtbox)]) = combinations.fetch_next() {
        let delta = Vec2::new(b_pos.x - a_pos.x, b_pos.y - a_pos.y);
        let distance = delta.length();
        let min_distance = a_hurtbox.radius + b_hurtbox.radius;

        if distance < min_distance && min_distance > 0.0 {
            let normal = if distance <= f32::EPSILON {
                Vec2::X
            } else {
                delta / distance
            };
            let push = (min_distance - distance) * 0.5;

            let a_resolved = resolve_world_collision(
                Vec2::new(a_pos.x, a_pos.y) - normal * push,
                a_hurtbox.radius,
                &arena,
            );
            let b_resolved = resolve_world_collision(
                Vec2::new(b_pos.x, b_pos.y) + normal * push,
                b_hurtbox.radius,
                &arena,
            );

            a_pos.x = a_resolved.x;
            a_pos.y = a_resolved.y;
            b_pos.x = b_resolved.x;
            b_pos.y = b_resolved.y;
        }
    }

    for (mut position, hurtbox) in &mut query {
        let resolved = resolve_world_collision(position.as_vec2(), hurtbox.radius, &arena);
        position.x = resolved.x;
        position.y = resolved.y;
    }
}

/// Resource storing the current room options so input/button systems can reference them.
#[derive(Resource)]
struct RoomOptions(Vec<RoomType>);

/// Spawn the room selection UI with 2-3 door buttons.
fn spawn_room_select_ui(
    mut commands: Commands,
    run_state: Option<Res<RunStateRes>>,
) {
    let room = run_state.as_ref().map(|r| r.current_room).unwrap_or(0);
    let options = generate_room_options(room);
    commands.insert_resource(RoomOptions(options.clone()));

    // Full-screen overlay.
    commands
        .spawn((
            RoomSelectUI,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // Title.
            parent.spawn((
                Text::new("Choose Your Path"),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.7)),
            ));

            // Door buttons container.
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(30.0),
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|row| {
                    for (i, room_type) in options.iter().enumerate() {
                        let enemy_count = room_type.enemy_count(room);
                        let subtitle = if enemy_count > 0 {
                            format!("{} enemies", enemy_count)
                        } else {
                            "No enemies".to_string()
                        };

                        row.spawn((
                            RoomDoorButton(i),
                            Button,
                            BackgroundColor(Color::srgb(0.15, 0.12, 0.10)),
                            Node {
                                width: Val::Px(200.0),
                                height: Val::Px(160.0),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(16.0)),
                                row_gap: Val::Px(8.0),
                                ..default()
                            },
                        ))
                        .with_children(|door| {
                            // Key hint.
                            door.spawn((
                                Text::new(format!("[{}]", i + 1)),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.6, 0.55, 0.45)),
                            ));
                            // Room type label.
                            door.spawn((
                                Text::new(room_type.label()),
                                TextFont {
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.85, 0.7)),
                            ));
                            // Enemy count.
                            door.spawn((
                                Text::new(subtitle),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.6, 0.5, 0.4)),
                            ));
                        });
                    }
                });
        });
}

/// Clean up room selection UI on state exit.
fn cleanup_room_select_ui(
    mut commands: Commands,
    query: Query<Entity, With<RoomSelectUI>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<RoomOptions>();
}

/// Handle keyboard input (1/2/3) for room selection.
fn room_select_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    combat_phase: Res<State<CombatPhase>>,
    mut next_phase: ResMut<NextState<CombatPhase>>,
    mut run_state: Option<ResMut<RunStateRes>>,
    room_options: Option<Res<RoomOptions>>,
) {
    if *combat_phase.get() != CombatPhase::RoomSelect {
        return;
    }

    let Some(ref options) = room_options else {
        return;
    };

    let selected = if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
        Some(0)
    } else if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2) {
        if options.0.len() > 1 { Some(1) } else { None }
    } else if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Numpad3) {
        if options.0.len() > 2 { Some(2) } else { None }
    } else {
        None
    };

    if let Some(idx) = selected {
        if let Some(ref mut run) = run_state {
            run.selected_room_type = Some(options.0[idx]);
            info!("Selected room: {:?}", options.0[idx]);
        }
        next_phase.set(CombatPhase::Combat);
    }
}

/// Handle mouse click on room door buttons.
fn room_select_button_system(
    combat_phase: Res<State<CombatPhase>>,
    mut next_phase: ResMut<NextState<CombatPhase>>,
    mut run_state: Option<ResMut<RunStateRes>>,
    room_options: Option<Res<RoomOptions>>,
    mut interaction_query: Query<
        (&Interaction, &RoomDoorButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    if *combat_phase.get() != CombatPhase::RoomSelect {
        return;
    }

    let Some(ref options) = room_options else {
        return;
    };

    for (interaction, door, mut bg) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if door.0 < options.0.len() {
                    if let Some(ref mut run) = run_state {
                        run.selected_room_type = Some(options.0[door.0]);
                        info!("Selected room via click: {:?}", options.0[door.0]);
                    }
                    next_phase.set(CombatPhase::Combat);
                }
            }
            Interaction::Hovered => {
                bg.0 = Color::srgb(0.25, 0.20, 0.16);
            }
            Interaction::None => {
                bg.0 = Color::srgb(0.15, 0.12, 0.10);
            }
        }
    }
}

/// Marker for room clear transition UI entities.
#[derive(Component)]
pub struct RoomClearUI;

/// Spawn the "Room Cleared!" transition UI with a random loading tip.
fn spawn_room_clear_ui(
    mut commands: Commands,
    run_state: Option<Res<RunStateRes>>,
    tips: Option<Res<LoadingTipsDefs>>,
) {
    let room = run_state.as_ref().map(|r| r.current_room).unwrap_or(0);

    // Pick a tip deterministically from room number (acts as a seed).
    let tip_text = if let Some(ref tips) = tips {
        // Mix room number for variety across clears.
        let tip_idx = (room as usize).wrapping_mul(7919);
        tips.get_tip(tip_idx).to_string()
    } else {
        "Tip: No tips loaded.".to_string()
    };

    commands
        .spawn((
            RoomClearUI,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(24.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            // "Room Cleared!" text with shadow.
            parent
                .spawn(Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|container| {
                    // Shadow.
                    container.spawn((
                        Text::new("Room Cleared!"),
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
                    // Foreground.
                    container.spawn((
                        Text::new("Room Cleared!"),
                        TextFont {
                            font_size: 48.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.2, 0.9, 0.3)),
                    ));
                });

            // Loading tip.
            parent.spawn((
                Text::new(tip_text),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.65, 0.55)),
                Node {
                    max_width: Val::Px(600.0),
                    ..default()
                },
            ));

            // Continue prompt.
            parent.spawn((
                Text::new("[Press Enter to continue]"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.45)),
            ));
        });
}

/// Clean up room clear UI on state exit.
fn cleanup_room_clear_ui(
    mut commands: Commands,
    query: Query<Entity, With<RoomClearUI>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn cleanup_run(
    mut commands: Commands,
    arena_query: Query<Entity, With<ArenaEntity>>,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    commands.remove_resource::<RunStateRes>();
    commands.remove_resource::<ArenaCollision>();
    for entity in arena_query.iter().chain(enemy_query.iter()) {
        commands.entity(entity).despawn();
    }
}

fn encounter_layout_for_room(room: u32, room_type: RoomType) -> Vec<(&'static str, Vec2)> {
    match (room, room_type) {
        (0, RoomType::Combat) => vec![
            ("undead_accountant", Vec2::new(-180.0, 140.0)),
            ("paper_shredder", Vec2::new(-60.0, -180.0)),
            ("paper_shredder", Vec2::new(60.0, -210.0)),
            ("tax_collector", Vec2::new(220.0, 80.0)),
        ],
        (0, RoomType::HardCombat) => vec![
            ("undead_accountant", Vec2::new(-220.0, 130.0)),
            ("tax_collector", Vec2::new(200.0, 120.0)),
            ("paper_shredder", Vec2::new(-120.0, -180.0)),
            ("paper_shredder", Vec2::new(-10.0, -220.0)),
            ("paper_shredder", Vec2::new(105.0, -200.0)),
            ("ink_crawler", Vec2::new(250.0, -40.0)),
        ],
        (_, RoomType::Treasure) => vec![
            ("paper_shredder", Vec2::new(0.0, 180.0)),
            ("ink_crawler", Vec2::new(180.0, -40.0)),
        ],
        (_, RoomType::Rest) => Vec::new(),
        (1, RoomType::Combat) => vec![
            ("tax_collector", Vec2::new(-210.0, 110.0)),
            ("ink_crawler", Vec2::new(210.0, 150.0)),
            ("paper_shredder", Vec2::new(-120.0, -180.0)),
            ("paper_shredder", Vec2::new(20.0, -210.0)),
            ("enforcement_agent", Vec2::new(250.0, -20.0)),
        ],
        (_, RoomType::HardCombat) => vec![
            ("tax_collector", Vec2::new(-230.0, 140.0)),
            ("tax_collector", Vec2::new(230.0, 120.0)),
            ("paper_shredder", Vec2::new(-140.0, -170.0)),
            ("paper_shredder", Vec2::new(-20.0, -220.0)),
            ("paper_shredder", Vec2::new(110.0, -185.0)),
            ("ink_crawler", Vec2::new(260.0, -30.0)),
            ("red_tape_weaver", Vec2::new(0.0, 230.0)),
        ],
        _ => vec![
            ("tax_collector", Vec2::new(-210.0, 125.0)),
            ("paper_shredder", Vec2::new(-90.0, -165.0)),
            ("paper_shredder", Vec2::new(60.0, -195.0)),
            ("ink_crawler", Vec2::new(210.0, 130.0)),
            ("enforcement_agent", Vec2::new(260.0, -35.0)),
        ],
    }
}

fn fallback_spawn_positions(enemy_count: u32) -> Vec<Vec2> {
    let radius = 320.0;
    (0..enemy_count)
        .map(|i| {
            let angle = (i as f32 / enemy_count.max(1) as f32) * std::f32::consts::TAU;
            Vec2::new(angle.cos() * radius, angle.sin() * radius)
        })
        .collect()
}

/// Spawn enemies for the current room using data-driven definitions.
fn spawn_room_enemies(
    mut run_state: Option<ResMut<RunStateRes>>,
    enemy_defs: Res<EnemyDefs>,
    patch_mods: Option<Res<crate::plugins::patch_notes::PatchNoteModifiers>>,
    mut spawn_msgs: MessageWriter<SpawnEnemyMsg>,
) {
    // TEMP: disabled for terrain testing
    return;
    #[allow(unreachable_code)]
    let Some(ref mut run) = run_state else {
        return;
    };

    let room_type = run.selected_room_type.unwrap_or(RoomType::Combat);
    let authored_layout = encounter_layout_for_room(run.current_room, room_type);
    let enemy_count = if authored_layout.is_empty() {
        room_type.enemy_count(run.current_room)
    } else {
        authored_layout.len() as u32
    };
    run.enemies_this_room = enemy_count;

    let fallback_positions = fallback_spawn_positions(enemy_count);

    for (i, position) in fallback_positions.iter().enumerate().take(enemy_count as usize) {
        let (key, position) = authored_layout
            .get(i)
            .map(|(key, pos)| (*key, *pos))
            .unwrap_or(("tax_collector", *position));

        // Look up enemy stats from loaded definitions; fall back to defaults.
        let (
            enemy_type,
            hp,
            damage,
            speed,
            aggro_range,
            attack_range,
            attack_cooldown_frames,
            windup_frames,
            behavior,
        ) =
            if let Some(def) = enemy_defs.get_by_key(key) {
                (
                    def.key.clone(),
                    def.base_hp as f32 + (run.current_room as f32 * 10.0),
                    def.base_damage as f32,
                    def.move_speed,
                    def.aggro_range,
                    def.attack_range,
                    (def.attack_cooldown_ms as f32 / 16.67).round() as u32,
                    (def.windup_ms as f32 / 16.67).round() as u32,
                    def.behavior,
                )
            } else {
                (
                    key.to_string(),
                    60.0 + (run.current_room as f32 * 10.0),
                    10.0,
                    100.0,
                    300.0,
                    50.0,
                    60,
                    24,
                    EnemyBehavior::Chase,
                )
            };

        // Apply patch note enemy HP modifier if present.
        let final_hp = if let Some(ref mods) = patch_mods {
            hp * mods.enemy_hp_mult
        } else {
            hp
        };

        spawn_msgs.write(SpawnEnemyMsg {
            enemy_type,
            position,
            hp: final_hp,
            damage,
            speed,
            aggro_range,
            attack_range,
            attack_cooldown_frames,
            windup_frames,
            behavior,
        });
    }
}

/// Animate vegetation with gentle sinusoidal sway.
fn vegetation_sway_system(
    time: Res<Time>,
    mut query: Query<(&VegetationSway, &mut Transform)>,
) {
    let t = time.elapsed_secs();
    for (sway, mut transform) in &mut query {
        let offset_x = (t * sway.speed + sway.phase).sin() * sway.amplitude;
        transform.translation.x = sway.base_x + offset_x;
    }
}

/// Detect when all enemies in a room are defeated.
fn room_clear_detection_system(
    enemy_query: Query<Entity, With<Enemy>>,
    combat_phase: Res<State<CombatPhase>>,
    mut next_phase: ResMut<NextState<CombatPhase>>,
    mut run_state: Option<ResMut<RunStateRes>>,
    // TEMP: skip room clear for terrain testing
) {
    return;
}

#[allow(dead_code, unreachable_code)]
fn _room_clear_detection_system_disabled(
    enemy_query: Query<Entity, With<Enemy>>,
    combat_phase: Res<State<CombatPhase>>,
    mut next_phase: ResMut<NextState<CombatPhase>>,
    mut run_state: Option<ResMut<RunStateRes>>,
    feel: Res<CombatFeelConfig>,
    mut shake_queue: ResMut<ShakeQueue>,
    mut screen_flash_msgs: MessageWriter<ScreenFlashMsg>,
) {
    if *combat_phase.get() != CombatPhase::Combat {
        return;
    }

    if enemy_query.is_empty() {
        if let Some(ref mut run) = run_state {
            // enemies_killed is already incremented per-enemy in enemy_death_system.
            // Award deductions: 10 per enemy killed this room.
            run.deductions_earned += run.enemies_this_room * 10;
            run.rooms_cleared += 1;
            next_phase.set(CombatPhase::RoomClear);
            shake_queue.push(Vec2::new(0.0, 1.0), feel.room_clear_shake_intensity, feel.room_clear_shake_frames);
            // White screen flash on room clear.
            screen_flash_msgs.write(ScreenFlashMsg {
                color: Color::srgba(1.0, 1.0, 1.0, 0.8),
            });
        }
    }
}

/// Check for win/lose conditions.
fn win_lose_detection_system(
    player_query: Query<&Health, With<Player>>,
    mut run_state: Option<ResMut<RunStateRes>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    let Ok(health) = player_query.single() else {
        return;
    };

    let Some(ref mut run) = run_state else {
        return;
    };

    // Lose: player dies.
    if health.is_dead() && !run.run_failed {
        run.run_failed = true;
    }

    // Win: all rooms cleared.
    if run.rooms_cleared >= run.total_rooms && !run.run_complete {
        run.run_complete = true;
        next_app_state.set(AppState::Results);
    }
}

/// Handle transitioning between rooms.
fn room_transition_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    combat_phase: Res<State<CombatPhase>>,
    mut next_phase: ResMut<NextState<CombatPhase>>,
    mut run_state: Option<ResMut<RunStateRes>>,
) {
    if *combat_phase.get() != CombatPhase::RoomClear {
        return;
    }

    // Press Enter to proceed to next room selection.
    if keyboard.just_pressed(KeyCode::Enter) {
        if let Some(ref mut run) = run_state {
            run.current_room += 1;
            if run.current_room >= run.total_rooms {
                // Boss room.
                run.is_boss_room = true;
                next_phase.set(CombatPhase::BossIntro);
            } else {
                next_phase.set(CombatPhase::RoomSelect);
            }
        }
    }
}
