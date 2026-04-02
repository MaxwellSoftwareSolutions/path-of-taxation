use serde::{Deserialize, Serialize};

use crate::types::DamageType;

/// Definition of an ability loaded from RON content files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDef {
    pub key: String,
    pub name: String,
    pub description: String,
    pub damage_type: DamageType,
    pub ability_type: AbilityType,

    // --- Damage ---
    pub base_damage: i64,
    pub mana_cost: i64,
    pub cooldown_ms: u64,

    // --- Animation windows (frames at 60fps) ---
    pub anticipation_frames: u32,
    pub active_frames: u32,
    pub recovery_frames: u32,
    /// Frame at which recovery can be canceled into dodge/other ability
    pub cancel_frame: u32,

    // --- Projectile (if applicable) ---
    pub projectile_speed: Option<f32>,
    pub projectile_count: Option<u32>,
    pub projectile_spread_deg: Option<f32>,
    pub pierce_count: Option<u32>,
    pub projectile_lifetime_ms: Option<u64>,

    // --- AoE (if applicable) ---
    pub aoe_radius: Option<f32>,
    pub aoe_duration_ms: Option<u64>,
    pub aoe_tick_interval_ms: Option<u64>,

    // --- Movement (if applicable) ---
    pub teleport_range: Option<f32>,
    pub dash_speed: Option<f32>,
    pub dash_duration_ms: Option<u64>,

    // --- Buff/Shield (if applicable) ---
    pub shield_amount: Option<i64>,
    pub shield_duration_ms: Option<u64>,
    pub buff_key: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityType {
    Projectile,
    AoE,
    Channel,
    Teleport,
    Shield,
    Melee,
}

/// A build-defining modification to an ability (Daedalus Hammer equivalent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegislativeAmendment {
    pub key: String,
    pub name: String,
    pub description: String,
    /// Which ability this modifies, or "any" for universal
    pub target_ability: String,
    /// Stat modifications applied
    pub modifications: Vec<StatModification>,
    /// Behavioral change description (implemented in code)
    pub behavior_change: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatModification {
    pub stat: String,
    pub operation: ModOperation,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModOperation {
    Add,
    Multiply,
    Override,
}
