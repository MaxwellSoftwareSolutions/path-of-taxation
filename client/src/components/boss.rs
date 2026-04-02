use bevy::prelude::*;

/// Marker component for the boss entity.
#[derive(Component, Default)]
pub struct Boss;

/// Boss AI state tracking.
#[derive(Component, Clone, Debug)]
pub struct BossState {
    /// Current boss phase (0, 1, 2). Advances at 66% and 33% HP.
    pub current_phase: u32,
    /// Timer counting frames between attacks.
    pub attack_timer: u32,
    /// Which attack in the cycle (0=Stamp Slam, 1=Paper Barrage, 2=Audit Charge).
    pub attack_index: u32,
    /// Whether each phase transition has already fired.
    pub phase_transitioned: [bool; 3],
}

impl Default for BossState {
    fn default() -> Self {
        Self {
            current_phase: 0,
            attack_timer: 0,
            attack_index: 0,
            phase_transitioned: [true, false, false], // phase 0 starts transitioned
        }
    }
}

impl BossState {
    /// Attack interval in frames for the current phase.
    pub fn attack_interval(&self) -> u32 {
        match self.current_phase {
            0 => 90,
            1 => 70,
            _ => 50,
        }
    }
}
