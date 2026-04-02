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

/// High-precision aim direction used by attacks and abilities.
#[derive(Component, Clone, Debug)]
pub struct AimVector(pub Vec2);

impl Default for AimVector {
    fn default() -> Self {
        Self(Vec2::new(0.0, -1.0))
    }
}

/// Current target point in world space.
#[derive(Component, Clone, Debug, Default)]
pub struct AimTarget(pub Vec2);

/// Current dodge roll state.
#[derive(Component, Clone, Debug)]
pub struct DodgeState {
    /// Whether the player is currently in a dodge roll.
    pub active: bool,
    /// Current frame within the dodge state.
    pub frame: u32,
    /// Direction of the dodge in world space.
    pub direction: Vec2,
    /// World-space speed during dodge.
    pub speed: f32,
    /// Frames where dodge movement is active.
    pub active_frames: u32,
    /// Frames after the burst where the player is still committed.
    pub recovery_frames: u32,
    /// Frame at which other actions may cancel out of the roll.
    pub cancel_frame: u32,
    /// Frames of temporary invulnerability.
    pub iframe_frames: u32,
}

#[derive(Component, Clone, Debug)]
pub struct DodgeCooldown {
    pub frames_remaining: u32,
    pub max_frames: u32,
}

impl Default for DodgeCooldown {
    fn default() -> Self {
        Self {
            frames_remaining: 0,
            max_frames: 18,
        }
    }
}

impl Default for DodgeState {
    fn default() -> Self {
        Self {
            active: false,
            frame: 0,
            direction: Vec2::ZERO,
            speed: 480.0,
            active_frames: 5,
            recovery_frames: 6,
            cancel_frame: 7,
            iframe_frames: 4,
        }
    }
}

impl DodgeState {
    pub fn total_frames(&self) -> u32 {
        self.active_frames + self.recovery_frames
    }

    pub fn is_in_iframes(&self) -> bool {
        self.active && self.frame < self.iframe_frames
    }

    pub fn can_cancel(&self) -> bool {
        self.active && self.frame >= self.cancel_frame
    }

    pub fn is_finished(&self) -> bool {
        !self.active || self.frame >= self.total_frames()
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
