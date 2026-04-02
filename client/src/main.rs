// Phase 0+1 foundation: many components/fields are scaffolded for future systems.
#![allow(dead_code)]

mod app_state;
mod components;
mod content;
mod plugins;
mod rendering;

use bevy::prelude::*;

use app_state::{AppState, CombatPhase};
use plugins::{
    boss::BossPlugin,
    camera::CameraPlugin,
    combat::CombatPlugin,
    enemies::EnemiesPlugin,
    hub::{HubPlugin, MetaProgression, RunSummary},
    input::InputPlugin,
    loot::LootPlugin,
    patch_notes::PatchNotesPlugin,
    pause::PausePlugin,
    player::PlayerPlugin,
    run::RunPlugin,
    ui::UiPlugin,
    vfx::VfxPlugin,
};
use content::ContentPlugin;
use rendering::isometric::IsometricPlugin;
use rendering::sprites::SpriteGenPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Path of Taxation".to_string(),
                        resolution: (1920, 1080).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        // State machines.
        .init_state::<AppState>()
        .add_sub_state::<CombatPhase>()
        // Persistent resources.
        .init_resource::<MetaProgression>()
        // Game plugins.
        .add_plugins((
            ContentPlugin,
            IsometricPlugin,
            SpriteGenPlugin,
            CameraPlugin,
            InputPlugin,
            PausePlugin,
            PlayerPlugin,
            CombatPlugin,
            EnemiesPlugin,
            VfxPlugin,
            UiPlugin,
            HubPlugin,
            RunPlugin,
            BossPlugin,
            LootPlugin,
        ))
        .add_plugins(PatchNotesPlugin)
        // Boot -> Menu transition (auto-advance for now).
        .add_systems(OnEnter(AppState::Boot), boot_to_menu)
        // Menu -> Hub transition.
        .add_systems(Update, menu_advance.run_if(in_state(AppState::Menu)))
        // Hub -> Run transition (guarded: don't start run while dialogue/cabinet is open).
        .add_systems(Update, hub_advance.run_if(in_state(AppState::Hub)))
        // Capture run stats before cleanup when leaving Run state.
        .add_systems(OnExit(AppState::Run), capture_run_summary)
        // Results -> Hub transition.
        .add_systems(Update, results_advance.run_if(in_state(AppState::Results)))
        .run();
}

/// Auto-advance from Boot straight to Run for testing.
fn boot_to_menu(mut next_state: ResMut<NextState<AppState>>) {
    // Skip menu and hub -- go straight to combat for testing.
    next_state.set(AppState::Run);
}

/// Press Enter in the menu to go to Hub.
fn menu_advance(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::Hub);
    }
}

/// Press Enter in the hub to start a run.
/// Guarded: don't fire while a dialogue or filing cabinet UI is open.
fn hub_advance(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    dialogue_res: Option<Res<plugins::hub::ActiveDialogue>>,
    cabinet_query: Query<Entity, With<plugins::hub::FilingCabinetUI>>,
) {
    // Block Enter while interacting with NPCs or the cabinet.
    if dialogue_res.is_some() || !cabinet_query.is_empty() {
        return;
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::Run);
    }
}

/// Capture run stats into a RunSummary before RunStateRes is cleaned up.
fn capture_run_summary(
    mut commands: Commands,
    run_state: Option<Res<plugins::run::RunStateRes>>,
) {
    let summary = if let Some(ref run) = run_state {
        RunSummary {
            run_complete: run.run_complete,
            rooms_cleared: run.rooms_cleared,
            enemies_killed: run.enemies_killed,
            deductions_earned: run.deductions_earned,
        }
    } else {
        RunSummary::default()
    };
    commands.insert_resource(summary);
}

/// Press Enter in results to go back to Hub.
fn results_advance(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::Hub);
    }
}
