use bevy::prelude::*;
use serde::Deserialize;

use pot_shared::ability_defs::AbilityDef;

/// RON wrapper matching the `AbilitySet(...)` format in refund_witch.ron.
#[derive(Debug, Deserialize)]
struct AbilitySet {
    #[allow(dead_code)]
    character: String,
    abilities: Vec<AbilityDef>,
    dodge_roll: DodgeRollDef,
    base_stats: BaseStatsDef,
}

/// Dodge roll definition from the RON file (not used yet, but must be parsed).
#[derive(Debug, Deserialize, Clone)]
pub struct DodgeRollDef {
    pub anticipation_frames: u32,
    pub active_frames: u32,
    pub recovery_frames: u32,
    pub cancel_frame: u32,
    pub speed_multiplier: f32,
    pub cooldown_frames: Option<u32>,
}

/// Base stats definition from the RON file (not used yet, but must be parsed).
#[derive(Debug, Deserialize, Clone)]
pub struct BaseStatsDef {
    pub hp: i64,
    pub mana: i64,
    pub mana_regen_per_sec: f32,
    pub move_speed: f32,
    pub ability_slots: u32,
    pub form_slots: u32,
}

impl Default for DodgeRollDef {
    fn default() -> Self {
        Self {
            anticipation_frames: 0,
            active_frames: 5,
            recovery_frames: 6,
            cancel_frame: 7,
            speed_multiplier: 2.4,
            cooldown_frames: Some(18),
        }
    }
}

impl Default for BaseStatsDef {
    fn default() -> Self {
        Self {
            hp: 300,
            mana: 50,
            mana_regen_per_sec: 5.0,
            move_speed: 200.0,
            ability_slots: 6,
            form_slots: 1,
        }
    }
}

/// Resource holding all loaded ability definitions.
#[derive(Resource, Debug, Clone, Default)]
pub struct AbilityDefs {
    pub abilities: Vec<AbilityDef>,
    pub dodge_roll: DodgeRollDef,
    pub base_stats: BaseStatsDef,
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
        dodge_roll: ability_set.dodge_roll,
        base_stats: ability_set.base_stats,
    })
}
