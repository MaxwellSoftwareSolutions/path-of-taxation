use bevy::prelude::*;

use crate::combat::RunOutcome;
use crate::player::PlayerHealth;
use crate::state::ScreenState;

pub struct UiPlugin;

#[derive(Component)]
struct ScreenText;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui_camera)
            .add_systems(OnEnter(ScreenState::Hub), spawn_hub_ui)
            .add_systems(OnEnter(ScreenState::Run), spawn_run_ui)
            .add_systems(OnEnter(ScreenState::Debate), spawn_debate_ui)
            .add_systems(OnExit(ScreenState::Hub), cleanup_ui)
            .add_systems(OnExit(ScreenState::Run), cleanup_ui)
            .add_systems(OnExit(ScreenState::Debate), cleanup_ui)
            .add_systems(Update, update_run_ui.run_if(in_state(ScreenState::Run)))
            .add_systems(Update, handle_debug_state_switches);
    }
}

fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_hub_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Hub: Press Space to start a run"),
        Node {
            position_type: PositionType::Absolute,
            top: px(24.0),
            left: px(24.0),
            ..default()
        },
        ScreenText,
    ));
}

fn spawn_run_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Run: Move with WASD or arrows. Press Space to cast. Press Tab to retreat."),
        Node {
            position_type: PositionType::Absolute,
            top: px(24.0),
            left: px(24.0),
            ..default()
        },
        ScreenText,
    ));
}

fn spawn_debate_ui(mut commands: Commands, outcome: Res<RunOutcome>) {
    let body = if outcome.room_cleared {
        "Debate Club: You cleared the room. Press Enter to claim a fake theorycraft win and return to the hub."
    } else {
        "Debate Club: Press Enter to return to the hub."
    };

    commands.spawn((
        Text::new(body),
        Node {
            position_type: PositionType::Absolute,
            top: px(24.0),
            left: px(24.0),
            ..default()
        },
        ScreenText,
    ));
}

fn cleanup_ui(mut commands: Commands, text_entities: Query<Entity, With<ScreenText>>) {
    for entity in &text_entities {
        commands.entity(entity).despawn();
    }
}

fn update_run_ui(
    mut texts: Query<&mut Text, With<ScreenText>>,
    players: Query<&PlayerHealth>,
    enemies: Query<Entity, With<crate::combat::Enemy>>,
) {
    let Ok(mut text) = texts.single_mut() else {
        return;
    };

    let health_line = match players.single() {
        Ok(health) => format!("Health: {}/{}", health.current.max(0), health.max),
        Err(_) => "Health: --".to_string(),
    };

    let enemy_count = enemies.iter().count();
    *text = Text::new(format!(
        "Run: Move with WASD or arrows. Press Space to cast. Press Tab to retreat.\n{} | Enemies: {}",
        health_line, enemy_count
    ));
}

fn handle_debug_state_switches(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<ScreenState>>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    match state.get() {
        ScreenState::Hub if keyboard.just_pressed(KeyCode::Space) => {
            next_state.set(ScreenState::Run);
        }
        ScreenState::Run if keyboard.just_pressed(KeyCode::Tab) => {
            next_state.set(ScreenState::Hub);
        }
        ScreenState::Debate if keyboard.just_pressed(KeyCode::Enter) => {
            next_state.set(ScreenState::Hub);
        }
        _ => {}
    }
}
