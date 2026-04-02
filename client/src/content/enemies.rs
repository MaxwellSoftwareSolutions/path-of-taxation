use bevy::prelude::*;
use serde::Deserialize;

use pot_shared::enemy_defs::EnemyDef;

/// RON wrapper matching the `Act1BasicEnemies(...)` format in act1_basic.ron.
#[derive(Debug, Deserialize)]
struct Act1BasicEnemies {
    enemies: Vec<EnemyDef>,
}

/// Resource holding all loaded enemy definitions.
#[derive(Resource, Debug, Clone, Default)]
pub struct EnemyDefs {
    pub enemies: Vec<EnemyDef>,
}

impl EnemyDefs {
    /// Look up an enemy by its key string.
    pub fn get_by_key(&self, key: &str) -> Option<&EnemyDef> {
        self.enemies.iter().find(|e| e.key == key)
    }
}

/// Load enemy definitions from RON files in `{base}/enemies/`.
pub fn load_enemy_defs(base: &std::path::Path) -> Result<EnemyDefs, String> {
    let enemies_dir = base.join("enemies");
    let mut all_enemies = Vec::new();

    // Load act1_basic.ron.
    let act1_path = enemies_dir.join("act1_basic.ron");
    let content = std::fs::read_to_string(&act1_path)
        .map_err(|e| format!("Failed to read {}: {e}", act1_path.display()))?;

    let act1: Act1BasicEnemies = ron::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {e}", act1_path.display()))?;

    all_enemies.extend(act1.enemies);

    Ok(EnemyDefs {
        enemies: all_enemies,
    })
}
