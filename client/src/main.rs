mod dorfromantik;

use bevy::prelude::*;

use dorfromantik::DorfromantikSandboxPlugin;

fn main() {
    let asset_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Path of Taxation".to_string(),
                        resolution: (1600, 1000).into(),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_path.to_string_lossy().into_owned(),
                    ..default()
                }),
        )
        .add_plugins(DorfromantikSandboxPlugin)
        .run();
}
