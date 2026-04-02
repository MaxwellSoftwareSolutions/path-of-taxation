use bevy::prelude::*;

use crate::app_state::AppState;
use crate::plugins::camera::{CameraZoomPulse, MainCamera};
use crate::plugins::run::ArenaEntity;

pub struct VfxPlugin;

impl Plugin for VfxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HitstopState>()
            .init_resource::<TimeScaleState>()
            .init_resource::<AmbientParticleCount>()
            .add_message::<HitstopMsg>()
            .add_message::<HitFlashMsg>()
            .add_message::<DamageNumberMsg>()
            .add_message::<ParticleBurstMsg>()
            .add_message::<KillSlowMoMsg>()
            .add_message::<CameraZoomMsg>()
            .add_message::<ScreenFlashMsg>()
            .add_systems(Update, (
                hitstop_system,
                hit_flash_system,
                damage_number_spawn_system,
                damage_number_float_system,
                particle_burst_spawn_system,
                particle_system,
                kill_slow_mo_system,
                camera_zoom_msg_system,
                screen_flash_spawn_system,
                screen_flash_fade_system,
                ambient_particle_spawn_system,
                ambient_particle_system,
            ).run_if(in_state(AppState::Run)));
    }
}

// --- Hitstop ---

/// Request to freeze attacker and target for N frames.
#[derive(Message, Clone, Debug)]
pub struct HitstopMsg {
    pub attacker: Entity,
    pub target: Entity,
    pub freeze_frames: u32,
}

/// Global hitstop state -- when active, gameplay entities should not advance.
#[derive(Resource, Default)]
pub struct HitstopState {
    pub frames_remaining: u32,
    /// Entities that are frozen (attacker + target).
    pub frozen_entities: Vec<Entity>,
}

impl HitstopState {
    pub fn is_active(&self) -> bool {
        self.frames_remaining > 0
    }
}

pub fn gameplay_unfrozen(hitstop: Res<HitstopState>) -> bool {
    !hitstop.is_active()
}

fn hitstop_system(
    mut state: ResMut<HitstopState>,
    mut msgs: MessageReader<HitstopMsg>,
) {
    // Apply new hitstop messages (take the longest one).
    for msg in msgs.read() {
        if msg.freeze_frames > state.frames_remaining {
            state.frames_remaining = msg.freeze_frames;
            state.frozen_entities.clear();
            state.frozen_entities.push(msg.attacker);
            state.frozen_entities.push(msg.target);
        }
    }

    // Tick down.
    if state.frames_remaining > 0 {
        state.frames_remaining -= 1;
        if state.frames_remaining == 0 {
            state.frozen_entities.clear();
        }
    }
}

// --- Hit Flash ---

/// Request to flash an entity white for N frames.
#[derive(Message, Clone, Debug)]
pub struct HitFlashMsg {
    pub entity: Entity,
    pub flash_frames: u32,
}

/// Component attached to entities that are currently flashing.
#[derive(Component)]
pub struct HitFlash {
    pub original_color: Color,
    pub frames_remaining: u32,
}

fn hit_flash_system(
    mut msgs: MessageReader<HitFlashMsg>,
    mut query: Query<(Entity, &mut Sprite, Option<&mut HitFlash>)>,
    mut commands: Commands,
) {
    // Apply new flash messages.
    for msg in msgs.read() {
        if let Ok((entity, sprite, existing)) = query.get(msg.entity) {
            if existing.is_none() {
                commands.entity(entity).insert(HitFlash {
                    original_color: sprite.color,
                    frames_remaining: msg.flash_frames,
                });
            }
        }
    }

    // Tick flashes.
    for (entity, mut sprite, flash) in &mut query {
        if let Some(mut flash) = flash {
            if flash.frames_remaining > 0 {
                sprite.color = Color::WHITE;
                flash.frames_remaining -= 1;
            } else {
                sprite.color = flash.original_color;
                commands.entity(entity).remove::<HitFlash>();
            }
        }
    }
}

