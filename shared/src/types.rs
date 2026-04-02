use serde::{Deserialize, Serialize};

/// 2D position in world space.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Cardinal + intercardinal directions for sprite rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl Direction {
    pub fn from_angle(radians: f32) -> Self {
        let deg = radians.to_degrees().rem_euclid(360.0);
        match deg as u32 {
            0..=22 | 338..=360 => Direction::E,
            23..=67 => Direction::NE,
            68..=112 => Direction::N,
            113..=157 => Direction::NW,
            158..=202 => Direction::W,
            203..=247 => Direction::SW,
            248..=292 => Direction::S,
            _ => Direction::SE,
        }
    }
}

/// Damage types in the game -- themed as bureaucratic harm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    /// Physical damage from stamps, briefcases, etc.
    Penalty,
    /// Fire-like damage -- burning audits
    Audit,
    /// Cold-like damage -- frozen assets
    Freeze,
    /// Chaos-like damage -- bureaucratic confusion
    Bureaucracy,
    /// Lightning-like damage -- rapid-fire forms
    Expedited,
    /// Poison-like damage -- compound interest DoT
    Interest,
}

/// Rarity tiers for items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rarity {
    /// White -- basic
    Normal,
    /// Blue -- one affix
    Magic,
    /// Yellow -- multiple affixes
    Rare,
    /// Orange -- fixed unique affixes
    Unique,
    /// Red -- corrupted / audited
    ExcessivelyTaxed,
}

/// Factions for hit detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    Player,
    Enemy,
    Neutral,
}

/// Room types available in runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomType {
    Combat,
    EliteCombat,
    Treasure,
    Shop,
    Event,
    Challenge,
    Rest,
    IrsAudit,
}

/// Run state progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RunState {
    Active,
    Boss,
    Completed,
    Failed,
    Abandoned,
    Paused,
}

/// AI behavior states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AiState {
    Idle,
    Patrol,
    Chase,
    Windup,
    Attack,
    Recover,
    Flee,
    Staggered,
}

/// Enemy variant (prefix system).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnemyVariant {
    Normal,
    /// "Auditing" -- DoT aura
    Magic,
    /// "Taxing" -- damage boost + "Regulatory" -- debuff on hit
    Rare,
    /// Named unique enemy
    Unique,
}
