use bevy::prelude::*;

use crate::app_state::AppState;
use crate::plugins::input::GameInput;

pub struct PausePlugin;

impl Plugin for PausePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PauseState>()
            .add_systems(
                Update,
                toggle_pause_system.run_if(in_state(AppState::Run)),
            )
            .add_systems(OnEnter(PauseState::Paused), (enter_pause, spawn_pause_ui))
            .add_systems(OnExit(PauseState::Paused), (exit_pause, despawn_pause_ui))
            .add_systems(
                Update,
                pause_menu_interaction_system.run_if(in_state(PauseState::Paused)),
            );
    }
}

/// Whether the game is playing or paused.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PauseState {
    #[default]
    Playing,
    Paused,
}

/// Marker for the pause overlay root node.
#[derive(Component)]
struct PauseOverlay;

/// Identifies which pause menu button was clicked.
#[derive(Component, Clone, Copy)]
enum PauseButton {
    Resume,
    QuitToMenu,
    QuitToDesktop,
}

/// Toggle pause state when Escape (or gamepad Start) is pressed during a run.
fn toggle_pause_system(
    game_input: Res<GameInput>,
    current_pause: Res<State<PauseState>>,
    mut next_pause: ResMut<NextState<PauseState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // The GameInput pause_pressed flag covers both keyboard Escape and gamepad Start.
    // However, when paused the gather_input_system may not run (it's gated on AppState::Run,
    // but Time is paused, not the state). We also listen directly for Escape here so that
    // unpausing always works.
    let pressed = game_input.pause_pressed || keyboard.just_pressed(KeyCode::Escape);
    if pressed {
        match current_pause.get() {
            PauseState::Playing => next_pause.set(PauseState::Paused),
            PauseState::Paused => next_pause.set(PauseState::Playing),
        }
    }
}

/// Freeze game time when entering pause.
fn enter_pause(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

/// Resume game time when leaving pause.
fn exit_pause(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

/// Spawn the pause menu UI overlay.
fn spawn_pause_ui(mut commands: Commands) {
    commands
        .spawn((
            PauseOverlay,
            // Dark semi-transparent fullscreen backdrop.
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            // Render on top of everything.
            GlobalZIndex(100),
        ))
        .with_children(|parent| {
            // Centered menu container.
            parent
                .spawn((
                    BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.95)),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(40.0)),
                        row_gap: Val::Px(20.0),
                        ..default()
                    },
                ))
                .with_children(|menu| {
                    // "PAUSED" title.
                    menu.spawn((
                        Text::new("PAUSED"),
                        TextFont {
                            font_size: 48.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.85, 0.7)),
                        Node {
                            margin: UiRect::bottom(Val::Px(20.0)),
                            ..default()
                        },
                    ));

                    // Resume button.
                    spawn_pause_button(menu, "Resume", PauseButton::Resume);
                    // Quit to Menu button.
                    spawn_pause_button(menu, "Quit to Menu", PauseButton::QuitToMenu);
                    // Quit to Desktop button.
                    spawn_pause_button(menu, "Quit to Desktop", PauseButton::QuitToDesktop);
                });
        });
}

/// Helper: spawn a styled pause menu button.
fn spawn_pause_button(parent: &mut ChildSpawnerCommands, label: &str, button: PauseButton) {
    parent
        .spawn((
            button,
            Button,
            BackgroundColor(Color::srgb(0.2, 0.2, 0.25)),
            Node {
                width: Val::Px(260.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

/// Despawn the pause overlay.
fn despawn_pause_ui(mut commands: Commands, query: Query<Entity, With<PauseOverlay>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Handle pause menu button interactions (hover, click).
fn pause_menu_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &PauseButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut next_pause: ResMut<NextState<PauseState>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut exit_events: MessageWriter<AppExit>,
) {
    for (interaction, button, mut bg) in &mut interaction_query {
        match interaction {
            Interaction::Pressed => {
                match button {
                    PauseButton::Resume => {
                        next_pause.set(PauseState::Playing);
                    }
                    PauseButton::QuitToMenu => {
                        next_pause.set(PauseState::Playing);
                        next_app_state.set(AppState::Menu);
                    }
                    PauseButton::QuitToDesktop => {
                        exit_events.write(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => {
                bg.0 = Color::srgb(0.35, 0.35, 0.4);
            }
            Interaction::None => {
                bg.0 = Color::srgb(0.2, 0.2, 0.25);
            }
        }
    }
}