// --- Damage Numbers ---

/// Request to spawn a floating damage number.
#[derive(Message, Clone, Debug)]
pub struct DamageNumberMsg {
    pub position: Vec2,
    pub amount: f32,
    pub is_critical: bool,
}

/// Component for floating damage number text.
#[derive(Component)]
pub struct DamageNumber {
    pub elapsed_frames: u32,
    pub total_frames: u32,
    pub start_y: f32,
}

fn damage_number_spawn_system(
    mut msgs: MessageReader<DamageNumberMsg>,
    mut commands: Commands,
) {
    for msg in msgs.read() {
        let screen_pos = crate::rendering::isometric::world_to_screen(msg.position.x, msg.position.y);
        let font_size = if msg.is_critical { 32.0 } else { 22.0 };
        // HDR values so damage numbers glow with bloom.
        let color = if msg.is_critical {
            Color::srgb(5.0, 4.0, 1.0)
        } else {
            Color::srgb(3.0, 2.5, 1.0)
        };
        let text = format!("{}", msg.amount as i32);

        commands.spawn((
            Text2d::new(text),
            TextFont {
                font_size,
                ..default()
            },
            TextColor(color),
            Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y + 20.0, 100.0)),
            DamageNumber {
                elapsed_frames: 0,
                total_frames: 40,
                start_y: screen_pos.y + 20.0,
            },
        ));
    }
}

/// Float damage numbers upward and fade out.
fn damage_number_float_system(
    mut query: Query<(Entity, &mut Transform, &mut TextColor, &mut DamageNumber)>,
    mut commands: Commands,
) {
    for (entity, mut transform, mut text_color, mut dmg_num) in &mut query {
        dmg_num.elapsed_frames += 1;
        let t = dmg_num.elapsed_frames as f32 / dmg_num.total_frames as f32;

        // Float upward.
        transform.translation.y = dmg_num.start_y + t * 40.0;

        // Fade out in the second half.
        let alpha = if t > 0.5 { 1.0 - (t - 0.5) * 2.0 } else { 1.0 };
        text_color.0 = text_color.0.with_alpha(alpha.max(0.0));

        if dmg_num.elapsed_frames >= dmg_num.total_frames {
            commands.entity(entity).despawn();
        }
    }
}

// --- Particles ---

/// Request to spawn a burst of particles.
#[derive(Message, Clone, Debug)]
pub struct ParticleBurstMsg {
    pub position: Vec2,
    pub direction: Vec2,
    pub count: u32,
    pub color: Color,
}

/// Individual particle component.
#[derive(Component)]
pub struct Particle {
    pub velocity: Vec2,
    pub elapsed_frames: u32,
    pub total_frames: u32,
    pub gravity: f32,
}

fn particle_burst_spawn_system(
    mut msgs: MessageReader<ParticleBurstMsg>,
    mut commands: Commands,
) {
    for msg in msgs.read() {
        let screen_pos = crate::rendering::isometric::world_to_screen(msg.position.x, msg.position.y);
        for i in 0..msg.count {
            // Spread particles in a cone around the direction.
            let angle_offset = (i as f32 / msg.count as f32 - 0.5) * std::f32::consts::PI;
            let base_angle = if msg.direction != Vec2::ZERO {
                msg.direction.y.atan2(msg.direction.x)
            } else {
                // Radial burst.
                (i as f32 / msg.count as f32) * std::f32::consts::TAU
            };
            let angle = base_angle + angle_offset * 0.5;
            let speed = 120.0 + (i as f32 * 7.0) % 80.0;
            let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

            commands.spawn((
                Sprite {
                    color: msg.color,
                    custom_size: Some(Vec2::splat(5.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(screen_pos.x, screen_pos.y, 90.0)),
                Particle {
                    velocity,
                    elapsed_frames: 0,
                    total_frames: 20,
                    gravity: -200.0,
                },
            ));
        }
    }
}

/// Move and fade particles.
fn particle_system(
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut Particle)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut transform, mut sprite, mut particle) in &mut query {
        particle.elapsed_frames += 1;
        let dt = time.delta_secs();

        // Apply velocity and gravity.
        transform.translation.x += particle.velocity.x * dt;
        transform.translation.y += particle.velocity.y * dt;
        particle.velocity.y += particle.gravity * dt;

        // Fade out.
        let t = particle.elapsed_frames as f32 / particle.total_frames as f32;
        let alpha = (1.0 - t).max(0.0);
        sprite.color = sprite.color.with_alpha(alpha);

        if particle.elapsed_frames >= particle.total_frames {
            commands.entity(entity).despawn();
        }
    }
}

