use bevy::prelude::*;

use crate::state::ScreenState;

pub struct WorldPlugin;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct HubMarker;

#[derive(Component)]
struct RunMarker;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(OnEnter(ScreenState::Hub), spawn_hub_shell)
            .add_systems(OnExit(ScreenState::Hub), cleanup_hub_shell);
        app.add_systems(OnEnter(ScreenState::Run), spawn_run_shell)
            .add_systems(OnExit(ScreenState::Run), cleanup_run_shell);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));
}

fn spawn_hub_shell(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.11, 0.12, 0.16), Vec2::new(1400.0, 760.0)),
        Transform::from_xyz(0.0, -120.0, 0.0),
        HubMarker,
        Name::new("HubAnchor"),
    ));
}

fn cleanup_hub_shell(mut commands: Commands, hubs: Query<Entity, With<HubMarker>>) {
    for entity in &hubs {
        commands.entity(entity).despawn();
    }
}

fn spawn_run_shell(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.1, 0.09, 0.08), Vec2::new(1400.0, 760.0)),
        Transform::from_xyz(0.0, 0.0, -1.0),
        RunMarker,
        Name::new("RunFloor"),
    ));

    for (x, y, w, h) in [
        (-680.0, 0.0, 24.0, 760.0),
        (680.0, 0.0, 24.0, 760.0),
        (0.0, 368.0, 1400.0, 24.0),
        (0.0, -368.0, 1400.0, 24.0),
    ] {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.22, 0.17, 0.14), Vec2::new(w, h)),
            Transform::from_xyz(x, y, 0.0),
            RunMarker,
        ));
    }
}

fn cleanup_run_shell(mut commands: Commands, runs: Query<Entity, With<RunMarker>>) {
    for entity in &runs {
        commands.entity(entity).despawn();
    }
}
