use bevy::prelude::*;
use serde::Deserialize;

use pot_shared::ability_defs::AbilityDef;

/// RON wrapper matching the `AbilitySet(...)` format in refund_witch.ron.
#[derive(Debug, Deserialize)]
struct AbilitySet {
    #[allow(dead_code)]
    character: String,
    abilities: Vec<AbilityDef>,
    #[allow(dead_code)]
    dodge_roll: DodgeRollDef,
    #[allow(dead_code)]
    base_stats: BaseStatsDef,
}

/// Dodge roll definition from the RON file (not used yet, but must be parsed).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DodgeRollDef {
    anticipation_frames: u32,
    active_frames: u32,
    recovery_frames: u32,
    cancel_frame: u32,
    speed_multiplier: f32,
}

/// Base stats definition from the RON file (not used yet, but must be parsed).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BaseStatsDef {
    hp: i64,
    mana: i64,
    mana_regen_per_sec: f32,
    move_speed: f32,
    ability_slots: u32,
    form_slots: u32,
}

/// Resource holding all loaded ability definitions.
#[derive(Resource, Debug, Clone, Default)]
pub struct AbilityDefs {
    pub abilities: Vec<AbilityDef>,
}

impl AbilityDefs {
    /// Look up an ability by its slot index (0-based).
    pub fn get_by_slot(&self, slot: usize) -> Option<&AbilityDef> {
        self.abilities.get(slot)
    }

    /// Look up an ability by its key string.
    #[allow(dead_code)]
    pub fn get_by_key(&self, key: &str) -> Option<&AbilityDef> {
        self.abilities.iter().find(|a| a.key == key)
    }
}

/// Load ability definitions from all RON files in `{base}/abilities/`.
pub fn load_ability_defs(base: &std::path::Path) -> Result<AbilityDefs, String> {
    let abilities_dir = base.join("abilities");
    let mut all_abilities = Vec::new();

    // Load refund_witch.ron (primary ability set for now).
    let refund_witch_path = abilities_dir.join("refund_witch.ron");
    let content = std::fs::read_to_string(&refund_witch_path)
        .map_err(|e| format!("Failed to read {}: {e}", refund_witch_path.display()))?;

    let ability_set: AbilitySet = ron::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {e}", refund_witch_path.display()))?;

    all_abilities.extend(ability_set.abilities);

    Ok(AbilityDefs {
        abilities: all_abilities,
    })
}
