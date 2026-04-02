use bevy::prelude::*;
use pot_shared::types::Direction;

/// Marker component for the player entity.
#[derive(Component, Default)]
pub struct Player;

/// Current and maximum health.
#[derive(Component, Clone, Debug)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 300.0,
            max: 300.0,
        }
    }
}

impl Health {
    pub fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }
}

/// Current and maximum mana.
#[derive(Component, Clone, Debug)]
pub struct Mana {
    pub current: f32,
    pub max: f32,
}

impl Default for Mana {
    fn default() -> Self {
        Self {
            current: 50.0,
            max: 50.0,
        }
    }
}

impl Mana {
    pub fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

/// Movement speed in world units per second.
#[derive(Component, Clone, Debug)]
pub struct MovementSpeed(pub f32);

impl Default for MovementSpeed {
    fn default() -> Self {
        Self(200.0)
    }
}

/// Which direction the entity is facing (for sprite rendering).
#[derive(Component, Clone, Debug)]
pub struct Facing(pub Direction);

impl Default for Facing {
    fn default() -> Self {
        Self(Direction::S)
    }
}

/// Current dodge roll state.
#[derive(Component, Clone, Debug)]
pub struct DodgeState {
    /// Whether the player is currently in a dodge roll.
    pub active: bool,
    /// Current frame within the dodge (0..=8).
    /// Frames 0-5: active i-frames. Frames 6-8: recovery. Cancel at frame 7.
    pub frame: u32,
    /// Direction of the dodge in world space.
    pub direction: Vec2,
    /// Speed multiplier during dodge.
    pub speed: f32,
}

impl Default for DodgeState {
    fn default() -> Self {
        Self {
            active: false,
            frame: 0,
            direction: Vec2::ZERO,
            speed: 400.0,
        }
    }
}

impl DodgeState {
    /// Active i-frame window: frames 0..6.
    pub const ACTIVE_FRAMES: u32 = 6;
    /// Recovery frames: frames 6..9.
    pub const RECOVERY_FRAMES: u32 = 3;
    /// Total frames for a dodge roll.
    pub const TOTAL_FRAMES: u32 = Self::ACTIVE_FRAMES + Self::RECOVERY_FRAMES;
    /// Frame at which dodge can be canceled into another action.
    pub const CANCEL_FRAME: u32 = 7;

    pub fn is_in_iframes(&self) -> bool {
        self.active && self.frame < Self::ACTIVE_FRAMES
    }

    pub fn can_cancel(&self) -> bool {
        self.active && self.frame >= Self::CANCEL_FRAME
    }

    pub fn is_finished(&self) -> bool {
        !self.active || self.frame >= Self::TOTAL_FRAMES
    }
}

/// Marks an entity as temporarily invulnerable (e.g. during dodge i-frames).
#[derive(Component, Clone, Debug, Default)]
pub struct Invulnerable {
    /// Remaining frames of invulnerability.
    pub frames_remaining: u32,
}

/// World-space velocity applied each frame.
#[derive(Component, Clone, Debug, Default)]
pub struct Velocity(pub Vec2);

/// Tracks animation frame timing for sprite sheet animation.
#[derive(Component, Clone, Debug)]
pub struct AnimationTimer {
    /// Counts game frames since last animation frame advance.
    pub frame_counter: u32,
    /// How many game frames per animation frame.
    pub frames_per_step: u32,
    /// Current animation column (0-7).
    pub current_column: u32,
}

impl Default for AnimationTimer {
    fn default() -> Self {
        Self {
            frame_counter: 0,
            frames_per_step: 8,
            current_column: 0,
        }
    }
}
