use bevy::prelude::*;
use bevy::post_process::bloom::Bloom;

use crate::content::CombatFeelConfig;
use crate::components::player::Player;
use crate::plugins::hub::HubPlayer;
use crate::plugins::input::GameInput;
use crate::plugins::vfx::HitstopState;
use crate::rendering::isometric::WorldPosition;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShakeQueue>()
            .init_resource::<CameraTarget>()
            .init_resource::<CameraZoomPulse>()
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (
                follow_player_system,
                camera_shake_system,
                camera_zoom_pulse_system,
            ).chain());
    }
}

/// Marker for the main game camera.
#[derive(Component)]
pub struct MainCamera;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::default(),
        Bloom {
            intensity: 0.08,
            ..Bloom::OLD_SCHOOL
        },
    ));
}

/// Where the camera wants to be (world position, projected to screen).
#[derive(Resource, Default)]
pub struct CameraTarget {
    pub position: Vec2,
    /// Lerp speed (0..1, higher = snappier).
    pub smoothing: f32,
}

impl CameraTarget {
    fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            smoothing: 0.1,
        }
    }
}

/// Follow the player entity smoothly (works for both combat Player and HubPlayer).
fn follow_player_system(
    game_input: Res<GameInput>,
    feel: Res<CombatFeelConfig>,
    hitstop: Res<HitstopState>,
    player_query: Query<&WorldPosition, With<Player>>,
    hub_player_query: Query<&WorldPosition, (With<HubPlayer>, Without<Player>)>,
    mut camera_target: ResMut<CameraTarget>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    if hitstop.is_active() {
        return;
    }

    let world_pos = if let Ok(pos) = player_query.single() {
        pos
    } else if let Ok(pos) = hub_player_query.single() {
        pos
    } else {
        return;
    };

    let screen_pos = crate::rendering::isometric::world_to_screen(world_pos.x, world_pos.y);

    let cursor_delta = game_input.mouse_screen_pos - screen_pos;
    let cursor_lead = if cursor_delta.length_squared() > 4.0 {
        cursor_delta.normalize() * feel.camera_cursor_lead_px
    } else {
        Vec2::ZERO
    };

    let move_lead = if game_input.move_direction.length_squared() > 0.0 {
        game_input.move_direction.normalize() * feel.camera_movement_lead_px
    } else {
        Vec2::ZERO
    };

    let desired_target = screen_pos + cursor_lead + move_lead;
    let lookahead_blend = feel.camera_lookahead_responsiveness.clamp(0.01, 1.0);
    camera_target.position = camera_target.position.lerp(desired_target, lookahead_blend);
    camera_target.smoothing = feel.camera_follow_smoothing;

    let Ok(mut cam_transform) = camera_query.single_mut() else {
        return;
    };

    let current = cam_transform.translation.xy();
    let target = camera_target.position;
    let smoothing = camera_target.smoothing.clamp(0.01, 1.0);
    let new_pos = current.lerp(target, smoothing);
    cam_transform.translation.x = new_pos.x;
    cam_transform.translation.y = new_pos.y;
}

// --- Screen Shake ---

/// A single shake entry in the queue.
#[derive(Clone, Debug)]
pub struct ShakeEntry {
    /// Direction of the shake (normalized). Zero = random.
    pub direction: Vec2,
    /// Maximum pixel offset.
    pub intensity: f32,
    /// Total duration in frames.
    pub total_frames: u32,
    /// Elapsed frames.
    pub elapsed_frames: u32,
}

impl ShakeEntry {
    /// Returns the current offset based on exponential decay.
    pub fn current_offset(&self) -> Vec2 {
        if self.total_frames == 0 {
            return Vec2::ZERO;
        }
        let t = self.elapsed_frames as f32 / self.total_frames as f32;
        // Exponential decay
        let decay = (-3.0 * t).exp();
        let magnitude = self.intensity * decay;

        if self.direction == Vec2::ZERO {
            // Alternating directions for random shake
            let sign = if self.elapsed_frames % 2 == 0 { 1.0 } else { -1.0 };
            Vec2::new(magnitude * sign, magnitude * sign * 0.5)
        } else {
            // Directional shake with alternating sign
            let sign = if self.elapsed_frames % 2 == 0 { 1.0 } else { -1.0 };
            self.direction * magnitude * sign
        }
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed_frames >= self.total_frames
    }
}

