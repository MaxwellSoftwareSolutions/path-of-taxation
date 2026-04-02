use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::window::PrimaryWindow;

use crate::app_state::AppState;
use crate::content::CombatFeelConfig;
use crate::plugins::camera::MainCamera;
use crate::rendering::isometric::screen_to_world;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .init_resource::<GameInput>()
            .init_resource::<InputBuffer>()
            .init_resource::<DebugOverlayVisible>()
            .init_resource::<FpsOverlayVisible>()
            .add_systems(Update, (
                gather_input_system,
                input_buffer_system,
            ).chain().run_if(in_state(AppState::Run)))
            .add_systems(Update, fps_overlay_system);
    }
}

/// Processed game input for the current frame.
#[derive(Resource, Default)]
pub struct GameInput {
    /// Movement direction in world space (WASD mapped to isometric).
    pub move_direction: Vec2,
    /// Whether dodge was pressed this frame.
    pub dodge_pressed: bool,
    /// Mouse position in projected screen space.
    pub mouse_screen_pos: Vec2,
    /// Mouse position in gameplay world space.
    pub mouse_world_pos: Vec2,
    /// Whether primary attack (left click) was pressed.
    pub attack_pressed: bool,
    /// Ability slot selected (0-5 for keys 1-6), None if no change.
    pub ability_slot: Option<usize>,
    /// Pause/menu toggled.
    pub pause_pressed: bool,
    /// Debug overlay toggled.
    pub debug_toggle: bool,
    /// Gamepad aim direction (right stick), overrides mouse for facing.
    pub aim_direction: Option<Vec2>,
}

/// Input buffer: stores the last N frames of input for buffered actions.
#[derive(Resource, Default)]
pub struct InputBuffer {
    pub attack_frames_remaining: u32,
    pub dodge_frames_remaining: u32,
}

impl InputBuffer {
    /// Returns true if attack was pressed within the buffer window.
    pub fn attack_buffered(&self) -> bool {
        self.attack_frames_remaining > 0
    }

    /// Returns true if dodge was pressed within the buffer window.
    pub fn dodge_buffered(&self) -> bool {
        self.dodge_frames_remaining > 0
    }

    /// Consume the buffered attack (clear all entries).
    pub fn consume_attack(&mut self) {
        self.attack_frames_remaining = 0;
    }

    /// Consume the buffered dodge.
    pub fn consume_dodge(&mut self) {
        self.dodge_frames_remaining = 0;
    }
}

/// Whether the debug overlay is currently visible.
#[derive(Resource, Default)]
pub struct DebugOverlayVisible(pub bool);

/// Whether the FPS overlay is currently visible (toggled by F2).
#[derive(Resource, Default)]
pub struct FpsOverlayVisible(pub bool);

/// Marker for the FPS overlay text node.
#[derive(Component)]
pub struct FpsOverlayText;

/// Deadzone threshold for gamepad stick input.
const STICK_DEADZONE: f32 = 0.15;

