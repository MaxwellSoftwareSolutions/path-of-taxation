use bevy::prelude::*;

use crate::app_state::CombatPhase;
use crate::components::player::*;

pub struct PatchNotesPlugin;

impl Plugin for PatchNotesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PatchNoteModifiers>()
            .add_systems(OnEnter(CombatPhase::RoomSelect), maybe_show_patch_notes)
            .add_systems(OnExit(CombatPhase::RoomSelect), cleanup_patch_notes_ui)
            .add_systems(
                Update,
                patch_notes_accept_system.run_if(resource_exists::<PatchNotesShowing>),
            );
    }
}

/// Stores all stat modifiers applied by mid-run patch notes.
#[derive(Resource, Clone, Debug)]
pub struct PatchNoteModifiers {
    pub damage_mult: f32,
    pub speed_mult: f32,
    pub mana_cost_mult: f32,
    pub enemy_hp_mult: f32,
    pub dodge_speed_mult: f32,
}

impl Default for PatchNoteModifiers {
    fn default() -> Self {
        Self {
            damage_mult: 1.0,
            speed_mult: 1.0,
            mana_cost_mult: 1.0,
            enemy_hp_mult: 1.0,
            dodge_speed_mult: 1.0,
        }
    }
}

/// Marker resource indicating the patch notes overlay is currently showing.
#[derive(Resource)]
struct PatchNotesShowing;

/// Marker for patch notes UI root.
#[derive(Component)]
struct PatchNotesUI;

/// Marker for the "ACCEPT AND CONTINUE" button.
#[derive(Component)]
struct PatchNotesAcceptButton;

/// A single patch note entry with its effect.
struct PatchNoteEntry {
    text: &'static str,
    apply: fn(&mut PatchNoteModifiers),
    apply_player: fn(&mut MovementSpeed, &mut DodgeState),
}

/// All possible patch notes.
fn all_patch_notes() -> Vec<PatchNoteEntry> {
    vec![
        PatchNoteEntry {
            text: "Tax Bolt damage reduced by 15%. This is a buff.",
            apply: |m| m.damage_mult *= 0.85,
            apply_player: |_, _| {},
        },
        PatchNoteEntry {
            text: "Movement speed increased by 10% for fairness.",
            apply: |m| m.speed_mult *= 1.1,
            apply_player: |speed, _| speed.0 *= 1.1,
        },
        PatchNoteEntry {
            text: "Mana costs reduced by 20%. We're generous.",
            apply: |m| m.mana_cost_mult *= 0.8,
            apply_player: |_, _| {},
        },
        PatchNoteEntry {
            text: "Enemy HP increased by 25%. Challenge is content.",
            apply: |m| m.enemy_hp_mult *= 1.25,
            apply_player: |_, _| {},
        },
        PatchNoteEntry {
            text: "Dodge roll distance increased. You're welcome.",
            apply: |m| m.dodge_speed_mult *= 1.15,
            apply_player: |_, dodge| dodge.speed *= 1.15,
        },
    ]
}

/// 15% chance on entering RoomSelect to show a patch notes overlay.
/// Uses a frame-count based seed for determinism.
fn maybe_show_patch_notes(
    mut commands: Commands,
    time: Res<Time>,
    modifiers: Res<PatchNoteModifiers>,
    mut player_query: Query<(&mut MovementSpeed, &mut DodgeState), With<Player>>,
) {
    // Derive a seed from elapsed time (acts as pseudo-random).
    let seed = (time.elapsed_secs() * 10000.0) as u32;
    let roll = seed % 100;

    // 15% chance.
    if roll >= 15 {
        return;
    }

    // Pick 2-3 patch notes deterministically.
    let all_notes = all_patch_notes();
    let count = if seed % 3 == 0 { 2 } else { 3 };
    let mut selected_indices: Vec<usize> = Vec::with_capacity(count);
    let mut h = seed;
    while selected_indices.len() < count {
        h = h.wrapping_mul(2654435761).wrapping_add(1);
        let idx = (h as usize) % all_notes.len();
        if !selected_indices.contains(&idx) {
            selected_indices.push(idx);
        }
    }

    // Apply modifiers to resource and player.
    let mut new_modifiers = modifiers.clone();
    let mut note_texts: Vec<&str> = Vec::new();
    for &idx in &selected_indices {
        let entry = &all_notes[idx];
        (entry.apply)(&mut new_modifiers);
        if let Ok((mut speed, mut dodge)) = player_query.single_mut() {
            (entry.apply_player)(&mut speed, &mut dodge);
        }
        note_texts.push(entry.text);
    }
    commands.insert_resource(new_modifiers);
    commands.insert_resource(PatchNotesShowing);

    // Spawn the overlay UI.
    commands
        .spawn((
            PatchNotesUI,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
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
            // High z-index so it renders above the room select UI.
            ZIndex(100),
        ))
        .with_children(|parent| {
            // Title: "PATCH NOTES"
            parent.spawn((
                Text::new("PATCH NOTES"),
                TextFont {
                    font_size: 44.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.3)),
            ));

            // Subtitle.
            parent.spawn((
                Text::new("The following changes have been applied to your run:"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.65, 0.55)),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ));

            // Individual patch notes.
            for text in &note_texts {
                parent
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        max_width: Val::Px(600.0),
                        ..default()
                    })
                    .with_children(|row| {
                        // Bullet.
                        row.spawn((
                            Text::new("-"),
                            TextFont {
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.3, 0.3)),
                        ));
                        // Note text.
                        row.spawn((
                            Text::new(*text),
                            TextFont {
                                font_size: 20.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.87, 0.8)),
                        ));
                    });
            }

            // "ACCEPT AND CONTINUE" button (no way to decline).
            parent
                .spawn((
                    PatchNotesAcceptButton,
                    Button,
                    BackgroundColor(Color::srgb(0.6, 0.15, 0.15)),
                    Node {
                        padding: UiRect::axes(Val::Px(32.0), Val::Px(12.0)),
                        margin: UiRect::top(Val::Px(24.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("ACCEPT AND CONTINUE"),
                        TextFont {
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.85)),
                    ));
                });

            // Small disclaimer.
            parent.spawn((
                Text::new("(You have no choice.)"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.45, 0.4)),
                Node {
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));
        });
}

/// Handle clicking the accept button or pressing Enter.
fn patch_notes_accept_system(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<PatchNotesAcceptButton>),
    >,
) {
    let mut accepted = false;

    // Mouse click on button.
    for (interaction, mut bg) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                accepted = true;
            }
            Interaction::Hovered => {
                bg.0 = Color::srgb(0.75, 0.2, 0.2);
            }
            Interaction::None => {
                bg.0 = Color::srgb(0.6, 0.15, 0.15);
            }
        }
    }

    // Enter key also accepts.
    if keyboard.just_pressed(KeyCode::Enter) {
        accepted = true;
    }

    if accepted {
        commands.remove_resource::<PatchNotesShowing>();
    }
}

/// Clean up patch notes UI when leaving RoomSelect.
fn cleanup_patch_notes_ui(
    mut commands: Commands,
    query: Query<Entity, With<PatchNotesUI>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<PatchNotesShowing>();
}
