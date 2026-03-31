mod game;
mod combat;
mod player;
mod state;
mod ui;
mod world;

use bevy::prelude::*;
use game::PathOfTaxationPlugin;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.07, 0.07, 0.09)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Path of Taxation".into(),
                resolution: (1600, 900).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PathOfTaxationPlugin)
        .run();
}
