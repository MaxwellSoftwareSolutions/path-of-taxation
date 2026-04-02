use bevy::prelude::*;
use pot_shared::enemy_defs::EnemyBehavior;
use pot_shared::types::{AiState, EnemyVariant};

/// Marker component for enemy entities.
#[derive(Component, Default)]
pub struct Enemy;

/// What type/archetype this enemy is.
#[derive(Component, Clone, Debug)]
pub struct EnemyType(pub String);

/// Which AI behavior pattern this enemy uses (set from EnemyDef at spawn).
#[derive(Component, Clone, Debug)]
pub struct BehaviorType(pub EnemyBehavior);

impl Default for BehaviorType {
    fn default() -> Self {
        Self(EnemyBehavior::Chase)
    }
}

/// Base damage for this enemy's attacks (from EnemyDef).
#[derive(Component, Clone, Debug)]
pub struct EnemyDamage(pub f32);

impl Default for EnemyDamage {
    fn default() -> Self {
        Self(8.0)
    }
}

/// Base movement speed for this enemy (from EnemyDef).
#[derive(Component, Clone, Debug)]
pub struct MoveSpeed(pub f32);

impl Default for MoveSpeed {
    fn default() -> Self {
        Self(100.0)
    }
}

/// Current AI behavior state.
#[derive(Component, Clone, Debug)]
pub struct AiBehavior {
    pub state: AiState,
    pub state_timer_frames: u32,
}

impl Default for AiBehavior {
    fn default() -> Self {
        Self {
            state: AiState::Idle,
            state_timer_frames: 0,
        }
    }
}

/// Aggro detection range.
#[derive(Component, Clone, Debug)]
pub struct AggroRange(pub f32);

impl Default for AggroRange {
    fn default() -> Self {
        Self(300.0)
    }
}

/// Enemy attack cooldown.
#[derive(Component, Clone, Debug)]
pub struct AttackCooldown {
    pub current_frames: u32,
    pub max_frames: u32,
}

impl Default for AttackCooldown {
    fn default() -> Self {
        Self {
            current_frames: 0,
            max_frames: 60,
        }
    }
}

impl AttackCooldown {
    pub fn is_ready(&self) -> bool {
        self.current_frames == 0
    }

    pub fn trigger(&mut self) {
        self.current_frames = self.max_frames;
    }

    pub fn tick(&mut self) {
        self.current_frames = self.current_frames.saturating_sub(1);
    }
}

/// Enemy is staggered (hit-stunned).
#[derive(Component, Clone, Debug)]
pub struct Staggered {
    pub frames_remaining: u32,
}

impl Default for Staggered {
    fn default() -> Self {
        Self { frames_remaining: 3 }
    }
}

/// Variant modifier (normal, magic, rare, unique).
#[derive(Component, Clone, Debug)]
pub struct Variant(pub EnemyVariant);

impl Default for Variant {
    fn default() -> Self {
        Self(EnemyVariant::Normal)
    }
}

/// Whether this enemy is actively attacking (part of the max-8 crowd).
#[derive(Component, Clone, Debug, Default)]
pub struct ActiveAttacker;

/// Attack range for melee or ranged attacks.
#[derive(Component, Clone, Debug)]
pub struct AttackRange(pub f32);

impl Default for AttackRange {
    fn default() -> Self {
        Self(50.0)
    }
}

/// Marks an enemy for death dissolution.
#[derive(Component, Clone, Debug)]
pub struct Dying {
    pub frames_remaining: u32,
}

impl Default for Dying {
    fn default() -> Self {
        Self { frames_remaining: 20 }
    }
}

/// Marker component for the attack telegraph ground indicator.
/// Attached to the telegraph entity, tracks which enemy spawned it.
#[derive(Component, Clone, Debug)]
pub struct AttackTelegraph {
    pub owner: Entity,
}

/// Tracks whether the enemy has already spawned its hitbox this attack cycle.
/// Prevents spawning multiple hitboxes during the multi-frame Attack state.
#[derive(Component, Clone, Debug, Default)]
pub struct AttackFired;