// --- Kill Slow-Motion ---

/// Message to trigger slow-motion on last enemy kill.
#[derive(Message, Clone, Debug)]
pub struct KillSlowMoMsg {
    /// Duration in frames at reduced speed.
    pub frames: u32,
    /// Time scale (e.g., 0.3 for 30% speed).
    pub time_scale: f32,
}

/// Global time scale state.
#[derive(Resource, Default)]
pub struct TimeScaleState {
    pub scale: f32,
    pub frames_remaining: u32,
}

fn kill_slow_mo_system(
    mut msgs: MessageReader<KillSlowMoMsg>,
    mut state: ResMut<TimeScaleState>,
    mut time_settings: ResMut<Time<Virtual>>,
) {
    for msg in msgs.read() {
        state.scale = msg.time_scale;
        state.frames_remaining = msg.frames;
    }

    if state.frames_remaining > 0 {
        time_settings.set_relative_speed(state.scale);
        state.frames_remaining -= 1;
        if state.frames_remaining == 0 {
            time_settings.set_relative_speed(1.0);
        }
    }
}

// --- Camera Zoom on Message ---

/// Request a camera zoom pulse.
#[derive(Message, Clone, Debug)]
pub struct CameraZoomMsg {
    pub zoom_factor: f32,
    pub duration_frames: u32,
}

fn camera_zoom_msg_system(
    mut msgs: MessageReader<CameraZoomMsg>,
    mut zoom: ResMut<CameraZoomPulse>,
) {
    for msg in msgs.read() {
        zoom.pulse(msg.zoom_factor, msg.duration_frames);
    }
}

// --- Screen Flash ---

/// Request a full-screen flash overlay.
#[derive(Message, Clone, Debug)]
pub struct ScreenFlashMsg {
    pub color: Color,
}

/// Component for a full-screen flash overlay that fades out.
#[derive(Component)]
pub struct ScreenFlash {
    pub elapsed_frames: u32,
    pub total_frames: u32,
    pub start_alpha: f32,
}

