use bevy::prelude::*;
use serde::Deserialize;

/// RON wrapper matching the `CombatFeel(...)` format in combat_feel.ron.
/// Only the fields we actively use in code are extracted; the rest are
/// parsed to allow the full RON file to load but stored for future use.
#[derive(Debug, Clone, Deserialize)]
struct CombatFeel {
    hitstop: HitstopFeel,
    screen_shake: ScreenShakeFeel,
    camera: CameraFeel,
    hit_flash: HitFlashFeel,
    particles: ParticlesFeel,
    damage_type_particles: DamageTypeParticles,
    input_buffer_frames: u32,
    coyote_time_frames: u32,
    time_dilation: TimeDilationFeel,
    enemy_feel: EnemyFeelData,
    sound: SoundFeel,
    player_movement: PlayerMovementFeel,
}

#[derive(Debug, Clone, Deserialize)]
struct HitstopFeel {
    normal_frames: u32,
    crit_frames: u32,
    heavy_frames: u32,
    kill_frames: u32,
    kill_slowmo_frames: u32,
    kill_slowmo_speed: f32,
    last_enemy_frames: u32,
    last_enemy_slowmo_frames: u32,
    last_enemy_slowmo_speed: f32,
    boss_transition_frames: u32,
    player_hit_frames: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct ScreenShakeFeel {
    normal_intensity_px: f32,
    normal_duration_frames: u32,
    normal_falloff: String,
    crit_intensity_px: f32,
    crit_duration_frames: u32,
    crit_falloff: String,
    player_hit_intensity_px: f32,
    player_hit_duration_frames: u32,
    player_hit_falloff: String,
    boss_slam_intensity_px: f32,
    boss_slam_duration_frames: u32,
    boss_slam_falloff: String,
    boss_slam_bounce_count: u32,
    explosion_intensity_px: f32,
    explosion_duration_frames: u32,
    explosion_falloff: String,
    room_clear_intensity_px: f32,
    room_clear_duration_frames: u32,
    room_clear_falloff: String,
    max_total_intensity_px: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct CameraFeel {
    special_zoom_percent: f32,
    special_zoom_duration_ms: u64,
    special_zoom_easing: String,
    boss_entrance_zoom_duration_ms: u64,
    boss_entrance_hold_ms: u64,
    kill_zoom_percent: f32,
    kill_zoom_duration_ms: u64,
    dodge_near_miss_zoom_percent: f32,
    dodge_near_miss_duration_ms: u64,
    debate_shake_px: f32,
    debate_vignette_flash_ms: u64,
    follow_smoothing: f32,
    cursor_lead_px: f32,
    movement_lead_px: f32,
    lookahead_responsiveness: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct HitFlashFeel {
    enemy_flash_frames: u32,
    enemy_damage_tint_frames: u32,
    player_vignette_frames: u32,
    low_hp_threshold: f32,
    low_hp_heartbeat_bpm: u32,
    chromatic_aberration_frames: u32,
    chromatic_aberration_offset_px: f32,
    death_dissolve_particle_count: u32,
    death_dissolve_frames: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct ParticlesFeel {
    normal_count: u32,
    normal_fade_frames: u32,
    normal_gravity: bool,
    crit_count: u32,
    crit_extra_sparks: u32,
    crit_fade_frames: u32,
    kill_count: u32,
    kill_screen_flash: bool,
    kill_linger_sparkle_count: u32,
    kill_linger_duration_frames: u32,
    dodge_dust_count: u32,
    dodge_dust_fade_frames: u32,
    cast_particle_count: u32,
    cast_particle_fade_frames: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct DamageTypeParticleTheme {
    description: String,
    color_primary: String,
    color_secondary: String,
    behavior: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct DamageTypeParticles {
    audit: DamageTypeParticleTheme,
    penalty: DamageTypeParticleTheme,
    freeze: DamageTypeParticleTheme,
    bureaucracy: DamageTypeParticleTheme,
    expedited: DamageTypeParticleTheme,
    interest: DamageTypeParticleTheme,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct TimeDilationFeel {
    kill_speed: f32,
    kill_duration_ms: u64,
    last_kill_speed: f32,
    last_kill_duration_ms: u64,
    boss_phase_speed: f32,
    boss_phase_duration_ms: u64,
    near_death_speed: f32,
    near_death_threshold: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct EnemyFeelData {
    recoil_animation_frames: u32,
    knockback_curve: String,
    wall_slam_bonus_damage_percent: f32,
    melee_telegraph_min_ms: u64,
    ranged_aoe_telegraph_min_ms: u64,
    max_active_attackers: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SoundFeel {
    layers_per_hit: u32,
    pitch_variation_percent: f32,
    pre_impact_dip_ms: u64,
    pre_impact_dip_amount: f32,
    min_variants_per_sfx: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct PlayerMovementFeel {
    acceleration: f32,
    deceleration: f32,
    attack_move_multiplier: f32,
    recovery_move_multiplier: f32,
}

/// Bevy resource exposing combat feel parameters to game systems.
/// Flattened from the nested RON structure for easy access.
#[derive(Resource, Debug, Clone)]
pub struct CombatFeelConfig {
    // Hitstop
    pub hitstop_normal_frames: u32,
    pub hitstop_crit_frames: u32,
    pub hitstop_player_hit_frames: u32,

    // Screen shake
    pub shake_normal_intensity: f32,
    pub shake_normal_frames: u32,
    pub shake_crit_intensity: f32,
    pub shake_crit_frames: u32,
    pub shake_player_hit_intensity: f32,
    pub shake_player_hit_frames: u32,
    pub shake_max_total_intensity: f32,

    // Hit flash
    pub hit_flash_frames: u32,
    pub death_dissolve_frames: u32,

    // Particles
    pub particle_normal_count: u32,
    pub particle_crit_count: u32,

    // Stagger
    pub stagger_frames: u32,

    // Knockback
    pub knockback_total_frames: u32,

    // Input buffering
    pub input_buffer_frames: u32,

    // Camera follow
    pub camera_follow_smoothing: f32,
    pub camera_cursor_lead_px: f32,
    pub camera_movement_lead_px: f32,
    pub camera_lookahead_responsiveness: f32,

    // Player locomotion
    pub player_acceleration: f32,
    pub player_deceleration: f32,
    pub player_attack_move_multiplier: f32,
    pub player_recovery_move_multiplier: f32,

    // Kill / room moments
    pub room_clear_shake_intensity: f32,
    pub room_clear_shake_frames: u32,
    pub kill_slowmo_frames: u32,
    pub kill_slowmo_speed: f32,
    pub last_enemy_slowmo_frames: u32,
    pub last_enemy_slowmo_speed: f32,
}

impl Default for CombatFeelConfig {
    fn default() -> Self {
        Self {
            hitstop_normal_frames: 3,
            hitstop_crit_frames: 5,
            hitstop_player_hit_frames: 2,
            shake_normal_intensity: 2.0,
            shake_normal_frames: 4,
            shake_crit_intensity: 4.0,
            shake_crit_frames: 6,
            shake_player_hit_intensity: 3.0,
            shake_player_hit_frames: 5,
            shake_max_total_intensity: 12.0,
            hit_flash_frames: 2,
            death_dissolve_frames: 20,
            particle_normal_count: 8,
            particle_crit_count: 15,
            stagger_frames: 3,
            knockback_total_frames: 8,
            input_buffer_frames: 4,
            camera_follow_smoothing: 0.12,
            camera_cursor_lead_px: 96.0,
            camera_movement_lead_px: 48.0,
            camera_lookahead_responsiveness: 0.18,
            player_acceleration: 2400.0,
            player_deceleration: 3000.0,
            player_attack_move_multiplier: 0.45,
            player_recovery_move_multiplier: 0.72,
            room_clear_shake_intensity: 3.0,
            room_clear_shake_frames: 12,
            kill_slowmo_frames: 8,
            kill_slowmo_speed: 0.5,
            last_enemy_slowmo_frames: 16,
            last_enemy_slowmo_speed: 0.3,
        }
    }
}

/// Load combat feel config from `{base}/feel/combat_feel.ron`.
pub fn load_combat_feel(base: &std::path::Path) -> Result<CombatFeelConfig, String> {
    let path = base.join("feel/combat_feel.ron");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let feel: CombatFeel = ron::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {e}", path.display()))?;

    Ok(CombatFeelConfig {
        hitstop_normal_frames: feel.hitstop.normal_frames,
        hitstop_crit_frames: feel.hitstop.crit_frames,
        hitstop_player_hit_frames: feel.hitstop.player_hit_frames,
        shake_normal_intensity: feel.screen_shake.normal_intensity_px,
        shake_normal_frames: feel.screen_shake.normal_duration_frames,
        shake_crit_intensity: feel.screen_shake.crit_intensity_px,
        shake_crit_frames: feel.screen_shake.crit_duration_frames,
        shake_player_hit_intensity: feel.screen_shake.player_hit_intensity_px,
        shake_player_hit_frames: feel.screen_shake.player_hit_duration_frames,
        shake_max_total_intensity: feel.screen_shake.max_total_intensity_px,
        hit_flash_frames: feel.hit_flash.enemy_flash_frames,
        death_dissolve_frames: feel.hit_flash.death_dissolve_frames,
        particle_normal_count: feel.particles.normal_count,
        particle_crit_count: feel.particles.crit_count,
        stagger_frames: feel.enemy_feel.recoil_animation_frames,
        knockback_total_frames: 8, // Not in the RON file; keep the existing default.
        input_buffer_frames: feel.input_buffer_frames,
        camera_follow_smoothing: feel.camera.follow_smoothing,
        camera_cursor_lead_px: feel.camera.cursor_lead_px,
        camera_movement_lead_px: feel.camera.movement_lead_px,
        camera_lookahead_responsiveness: feel.camera.lookahead_responsiveness,
        player_acceleration: feel.player_movement.acceleration,
        player_deceleration: feel.player_movement.deceleration,
        player_attack_move_multiplier: feel.player_movement.attack_move_multiplier,
        player_recovery_move_multiplier: feel.player_movement.recovery_move_multiplier,
        room_clear_shake_intensity: feel.screen_shake.room_clear_intensity_px,
        room_clear_shake_frames: feel.screen_shake.room_clear_duration_frames,
        kill_slowmo_frames: feel.hitstop.kill_slowmo_frames,
        kill_slowmo_speed: feel.hitstop.kill_slowmo_speed,
        last_enemy_slowmo_frames: feel.hitstop.last_enemy_slowmo_frames,
        last_enemy_slowmo_speed: feel.hitstop.last_enemy_slowmo_speed,
    })
}
