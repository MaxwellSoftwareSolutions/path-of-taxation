mod abilities;
mod combat_feel;
mod enemies;
mod loading_tips;

pub use abilities::AbilityDefs;
pub use combat_feel::CombatFeelConfig;
pub use enemies::EnemyDefs;
pub use loading_tips::LoadingTipsDefs;

use bevy::prelude::*;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_all_content);
    }
}

/// Load all RON content files and insert them as resources during Startup.
fn load_all_content(mut commands: Commands) {
    let base = content_base_path();

    // Load ability definitions.
    match abilities::load_ability_defs(&base) {
        Ok(defs) => {
            info!("Loaded {} ability definitions", defs.abilities.len());
            commands.insert_resource(defs);
        }
        Err(e) => {
            error!("Failed to load ability definitions: {e}");
            commands.insert_resource(AbilityDefs::default());
        }
    }

    // Load enemy definitions.
    match enemies::load_enemy_defs(&base) {
        Ok(defs) => {
            info!("Loaded {} enemy definitions", defs.enemies.len());
            commands.insert_resource(defs);
        }
        Err(e) => {
            error!("Failed to load enemy definitions: {e}");
            commands.insert_resource(EnemyDefs::default());
        }
    }

    // Load combat feel config.
    match combat_feel::load_combat_feel(&base) {
        Ok(config) => {
            info!("Loaded combat feel config");
            commands.insert_resource(config);
        }
        Err(e) => {
            error!("Failed to load combat feel config: {e}");
            commands.insert_resource(CombatFeelConfig::default());
        }
    }

    // Load loading tips.
    match loading_tips::load_loading_tips(&base) {
        Ok(tips) => {
            info!("Loaded {} loading tips", tips.tips.len());
            commands.insert_resource(tips);
        }
        Err(e) => {
            error!("Failed to load loading tips: {e}");
            commands.insert_resource(LoadingTipsDefs::default());
        }
    }
}

/// Resolve the `content/` directory path relative to the workspace root.
/// The binary runs from `client/`, so `content/` is at `../content/`.
/// We also try the workspace root directly in case cwd differs.
fn content_base_path() -> std::path::PathBuf {
    // Try paths relative to the executable and cwd.
    let candidates = [
        std::path::PathBuf::from("content"),
        std::path::PathBuf::from("../content"),
        // Absolute fallback for development.
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content"),
    ];

    for path in &candidates {
        if path.exists() && path.is_dir() {
            return path.clone();
        }
    }

    // Default: relative to manifest dir (compile-time).
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../content")
}
