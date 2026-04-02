use bevy::prelude::*;
use pot_shared::types::DamageType;

/// Damage value component (attached to hitbox entities).
#[derive(Component, Clone, Debug)]
pub struct Damage {
    pub amount: f32,
    pub damage_type: DamageType,
    pub knockback_force: f32,
    /// Direction of knockback (normalized).
    pub knockback_dir: Vec2,
    pub is_critical: bool,
}

/// Marker for projectile entities.
#[derive(Component, Clone, Debug)]
pub struct Projectile {
    pub speed: f32,
    pub direction: Vec2,
    pub lifetime_frames: u32,
    pub elapsed_frames: u32,
    pub pierce_remaining: u32,
    pub radius: f32,
}

/// Area-of-effect zone marker.
#[derive(Component, Clone, Debug)]
pub struct AoeZone {
    pub radius: f32,
    pub damage_per_tick: f32,
    pub damage_type: DamageType,
    pub tick_interval_frames: u32,
    pub frames_since_tick: u32,
    pub lifetime_frames: u32,
    pub elapsed_frames: u32,
}

/// Active knockback being applied to an entity.
#[derive(Component, Clone, Debug)]
pub struct Knockback {
    pub direction: Vec2,
    pub initial_force: f32,
    pub elapsed_frames: u32,
    pub total_frames: u32,
}

impl Knockback {
    /// Ease-out curve: fast initial push, slow deceleration.
    pub fn current_force(&self) -> f32 {
        if self.total_frames == 0 {
            return 0.0;
        }
        let t = self.elapsed_frames as f32 / self.total_frames as f32;
        // Ease-out: 1 - t^2
        self.initial_force * (1.0 - t * t)
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed_frames >= self.total_frames
    }
}

/// Cooldown tracker for abilities. Indexed by slot (0..5).
#[derive(Component, Clone, Debug)]
pub struct Cooldowns {
    /// Remaining cooldown frames per ability slot.
    pub slots: [u32; 6],
    /// Max cooldown frames per ability slot.
    pub max_slots: [u32; 6],
}

impl Default for Cooldowns {
    fn default() -> Self {
        Self {
            slots: [0; 6],
            max_slots: [60; 6],
        }
    }
}

impl Cooldowns {
    pub fn is_ready(&self, slot: usize) -> bool {
        slot < 6 && self.slots[slot] == 0
    }

    pub fn trigger(&mut self, slot: usize) {
        if slot < 6 {
            self.slots[slot] = self.max_slots[slot];
        }
    }

    pub fn tick(&mut self) {
        for cd in &mut self.slots {
            *cd = cd.saturating_sub(1);
        }
    }

    pub fn fraction(&self, slot: usize) -> f32 {
        if slot >= 6 || self.max_slots[slot] == 0 {
            return 1.0;
        }
        1.0 - (self.slots[slot] as f32 / self.max_slots[slot] as f32)
    }
}

/// Current ability execution state for an entity.
#[derive(Component, Clone, Debug)]
pub struct AbilityState {
    pub phase: AnimationPhase,
    pub current_slot: Option<usize>,
    pub frame_in_phase: u32,
    pub anticipation_frames: u32,
    pub active_frames: u32,
    pub recovery_frames: u32,
    pub cancel_frame: u32,
}

impl Default for AbilityState {
    fn default() -> Self {
        Self {
            phase: AnimationPhase::Idle,
            current_slot: None,
            frame_in_phase: 0,
            anticipation_frames: 2,
            active_frames: 3,
            recovery_frames: 4,
            cancel_frame: 6,
        }
    }
}

impl AbilityState {
    pub fn is_idle(&self) -> bool {
        matches!(self.phase, AnimationPhase::Idle)
    }

    pub fn can_cancel(&self) -> bool {
        match self.phase {
            AnimationPhase::Recovery => {
                let total_before_recovery = self.anticipation_frames + self.active_frames;
                let absolute_frame = total_before_recovery + self.frame_in_phase;
                absolute_frame >= self.cancel_frame
            }
            AnimationPhase::Idle => true,
            _ => false,
        }
    }

    pub fn start_ability(&mut self, slot: usize, anticipation: u32, active: u32, recovery: u32, cancel: u32) {
        self.phase = AnimationPhase::Anticipation;
        self.current_slot = Some(slot);
        self.frame_in_phase = 0;
        self.anticipation_frames = anticipation;
        self.active_frames = active;
        self.recovery_frames = recovery;
        self.cancel_frame = cancel;
    }

    pub fn reset(&mut self) {
        self.phase = AnimationPhase::Idle;
        self.current_slot = None;
        self.frame_in_phase = 0;
    }
}

/// Animation phase for the ability pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AnimationPhase {
    #[default]
    Idle,
    Anticipation,
    Active,
    Recovery,
}

/// Hitbox component for circle-circle collision.
#[derive(Component, Clone, Debug)]
pub struct Hitbox {
    pub radius: f32,
    pub faction: pot_shared::types::Faction,
    /// Entities already hit by this hitbox (to prevent multi-hit per swing).
    pub already_hit: Vec<Entity>,
}

/// Hurtbox component -- the damageable region.
#[derive(Component, Clone, Debug)]
pub struct Hurtbox {
    pub radius: f32,
    pub faction: pot_shared::types::Faction,
}

/// Currently selected ability slot (0..5).
#[derive(Component, Clone, Debug, Default)]
pub struct SelectedAbility(pub usize);

/// Shield state -- absorbs damage before HP.
#[derive(Component, Clone, Debug)]
pub struct ShieldState {
    /// Current shield HP remaining.
    pub amount: f32,
    /// Maximum shield HP (for refund calculation).
    pub max_amount: f32,
    /// Total damage absorbed over the shield's lifetime.
    pub absorbed: f32,
    /// Remaining frames before the shield expires.
    pub frames_remaining: u32,
}