/// Read keyboard/mouse/gamepad input and produce a GameInput for this frame.
fn gather_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    gamepads: Query<&Gamepad>,
    mut game_input: ResMut<GameInput>,
    mut debug_vis: ResMut<DebugOverlayVisible>,
) {
    // --- Movement (WASD mapped to isometric axes) ---
    let mut raw_dir = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        raw_dir.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        raw_dir.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        raw_dir.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        raw_dir.x += 1.0;
    }

    // --- Gamepad: left stick -> movement ---
    for gamepad in &gamepads {
        let stick = gamepad.left_stick();
        if stick.length() > STICK_DEADZONE {
            raw_dir += stick;
        }
    }

    // Rotate input to align with isometric axes.
    // In our 2:1 dimetric projection: screen_x = world_x - world_y, screen_y = (world_x + world_y) / 2
    // So "screen up" (W) = +world_x, +world_y and "screen right" (D) = +world_x, -world_y.
    // This is a 45-degree rotation of the input vector.
    game_input.move_direction = if raw_dir != Vec2::ZERO {
        let normalized = raw_dir.normalize();
        Vec2::new(
            normalized.x + normalized.y,
            normalized.y - normalized.x,
        ).normalize()
    } else {
        Vec2::ZERO
    };

    // --- Dodge (Space or Shift or gamepad South/A/Cross) ---
    game_input.dodge_pressed = keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::ShiftLeft)
        || keyboard.just_pressed(KeyCode::ShiftRight);

    // --- Attack (Left click or gamepad West/X/Square) ---
    game_input.attack_pressed = mouse_buttons.just_pressed(MouseButton::Left);

    // --- Ability selection (1-6) ---
    game_input.ability_slot = None;
    if keyboard.just_pressed(KeyCode::Digit1) {
        game_input.ability_slot = Some(0);
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        game_input.ability_slot = Some(1);
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        game_input.ability_slot = Some(2);
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        game_input.ability_slot = Some(3);
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        game_input.ability_slot = Some(4);
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        game_input.ability_slot = Some(5);
    }

    // --- Pause (Esc or gamepad Start) ---
    game_input.pause_pressed = keyboard.just_pressed(KeyCode::Escape);

    // --- Debug toggle (F1) ---
    game_input.debug_toggle = keyboard.just_pressed(KeyCode::F1);
    if game_input.debug_toggle {
        debug_vis.0 = !debug_vis.0;
    }

    // --- Gamepad aim direction (right stick) ---
    game_input.aim_direction = None;

    // --- Gamepad button inputs ---
    for gamepad in &gamepads {
        // Dodge: South button (A/Cross).
        if gamepad.just_pressed(GamepadButton::South) {
            game_input.dodge_pressed = true;
        }
        // Attack: West button (X/Square).
        if gamepad.just_pressed(GamepadButton::West) {
            game_input.attack_pressed = true;
        }
        // Pause: Start button.
        if gamepad.just_pressed(GamepadButton::Start) {
            game_input.pause_pressed = true;
        }
        // Ability cycle: bumpers/triggers cycle through slots.
        if game_input.ability_slot.is_none() {
            if gamepad.just_pressed(GamepadButton::RightTrigger)
                || gamepad.just_pressed(GamepadButton::RightTrigger2)
            {
                // Cycle forward (handled by player system via slot increment).
                game_input.ability_slot = Some(usize::MAX); // sentinel: cycle forward
            } else if gamepad.just_pressed(GamepadButton::LeftTrigger)
                || gamepad.just_pressed(GamepadButton::LeftTrigger2)
            {
                game_input.ability_slot = Some(usize::MAX - 1); // sentinel: cycle backward
            }
        }
        // Right stick -> aim direction.
        let right = gamepad.right_stick();
        if right.length() > STICK_DEADZONE {
            game_input.aim_direction = Some(right.normalize());
        }
    }

    // --- Mouse world position ---
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_query.single() else {
        return;
    };

    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(screen_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
            game_input.mouse_screen_pos = screen_pos;
            let world_pos = screen_to_world(screen_pos.x, screen_pos.y);
            game_input.mouse_world_pos = world_pos;
        }
    }
}

/// Write current frame's input into the ring buffer.
fn input_buffer_system(
    game_input: Res<GameInput>,
    feel: Res<CombatFeelConfig>,
    mut buffer: ResMut<InputBuffer>,
) {
    let window = feel.input_buffer_frames.max(1);

    buffer.attack_frames_remaining = if game_input.attack_pressed {
        window
    } else {
        buffer.attack_frames_remaining.saturating_sub(1)
    };

    buffer.dodge_frames_remaining = if game_input.dodge_pressed {
        window
    } else {
        buffer.dodge_frames_remaining.saturating_sub(1)
    };
}

/// FPS debug overlay: toggle with F2, shows FPS and entity count at top-right.
fn fps_overlay_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut fps_vis: ResMut<FpsOverlayVisible>,
    diagnostics: Res<DiagnosticsStore>,
    existing_text: Query<Entity, With<FpsOverlayText>>,
    all_entities: Query<Entity>,
    mut commands: Commands,
) {
    // Toggle on F2 (works in any app state).
    if keyboard.just_pressed(KeyCode::F2) {
        fps_vis.0 = !fps_vis.0;
    }

    // Always clean up old text.
    for entity in &existing_text {
        commands.entity(entity).despawn();
    }

    if !fps_vis.0 {
        return;
    }

    // Read FPS from diagnostics.
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let entity_count = all_entities.iter().count();

    commands.spawn((
        FpsOverlayText,
        Text::new(format!("FPS: {:.0}  Entities: {}", fps, entity_count)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(6.0),
            right: Val::Px(140.0),
            ..default()
        },
        GlobalZIndex(200),
    ));
}