fn screen_flash_spawn_system(
    mut msgs: MessageReader<ScreenFlashMsg>,
    mut commands: Commands,
    camera_query: Query<&Transform, With<MainCamera>>,
) {
    for msg in msgs.read() {
        let cam_pos = camera_query
            .single()
            .map(|t| t.translation.xy())
            .unwrap_or(Vec2::ZERO);

        commands.spawn((
            ArenaEntity,
            Sprite {
                color: msg.color.with_alpha(0.8),
                custom_size: Some(Vec2::new(1920.0, 1080.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(cam_pos.x, cam_pos.y, 600.0)),
            ScreenFlash {
                elapsed_frames: 0,
                total_frames: 5,
                start_alpha: 0.8,
            },
        ));
    }
}

fn screen_flash_fade_system(
    mut query: Query<(Entity, &mut Sprite, &mut ScreenFlash)>,
    mut commands: Commands,
) {
    for (entity, mut sprite, mut flash) in &mut query {
        flash.elapsed_frames += 1;
        let t = flash.elapsed_frames as f32 / flash.total_frames as f32;
        let alpha = flash.start_alpha * (1.0 - t).max(0.0);
        sprite.color = sprite.color.with_alpha(alpha);

        if flash.elapsed_frames >= flash.total_frames {
            commands.entity(entity).despawn();
        }
    }
}

// --- Ambient Particles (dust & embers) ---

/// Tracks the total number of ambient particles to enforce the cap.
#[derive(Resource, Default)]
pub struct AmbientParticleCount {
    pub count: u32,
}

/// Component for ambient dust/ember particles.
#[derive(Component)]
pub struct AmbientParticle {
    pub velocity: Vec2,
    pub elapsed_frames: u32,
    pub total_frames: u32,
    pub phase: f32,
    pub is_ember: bool,
}

fn ambient_particle_spawn_system(
    mut commands: Commands,
    mut count: ResMut<AmbientParticleCount>,
    camera_query: Query<&Transform, With<MainCamera>>,
    time: Res<Time>,
) {
    if count.count >= 60 {
        return;
    }

    let cam_pos = camera_query
        .single()
        .map(|t| t.translation.xy())
        .unwrap_or(Vec2::ZERO);

    // Spawn 1-2 per frame using elapsed time as seed for variation.
    let seed = (time.elapsed_secs() * 1000.0) as u32;
    let to_spawn = 1 + (seed % 2);

    for i in 0..to_spawn {
        if count.count >= 60 {
            break;
        }

        let h = seed.wrapping_mul(2654435761).wrapping_add(i * 1337);
        let is_ember = h % 3 == 0;

        // Random position within viewport area around camera.
        let ox = ((h % 1920) as f32) - 960.0;
        let oy = (((h >> 8) % 1080) as f32) - 540.0;
        let px = cam_pos.x + ox;
        let py = cam_pos.y + oy;

        let lifetime = 120 + (h % 60);

        let (color, size, velocity) = if is_ember {
            // Ember: orange, gentle sine wave motion.
            (
                Color::srgba(3.0, 1.5, 0.3, 0.35),
                Vec2::splat(2.0),
                Vec2::new(((h % 40) as f32 - 20.0) * 0.3, 8.0 + (h % 10) as f32),
            )
        } else {
            // Dust mote: warm brown, slow drift upward.
            let alpha = 0.2 + ((h % 20) as f32 * 0.01);
            (
                Color::srgba(0.45, 0.35, 0.25, alpha),
                Vec2::splat(3.0),
                Vec2::new(((h % 30) as f32 - 15.0) * 0.2, 5.0 + (h % 8) as f32),
            )
        };

        let phase = (h % 628) as f32 * 0.01;

        commands.spawn((
            ArenaEntity,
            Sprite {
                color,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(Vec3::new(px, py, 95.0)),
            AmbientParticle {
                velocity,
                elapsed_frames: 0,
                total_frames: lifetime,
                phase,
                is_ember,
            },
        ));

        count.count += 1;
    }
}

fn ambient_particle_system(
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut AmbientParticle)>,
    mut commands: Commands,
    mut count: ResMut<AmbientParticleCount>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut sprite, mut particle) in &mut query {
        particle.elapsed_frames += 1;
        let dt = time.delta_secs();

        // Apply drift velocity.
        transform.translation.x += particle.velocity.x * dt;
        transform.translation.y += particle.velocity.y * dt;

        // Embers get a sine wave x-offset.
        if particle.is_ember {
            let t = time.elapsed_secs();
            transform.translation.x += (t * 2.0 + particle.phase).sin() * 15.0 * dt;
        }

        // Fade in at start, fade out at end.
        let t = particle.elapsed_frames as f32 / particle.total_frames as f32;
        let alpha_mult = if t < 0.1 {
            t / 0.1
        } else if t > 0.7 {
            (1.0 - t) / 0.3
        } else {
            1.0
        };
        sprite.color = sprite.color.with_alpha(
            sprite.color.alpha().min(0.4) * alpha_mult.max(0.0),
        );

        if particle.elapsed_frames >= particle.total_frames {
            commands.entity(entity).despawn();
            count.count = count.count.saturating_sub(1);
        }
    }
}
