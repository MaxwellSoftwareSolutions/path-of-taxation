use bevy::prelude::*;

/// 2:1 dimetric isometric projection.
///
/// World space: X goes right, Y goes up.
/// Screen space: standard Bevy 2D (X right, Y up).
///
/// In a 2:1 dimetric projection:
///   screen_x = (world_x - world_y)
///   screen_y = (world_x + world_y) / 2

/// Z-layer constants for the rendering stack.
pub mod z_layers {
    pub const BG_FAR: f32 = -500.0;
    pub const TERRAIN_BASE: f32 = -350.0;
    pub const TERRAIN_DETAIL: f32 = -250.0;
    pub const GROUND_PROPS: f32 = -150.0;
    // Depth-sorted entities live in -80..+80 range (world_y * 0.1)
    pub const FOG: f32 = 200.0;
    pub const VIGNETTE: f32 = 300.0;
    pub const FOREGROUND: f32 = 500.0;
    pub const UI_WORLD: f32 = 90.0;
    pub const DEBUG: f32 = 200.0;
}

/// Convert world coordinates to screen (pixel) coordinates.
pub fn world_to_screen(world_x: f32, world_y: f32) -> Vec2 {
    Vec2::new(
        world_x - world_y,
        (world_x + world_y) / 2.0,
    )
}

/// Convert screen coordinates back to world coordinates.
pub fn screen_to_world(screen_x: f32, screen_y: f32) -> Vec2 {
    Vec2::new(
        screen_y + screen_x / 2.0,
        screen_y - screen_x / 2.0,
    )
}

/// Plugin that handles depth sorting for isometric rendering.
pub struct IsometricPlugin;

impl Plugin for IsometricPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, depth_sort_system);
    }
}

/// Depth sorting: scale world_y into the -80..+80 z-band so entities
/// sort correctly relative to each other without colliding with terrain layers.
fn depth_sort_system(
    mut query: Query<(&WorldPosition, &mut Transform)>,
) {
    for (world_pos, mut transform) in &mut query {
        let screen = world_to_screen(world_pos.x, world_pos.y);
        transform.translation.x = screen.x;
        transform.translation.y = screen.y;
        transform.translation.z = -world_pos.y * 0.1;
    }
}

/// World-space position component (separate from Transform which is screen-space).
#[derive(Component, Clone, Debug, Default)]
pub struct WorldPosition {
    pub x: f32,
    pub y: f32,
}

impl WorldPosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn as_vec2(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn distance_to(&self, other: &WorldPosition) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}