/// Queue of active screen shakes. Concurrent shakes are summed with diminishing returns.
#[derive(Resource, Default)]
pub struct ShakeQueue {
    pub entries: Vec<ShakeEntry>,
    /// Maximum total offset (cap to prevent absurd shaking).
    pub max_offset: f32,
}

impl ShakeQueue {
    pub fn push(&mut self, direction: Vec2, intensity: f32, duration_frames: u32) {
        self.entries.push(ShakeEntry {
            direction,
            intensity,
            total_frames: duration_frames,
            elapsed_frames: 0,
        });
    }

    /// Compute the combined shake offset for this frame.
    pub fn combined_offset(&self) -> Vec2 {
        let mut total = Vec2::ZERO;
        for entry in &self.entries {
            total += entry.current_offset();
        }
        // Cap at 12px as per spec
        let max = if self.max_offset > 0.0 { self.max_offset } else { 12.0 };
        let len = total.length();
        if len > max {
            total * (max / len)
        } else {
            total
        }
    }
}

/// Apply screen shake offset to the camera each frame.
fn camera_shake_system(
    feel: Res<CombatFeelConfig>,
    mut shake_queue: ResMut<ShakeQueue>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    shake_queue.max_offset = feel.shake_max_total_intensity;
    let offset = shake_queue.combined_offset();

    // Advance all shake entries and remove finished ones.
    for entry in &mut shake_queue.entries {
        entry.elapsed_frames += 1;
    }
    shake_queue.entries.retain(|e| !e.is_finished());

    // Apply offset to camera.
    let Ok(mut cam_transform) = camera_query.single_mut() else {
        return;
    };
    cam_transform.translation.x += offset.x;
    cam_transform.translation.y += offset.y;
}

// --- Camera Zoom Pulse ---

/// Resource for temporary camera zoom effects.
#[derive(Resource)]
pub struct CameraZoomPulse {
    /// Target zoom factor (1.0 = normal, 1.05 = 5% zoom in).
    pub target_scale: f32,
    /// Current interpolated scale.
    pub current_scale: f32,
    /// Frames remaining for the pulse.
    pub frames_remaining: u32,
    /// How quickly to return to 1.0.
    pub return_speed: f32,
    /// Base orthographic projection scale (lower = more zoomed in).
    pub base_ortho_scale: f32,
}

impl Default for CameraZoomPulse {
    fn default() -> Self {
        Self {
            target_scale: 1.0,
            current_scale: 1.0,
            frames_remaining: 0,
            return_speed: 0.15,
            base_ortho_scale: 0.85,
        }
    }
}

impl CameraZoomPulse {
    pub fn pulse(&mut self, zoom_factor: f32, duration_frames: u32) {
        self.target_scale = zoom_factor;
        self.current_scale = zoom_factor;
        self.frames_remaining = duration_frames;
        self.return_speed = 0.15;
    }
}

fn camera_zoom_pulse_system(
    mut zoom: ResMut<CameraZoomPulse>,
    mut camera_query: Query<&mut Projection, With<MainCamera>>,
) {
    if zoom.frames_remaining > 0 {
        zoom.frames_remaining -= 1;
        if zoom.frames_remaining == 0 {
            zoom.target_scale = 1.0;
        }
    }

    // Lerp current scale toward target.
    zoom.current_scale += (zoom.target_scale - zoom.current_scale) * zoom.return_speed;

    let Ok(mut proj) = camera_query.single_mut() else {
        return;
    };
    if let Projection::Orthographic(ref mut ortho) = *proj {
        ortho.scale = zoom.base_ortho_scale / zoom.current_scale.max(0.01);
    }
}
