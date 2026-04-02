use serde::{Deserialize, Serialize};

use crate::types::DamageType;

/// Definition of an enemy archetype loaded from RON content files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnemyDef {
    pub key: String,
    pub name: String,
    pub description: String,

    // --- Stats ---
    pub base_hp: i64,
    pub base_damage: i64,
    pub damage_type: DamageType,
    pub move_speed: f32,

    // --- AI ---
    pub behavior: EnemyBehavior,
    pub aggro_range: f32,
    pub attack_range: f32,
    pub attack_cooldown_ms: u64,
    /// Minimum telegraph time before attack lands (for player reaction)
    pub windup_ms: u64,

    // --- Visual ---
    pub sprite_key: String,
    pub sprite_size_px: [u32; 2],
    /// Does this enemy have a distinct silhouette? (design validation)
    pub is_silhouette_distinct: bool,

    // --- Drops ---
    pub deduction_drop_min: u64,
    pub deduction_drop_max: u64,
    pub item_drop_chance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyBehavior {
    /// Shambles toward player, melee swipe
    Shamble,
    /// Fast, low HP, swarm in groups
    Swarm,
    /// Chases player, telegraphed strike
    Chase,
    /// Ranged projectile attacker
    Ranged,
    /// Stationary area denial
    Stationary,
    /// Ranged, repositions to keep distance
    Kiter,
    /// Casts debuffs on player
    Debuffer,
}

/// Elite prefix that modifies an enemy's base stats and behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElitePrefix {
    pub key: String,
    pub name: String,
    pub description: String,
    pub hp_multiplier: f32,
    pub damage_multiplier: f32,
    /// Extra behavior (e.g., "dot_aura", "damage_reflect", "debuff_on_hit")
    pub special_behavior: Option<String>,
}

/// Boss definition with multi-phase support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossDef {
    pub key: String,
    pub name: String,
    pub title: String,
    pub description: String,
    pub base_hp: i64,
    pub phases: Vec<BossPhase>,
    pub sprite_key: String,
    pub sprite_size_px: [u32; 2],
    pub arena_hazards: Vec<String>,
    /// Voice lines during fight
    pub bark_lines: Vec<BossBark>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossPhase {
    pub name: String,
    /// HP percentage threshold to enter this phase (1.0 = full, 0.33 = 33%)
    pub hp_threshold: f32,
    pub attacks: Vec<String>,
    /// Optional description for phase transition
    pub transition_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossBark {
    pub trigger: String,
    pub line: String,
}
