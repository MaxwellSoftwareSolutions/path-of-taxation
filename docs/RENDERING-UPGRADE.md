# Rendering and Atmosphere Upgrade Plan

## Current Architecture Summary

Path of Taxation uses a pure 2D sprite pipeline with Bevy 0.18's `Camera2d` and `Sprite` components. The coordinate system is 2:1 dimetric isometric projection with explicit z-layer constants controlling draw order.

**Projection:** `world_to_screen()` maps `(world_x, world_y)` to screen via `(x - y, (x + y) / 2)`.

**Z-layer stack (current):**

| Layer | Z Value | Contents |
|---|---|---|
| `BG_FAR` | -500 | Dark fade ring beyond arena |
| `TERRAIN_BASE` | -350 | Floor tiles with jitter/rotation |
| `TERRAIN_DETAIL` | -250 | Ground overlays (shadow patches, moss, puddles) |
| `GROUND_PROPS` | -150 | Walls, barrels, crates, columns |
| Entities | -80..+80 | Depth-sorted characters (`-world_y * 0.1`) |
| `FOG` | 200 | Drifting fog rectangles |
| `VIGNETTE` | 300 | Sprite-based edge darkness bars |
| `UI_WORLD` | 90 | World-space UI (enemy health bars) |

**Camera:** `Camera2d` with smooth follow, screen shake queue, and zoom pulse. Orthographic projection accessed via `Projection::Orthographic`.

**Assets:** ~35 tile PNGs, 7 effect PNGs, FLARE-format 2048x2048 character spritesheets (8x8 grid, 256px cells). No WGSL shaders yet.

**VFX:** Frame-counted hitstop, white hit flash (color swap), floating damage numbers, directional particle bursts (small colored squares), kill slow-mo via `Time<Virtual>`, camera zoom pulses.

**Atmosphere:** 8 drifting fog sprites (tinted semi-transparent rectangles with sinusoidal motion), 45 ground detail overlays (colored rectangles), 4 vignette bars around screen edges.

**Terrain:** Diamond-shaped arena (radius 7 in Manhattan distance), tiles placed on an 88px grid with position jitter, random rotation, scale variation, and per-tile color noise. Tile selection varies by distance from center (clean stone center, mixed mid-ring, damaged edges). Shadow emboss copies on 1/3 of tiles.

---

## PATH A: 2D Polish (Achievable Now, Incremental)

Everything below stays within the existing `Camera2d` sprite pipeline. No 3D meshes, no fundamental architecture changes. Each item is independent and can be merged in any order.

### A1. Post-Processing Color Grading

**Goal:** Apply a full-screen color transform (desaturation, tint, contrast curve, optional vignette) as a GPU post-process instead of manually tinting every sprite.

**Approach:** Bevy 0.18's `Material2d` trait with a fullscreen quad rendered after the main 2D pass.

**Implementation:**

1. Create `assets/shaders/color_grading.wgsl`:

```wgsl
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var screen_texture: texture_2d<f32>;
@group(2) @binding(1) var screen_sampler: sampler;
@group(2) @binding(2) var<uniform> params: ColorGradingParams;

struct ColorGradingParams {
    // x: contrast, y: saturation, z: brightness, w: vignette_intensity
    adjustments: vec4<f32>,
    // RGB tint applied after grading, w: vignette_radius
    tint: vec4<f32>,
    // x: lift, y: gamma, z: gain (shadow/mid/highlight)
    lift_gamma_gain: vec4<f32>,
};

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var color = textureSample(screen_texture, screen_sampler, uv);

    // --- Lift / Gamma / Gain (3-way color correction) ---
    let lift = params.lift_gamma_gain.x;
    let gamma = params.lift_gamma_gain.y;
    let gain = params.lift_gamma_gain.z;
    color = vec4<f32>(
        pow(max((color.rgb - lift) * gain, vec3<f32>(0.0)), vec3<f32>(1.0 / gamma)),
        color.a
    );

    // --- Contrast ---
    let contrast = params.adjustments.x;
    color = vec4<f32>(
        (color.rgb - 0.5) * contrast + 0.5,
        color.a
    );

    // --- Saturation ---
    let saturation = params.adjustments.y;
    let luma = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = vec4<f32>(
        mix(vec3<f32>(luma), color.rgb, saturation),
        color.a
    );

    // --- Brightness ---
    color = vec4<f32>(color.rgb * params.adjustments.z, color.a);

    // --- Tint ---
    color = vec4<f32>(color.rgb * params.tint.rgb, color.a);

    // --- Vignette ---
    let vig_intensity = params.adjustments.w;
    let vig_radius = params.tint.w;
    let dist = distance(uv, vec2<f32>(0.5, 0.5));
    let vig = smoothstep(vig_radius, vig_radius - 0.25, dist);
    color = vec4<f32>(color.rgb * mix(1.0 - vig_intensity, 1.0, vig), color.a);

    return saturate(color);
}
```

2. Create `client/src/rendering/post_process.rs`:

```rust
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::Material2d;

/// Color grading material applied as a fullscreen post-process.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct ColorGradingMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub screen_texture: Handle<Image>,
    #[uniform(2)]
    pub params: ColorGradingParams,
}

#[derive(Clone, Copy, ShaderType)]
pub struct ColorGradingParams {
    pub adjustments: Vec4,  // contrast, saturation, brightness, vignette_intensity
    pub tint: Vec4,         // r, g, b, vignette_radius
    pub lift_gamma_gain: Vec4, // lift, gamma, gain, _pad
}

impl Default for ColorGradingParams {
    fn default() -> Self {
        Self {
            // Slightly crushed, desaturated, warm -- PoE dungeon look
            adjustments: Vec4::new(1.15, 0.85, 0.95, 0.45),
            tint: Vec4::new(1.0, 0.95, 0.88, 0.65),
            lift_gamma_gain: Vec4::new(0.02, 1.1, 1.0, 0.0),
        }
    }
}

impl Material2d for ColorGradingMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/color_grading.wgsl".into()
    }
}
```

3. System to spawn the fullscreen quad and pipe the camera output through it. In Bevy 0.18, the recommended approach is to use a second camera at a higher order that renders a `Mesh2d` + `MeshMaterial2d<ColorGradingMaterial>` quad textured with the first camera's render target.

```rust
fn setup_post_process(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorGradingMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Create render target image
    let size = Extent3d { width: 1920, height: 1080, depth_or_array_layers: 1 };
    let mut render_target = Image {
        texture_descriptor: TextureDescriptor {
            size,
            format: TextureFormat::Bevy8BitTransmittance,
            usage: TextureUsages::TEXTURE_BINDING
                 | TextureUsages::COPY_DST
                 | TextureUsages::RENDER_ATTACHMENT,
            ..default()
        },
        ..default()
    };
    render_target.resize(size);
    let render_target_handle = images.add(render_target);

    // Main game camera renders to texture (order 0)
    // (Modify existing spawn_camera to set Camera.target)

    // Post-process camera (order 1) renders fullscreen quad to screen
    let material = materials.add(ColorGradingMaterial {
        screen_texture: render_target_handle.clone(),
        params: ColorGradingParams::default(),
    });

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(2.0, 2.0))),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));

    commands.spawn((
        Camera2d,
        Camera { order: 1, ..default() },
    ));
}
```

**Preset library** (swap at runtime for biome variation):

| Preset | Contrast | Saturation | Tint | Feel |
|---|---|---|---|---|
| `dungeon_cold` | 1.20 | 0.75 | (0.85, 0.90, 1.0) | Icy tax office |
| `dungeon_warm` | 1.15 | 0.85 | (1.0, 0.95, 0.88) | Default warm dungeon |
| `blood_arena` | 1.30 | 1.10 | (1.1, 0.85, 0.80) | Boss room |
| `forest_decay` | 1.10 | 0.70 | (0.90, 0.95, 0.85) | Outdoor ruins |

**Effort:** ~1 day. One shader file, one Rust module, wire into camera plugin.

---

### A2. Bloom for Spell Effects and Emissives

**Goal:** Projectiles, explosions, and torches emit a soft glow that bleeds past their sprite boundaries.

**Approach:** Bevy 0.18 ships `BloomSettings` on the camera entity. For 2D this works when using HDR rendering.

**Implementation:**

```rust
// In spawn_camera(), add to the camera entity:
commands.spawn((
    Camera2d,
    MainCamera,
    Camera {
        hdr: true,
        ..default()
    },
    Bloom {
        intensity: 0.15,
        low_frequency_boost: 0.6,
        low_frequency_boost_curvature: 0.5,
        high_pass_frequency: 1.0,
        composite_mode: BloomCompositeMode::Additive,
        ..default()
    },
    Tonemapping::AcesFitted,
    Transform::default(),
));
```

**Making sprites emit:** Bloom picks up fragments brighter than 1.0 in HDR space. Use `Color::srgb(2.0, 1.5, 0.5)` (values > 1.0) for emissive sprites:

```rust
// Projectile spawning (in combat.rs or wherever projectiles are created):
Sprite {
    image: sprites.projectile.clone(),
    color: Color::srgb(2.5, 1.8, 0.6), // HDR -- triggers bloom
    custom_size: Some(Vec2::new(16.0, 16.0)),
    ..default()
}
```

**Categorize emissive objects:**

| Object | HDR Color | Bloom Behavior |
|---|---|---|
| Arcane Bolt | `(2.5, 1.8, 0.6)` | Strong warm glow |
| Fireball / Slash | `(3.0, 0.8, 0.2)` | Hot orange bleed |
| Darkness Bolt | `(1.2, 0.5, 2.5)` | Purple corona |
| Magic Orb | `(0.8, 2.0, 2.5)` | Cyan shimmer |
| Torch glow sprite | `(2.0, 1.4, 0.4)` | Warm ambient |
| Hit flash white | `(3.0, 3.0, 3.0)` | Bright pop on damage |

**Dynamic bloom intensity:** Pulse bloom `intensity` up briefly on big hits for a satisfying flash. Wire a system that reads `HitstopState` and temporarily increases bloom:

```rust
fn bloom_hitstop_pulse(
    hitstop: Res<HitstopState>,
    mut camera_query: Query<&mut Bloom, With<MainCamera>>,
) {
    let Ok(mut bloom) = camera_query.single_mut() else { return };
    bloom.intensity = if hitstop.is_active() { 0.35 } else { 0.15 };
}
```

**Effort:** ~2 hours. Add `Bloom` + `hdr: true` to camera, change emissive sprite colors to HDR values.

**Caveat:** HDR + bloom has a GPU cost. Profile on target hardware. On integrated GPUs, reduce `Bloom::max_mip` or disable entirely behind a settings toggle.

---

### A3. Ambient Particle Systems (Rain, Dust, Embers)

**Goal:** Persistent ambient particles that fill the arena with life -- falling rain, drifting dust motes, floating embers near torches.

**Approach:** Pre-allocate a fixed particle pool. Each frame, update positions in a single system. No per-frame spawn/despawn allocation.

**Implementation:**

```rust
// client/src/plugins/ambient_particles.rs

use bevy::prelude::*;
use crate::plugins::camera::MainCamera;

pub struct AmbientParticlePlugin;

impl Plugin for AmbientParticlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AmbientParticleConfig>()
            .add_systems(Startup, spawn_particle_pool)
            .add_systems(Update, update_ambient_particles);
    }
}

#[derive(Resource)]
pub struct AmbientParticleConfig {
    pub mode: ParticleMode,
    pub density: u32,        // active particles out of pool
    pub wind: Vec2,          // world-space wind direction
}

pub enum ParticleMode {
    Rain,
    Dust,
    Embers,
    None,
}

impl Default for AmbientParticleConfig {
    fn default() -> Self {
        Self {
            mode: ParticleMode::Rain,
            density: 200,
            wind: Vec2::new(-30.0, 0.0),
        }
    }
}

#[derive(Component)]
pub struct AmbientParticle {
    pub index: u32,
    pub velocity: Vec2,
    pub phase: f32,          // for sinusoidal drift
    pub lifetime_phase: f32, // 0..1 wrap for fading
}

const POOL_SIZE: u32 = 300;
const SPREAD: f32 = 1200.0; // screen-space spread around camera

fn spawn_particle_pool(mut commands: Commands) {
    for i in 0..POOL_SIZE {
        // Start particles at random positions (will be repositioned on first update)
        let hash = i.wrapping_mul(2654435761);
        let x = ((hash % 2400) as f32 - 1200.0);
        let y = (((hash >> 8) % 1600) as f32 - 800.0);

        commands.spawn((
            AmbientParticle {
                index: i,
                velocity: Vec2::ZERO,
                phase: (hash % 628) as f32 / 100.0,
                lifetime_phase: (i as f32) / (POOL_SIZE as f32),
            },
            Sprite {
                color: Color::srgba(0.7, 0.75, 0.85, 0.0), // invisible until activated
                custom_size: Some(Vec2::new(1.5, 6.0)),     // rain streak shape
                ..default()
            },
            Transform::from_xyz(x, y, 150.0), // between FOG and VIGNETTE
        ));
    }
}

fn update_ambient_particles(
    config: Res<AmbientParticleConfig>,
    time: Res<Time>,
    camera_query: Query<&Transform, With<MainCamera>>,
    mut particles: Query<(
        &mut AmbientParticle,
        &mut Transform,
        &mut Sprite,
    ), Without<MainCamera>>,
) {
    let Ok(cam) = camera_query.single() else { return };
    let cam_pos = cam.translation.truncate();
    let dt = time.delta_secs();

    for (mut particle, mut transform, mut sprite) in &mut particles {
        // Deactivate particles beyond density threshold
        if particle.index >= config.density {
            sprite.color = sprite.color.with_alpha(0.0);
            continue;
        }

        match config.mode {
            ParticleMode::Rain => {
                // Rain falls down-left in screen space (isometric rain direction)
                let rain_speed = 400.0 + (particle.phase * 47.0) % 120.0;
                let drift = (time.elapsed_secs() * 0.5 + particle.phase).sin() * 8.0;

                transform.translation.x += (config.wind.x + drift) * dt;
                transform.translation.y -= rain_speed * dt;

                // Wrap around camera viewport
                let rel = transform.translation.truncate() - cam_pos;
                if rel.y < -SPREAD * 0.5 {
                    transform.translation.y += SPREAD;
                    transform.translation.x = cam_pos.x + ((particle.index as f32 * 97.0) % SPREAD) - SPREAD * 0.5;
                }
                if rel.x.abs() > SPREAD * 0.5 {
                    transform.translation.x = cam_pos.x + ((particle.index as f32 * 53.0) % SPREAD) - SPREAD * 0.5;
                    transform.translation.y = cam_pos.y + SPREAD * 0.5;
                }

                // Rain appearance: thin white streaks, semi-transparent
                sprite.custom_size = Some(Vec2::new(1.0, 5.0 + (particle.phase * 13.0) % 4.0));
                sprite.color = Color::srgba(0.6, 0.65, 0.75, 0.15 + (particle.phase * 7.0) % 0.1);
            }
            ParticleMode::Dust => {
                // Slow drifting motes
                let t = time.elapsed_secs();
                let drift_x = (t * 0.3 + particle.phase).sin() * 15.0;
                let drift_y = (t * 0.2 + particle.phase * 1.3).cos() * 10.0;

                transform.translation.x += (config.wind.x * 0.1 + drift_x * 0.5) * dt;
                transform.translation.y += drift_y * 0.5 * dt;

                // Wrap
                let rel = transform.translation.truncate() - cam_pos;
                if rel.x.abs() > SPREAD * 0.5 || rel.y.abs() > SPREAD * 0.5 {
                    transform.translation.x = cam_pos.x + ((particle.index as f32 * 97.0) % SPREAD) - SPREAD * 0.5;
                    transform.translation.y = cam_pos.y + ((particle.index as f32 * 53.0) % SPREAD) - SPREAD * 0.5;
                }

                // Dust: small warm circles, very faint
                let pulse = ((t * 0.8 + particle.phase).sin() * 0.5 + 0.5) * 0.12;
                sprite.custom_size = Some(Vec2::splat(2.0 + (particle.phase * 11.0) % 2.0));
                sprite.color = Color::srgba(0.9, 0.85, 0.7, 0.05 + pulse);
            }
            ParticleMode::Embers => {
                // Rise upward with drift
                let t = time.elapsed_secs();
                let rise_speed = 30.0 + (particle.phase * 23.0) % 40.0;
                let drift_x = (t * 0.6 + particle.phase).sin() * 20.0;

                transform.translation.x += drift_x * dt;
                transform.translation.y += rise_speed * dt;

                // Wrap from bottom
                let rel = transform.translation.truncate() - cam_pos;
                if rel.y > SPREAD * 0.5 {
                    transform.translation.y -= SPREAD;
                    transform.translation.x = cam_pos.x + ((particle.index as f32 * 97.0) % SPREAD) - SPREAD * 0.5;
                }

                // Embers: small orange-red dots
                let flicker = ((t * 3.0 + particle.phase * 5.0).sin() * 0.5 + 0.5);
                sprite.custom_size = Some(Vec2::splat(2.0 + flicker));
                // HDR orange for bloom interaction
                sprite.color = Color::srgba(
                    1.5 + flicker,
                    0.6 + flicker * 0.3,
                    0.1,
                    0.2 + flicker * 0.15,
                );
            }
            ParticleMode::None => {
                sprite.color = sprite.color.with_alpha(0.0);
            }
        }
    }
}
```

**Performance notes:**
- 300 sprites is negligible for Bevy's sprite batching.
- No per-frame entity spawn/despawn -- zero allocator pressure.
- Wind vector can be animated per-biome or per-room.
- Switch `ParticleMode` on room transitions for variety.

**Effort:** ~3 hours. One new plugin file, register in `mod.rs`, configure per biome.

---

### A4. Fake Point Light Glow (Sprite-Based)

**Goal:** Torches, braziers, and spell impacts cast visible pools of warm light on the ground without real lighting.

**Approach:** A soft radial-gradient texture (white center fading to transparent edges) spawned at light source positions, tinted and blended additively. Animate scale and alpha for flicker.

**Implementation:**

1. Create a 128x128 radial gradient PNG (`assets/sprites/effects/light_glow.png`) -- white center, alpha fades to 0 at edges. Can be generated procedurally at startup instead:

```rust
fn generate_glow_texture(images: &mut Assets<Image>) -> Handle<Image> {
    let size = 128u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    let center = size as f32 / 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt() / center;
            let alpha = (1.0 - dist).max(0.0).powi(2); // quadratic falloff
            let idx = ((y * size + x) * 4) as usize;
            data[idx] = 255;     // R
            data[idx + 1] = 255; // G
            data[idx + 2] = 255; // B
            data[idx + 3] = (alpha * 255.0) as u8;
        }
    }

    images.add(Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}
```

2. Spawn glow sprites at light positions:

```rust
#[derive(Component)]
pub struct FakePointLight {
    pub base_radius: f32,
    pub flicker_speed: f32,
    pub flicker_amount: f32, // 0.0-1.0
    pub phase: f32,
}

fn spawn_torch_light(
    commands: &mut Commands,
    glow_handle: Handle<Image>,
    screen_pos: Vec2,
    z: f32,
) {
    // Ground glow pool
    commands.spawn((
        FakePointLight {
            base_radius: 180.0,
            flicker_speed: 2.5,
            flicker_amount: 0.15,
            phase: rand::random::<f32>() * std::f32::consts::TAU,
        },
        Sprite {
            image: glow_handle.clone(),
            // HDR warm orange for bloom interaction
            color: Color::srgba(1.8, 1.2, 0.4, 0.12),
            custom_size: Some(Vec2::splat(180.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(
            screen_pos.x,
            screen_pos.y,
            z_layers::TERRAIN_DETAIL + 1.0, // just above terrain
        )),
    ));
}

fn animate_fake_lights(
    time: Res<Time>,
    mut query: Query<(&FakePointLight, &mut Transform, &mut Sprite)>,
) {
    let t = time.elapsed_secs();
    for (light, mut transform, mut sprite) in &mut query {
        let flicker = (t * light.flicker_speed + light.phase).sin();
        let noise = ((t * light.flicker_speed * 2.7 + light.phase * 3.0).sin()) * 0.3;
        let combined = (flicker + noise).clamp(-1.0, 1.0);

        let radius = light.base_radius * (1.0 + combined * light.flicker_amount);
        sprite.custom_size = Some(Vec2::splat(radius));

        let alpha = 0.12 + combined * 0.03;
        sprite.color = Color::srgba(1.8, 1.2, 0.4, alpha.max(0.05));
    }
}
```

3. Place lights at the 4 stone columns and at scattered torch positions in `setup_run`:

```rust
// After spawning each stone column:
spawn_torch_light(&mut commands, glow_handle.clone(), screen, z_layers::TERRAIN_DETAIL + 1.0);
```

**Visual layers per light source:**
- Layer 1: Ground glow (large, low alpha, below entities)
- Layer 2: Core glow (small, brighter, at entity z)
- Layer 3: Embers (via `AmbientParticle` emitter, optional)

**Effort:** ~2 hours. Generate or load texture, spawn system, flicker animation.

---

### A5. Dense Vegetation and Prop Scatter

**Goal:** Break the sparse feel by filling arena edges with foliage, rubble, and environmental clutter.

**Current state:** 22 hand-placed props at fixed positions. This is not enough density.

**Approach:** Procedural scatter using the existing `tile_hash` for determinism.

```rust
// In setup_run, after LAYER 3b (scattered props):

// === LAYER 3c: VEGETATION SCATTER ===
let vegetation_assets: Vec<Handle<Image>> = vec![
    asset_server.load("sprites/tiles/vegetation_grass_01.png"),
    asset_server.load("sprites/tiles/vegetation_grass_02.png"),
    asset_server.load("sprites/tiles/vegetation_fern.png"),
    asset_server.load("sprites/tiles/vegetation_bush.png"),
    asset_server.load("sprites/tiles/vegetation_dead_branch.png"),
    asset_server.load("sprites/tiles/rubble_stones.png"),
    asset_server.load("sprites/tiles/rubble_bones.png"),
];

for row in -arena_radius..=arena_radius {
    for col in -arena_radius..=arena_radius {
        let dist = row.abs() + col.abs();
        if dist > arena_radius { continue; }

        let h = tile_hash(row + 100, col + 200); // different seed than floor

        // Density increases toward edges: 0% at center, ~60% at edge
        let edge_t = dist as f32 / arena_radius as f32;
        let spawn_chance = (edge_t * 0.6 * 100.0) as u32;
        if h % 100 >= spawn_chance { continue; }

        let jx = ((h % 40) as f32 - 20.0) * 3.0;
        let jy = (((h >> 6) % 40) as f32 - 20.0) * 3.0;
        let wx = col as f32 * tile_spacing + jx;
        let wy = row as f32 * tile_spacing + jy;
        let screen = world_to_screen(wx, wy);

        let veg_idx = (h % vegetation_assets.len() as u32) as usize;
        let scale = 0.6 + ((h >> 3) % 10) as f32 * 0.05;
        let rotation = ((h >> 5) % 30) as f32 * 0.02 - 0.3;

        // Darker and more muted toward edges
        let brightness = 0.25 + (1.0 - edge_t) * 0.15;
        let tint = Color::srgba(
            brightness * 0.85,
            brightness,
            brightness * 0.75,
            0.7 + ((h >> 8) % 30) as f32 * 0.01,
        );

        commands.spawn((
            ArenaEntity,
            Sprite {
                image: vegetation_assets[veg_idx].clone(),
                color: tint,
                custom_size: Some(Vec2::new(48.0, 48.0) * scale),
                ..default()
            },
            Transform::from_translation(Vec3::new(
                screen.x, screen.y,
                z_layers::GROUND_PROPS - 1.0 + (row + col) as f32 * 0.001,
            )).with_rotation(Quat::from_rotation_z(rotation)),
        ));
    }
}
```

**Required new assets (download or generate):**
- `vegetation_grass_01.png` / `_02.png` -- low tufts, pre-rendered isometric
- `vegetation_fern.png` -- small fern frond
- `vegetation_bush.png` -- scrubby bush
- `vegetation_dead_branch.png` -- fallen stick/branch
- `rubble_stones.png` -- scattered pebbles
- `rubble_bones.png` -- skeletal remains (tax-themed)

**Effort:** ~1 hour code, plus asset sourcing time.

---

### A6. Better Fog System

**Current state:** 8 solid-colored rectangles with sinusoidal drift. These read as floating colored boxes.

**Upgrade:** Use the same radial gradient texture from A4 (or a dedicated fog texture with perlin-like noise baked in), increase count, vary sizes dramatically, and add opacity pulsing.

```rust
// Replace the current fog_configs block in setup_run:

let fog_texture: Handle<Image> = asset_server.load("sprites/effects/fog_cloud.png");
// fog_cloud.png: a 256x128 soft white cloud shape, alpha-only essentially

let fog_count = 20;
for i in 0..fog_count {
    let h = tile_hash(i * 31, i * 17 + 99);
    let wx = ((h % 1200) as f32 - 600.0);
    let wy = (((h >> 8) % 1000) as f32 - 500.0);
    let screen = world_to_screen(wx, wy);

    let width = 200.0 + ((h >> 4) % 400) as f32;
    let height = width * (0.3 + ((h >> 12) % 30) as f32 * 0.01);
    let speed = 0.1 + ((h >> 6) % 20) as f32 * 0.015;
    let phase = (h % 628) as f32 / 100.0;

    commands.spawn((
        ArenaEntity,
        FogDrift {
            speed,
            phase,
            amplitude: Vec2::new(40.0 + ((h >> 3) % 30) as f32, 15.0),
            base_pos: screen,
        },
        Sprite {
            image: fog_texture.clone(),
            color: Color::srgba(0.15, 0.12, 0.10, 0.04 + ((h >> 10) % 4) as f32 * 0.01),
            custom_size: Some(Vec2::new(width, height)),
            ..default()
        },
        Transform::from_translation(Vec3::new(screen.x, screen.y, z_layers::FOG))
            .with_rotation(Quat::from_rotation_z(((h % 60) as f32 - 30.0) * 0.02)),
    ));
}
```

Additionally, add opacity pulsing to `fog_drift_system`:

```rust
fn fog_drift_system(
    time: Res<Time>,
    mut query: Query<(&FogDrift, &mut Transform, &mut Sprite)>,
) {
    let t = time.elapsed_secs();
    for (fog, mut transform, mut sprite) in &mut query {
        let offset_x = (t * fog.speed + fog.phase).sin() * fog.amplitude.x;
        let offset_y = (t * fog.speed * 0.7 + fog.phase + 1.5).cos() * fog.amplitude.y;
        transform.translation.x = fog.base_pos.x + offset_x;
        transform.translation.y = fog.base_pos.y + offset_y;

        // Subtle opacity pulse
        let base_alpha = sprite.color.alpha();
        let pulse = (t * fog.speed * 0.4 + fog.phase * 2.0).sin() * 0.015;
        sprite.color = sprite.color.with_alpha((base_alpha + pulse).clamp(0.01, 0.12));
    }
}
```

**Effort:** ~1 hour. Replace fog spawns, add fog texture asset, update drift system.

---

### A7. Improved Vignette

**Current state:** 4 solid-colored rectangles positioned around screen edges. Moves with the arena, not the camera.

**Fix:** The vignette should be a camera-attached overlay, not world-space. Two options:

**Option 1 (quick):** Parent vignette sprites to the camera entity so they track automatically.

**Option 2 (better):** Handle in the post-process shader (already included in A1 above -- the `vig_intensity` and `vig_radius` parameters).

If implementing A1, remove the sprite-based vignette entirely. If not implementing A1, use Option 1:

```rust
// Spawn vignette as child of camera
commands.entity(camera_entity).with_children(|parent| {
    // Use the glow texture inverted, or a dedicated vignette texture
    parent.spawn((
        Sprite {
            image: vignette_texture.clone(),
            color: Color::srgba(0.01, 0.005, 0.005, 0.6),
            custom_size: Some(Vec2::new(1920.0, 1080.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, z_layers::VIGNETTE)),
    ));
});
```

**Effort:** 15 minutes if going shader route (already done in A1). 30 minutes if camera-parented sprite.

---

### A8. Ground Decals via Sprite Overlays

**Goal:** Blood splatters on kills, scorch marks from fire spells, cracks from heavy impacts.

**Implementation:** On kill/impact events, spawn a persistent sprite at the impact location with slow fade-out or a max count ring buffer.

```rust
#[derive(Component)]
pub struct GroundDecal {
    pub spawn_time: f32,
    pub fade_start: f32,  // seconds before fading begins
    pub fade_duration: f32,
}

#[derive(Resource)]
pub struct DecalPool {
    pub decals: Vec<Entity>,
    pub max_decals: usize,
}

fn spawn_blood_decal(
    commands: &mut Commands,
    pool: &mut DecalPool,
    asset_server: &AssetServer,
    screen_pos: Vec2,
    time: f32,
) {
    // Recycle oldest decal if at capacity
    if pool.decals.len() >= pool.max_decals {
        if let Some(old) = pool.decals.first() {
            commands.entity(*old).despawn();
        }
        pool.decals.remove(0);
    }

    let decal_textures = [
        "sprites/effects/blood_left.png",
        "sprites/effects/blood_right.png",
    ];
    let idx = (time as usize) % decal_textures.len();
    let rotation = (time * 100.0) % std::f32::consts::TAU;

    let entity = commands.spawn((
        GroundDecal {
            spawn_time: time,
            fade_start: 8.0,
            fade_duration: 4.0,
        },
        Sprite {
            image: asset_server.load(decal_textures[idx]),
            color: Color::srgba(0.4, 0.02, 0.02, 0.6),
            custom_size: Some(Vec2::new(32.0, 32.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(
            screen_pos.x, screen_pos.y,
            z_layers::TERRAIN_DETAIL + 0.5,
        )).with_rotation(Quat::from_rotation_z(rotation)),
    )).id();

    pool.decals.push(entity);
}

fn fade_decals(
    time: Res<Time>,
    mut query: Query<(&GroundDecal, &mut Sprite)>,
) {
    let t = time.elapsed_secs();
    for (decal, mut sprite) in &mut query {
        let age = t - decal.spawn_time;
        if age > decal.fade_start {
            let fade_t = ((age - decal.fade_start) / decal.fade_duration).clamp(0.0, 1.0);
            let alpha = 0.6 * (1.0 - fade_t);
            sprite.color = sprite.color.with_alpha(alpha);
        }
    }
}
```

**Effort:** ~2 hours. New decal module, hook into kill/damage events.

---

### PATH A Priority Order

Ranked by visual impact per hour of effort:

| Priority | Item | Effort | Impact |
|---|---|---|---|
| 1 | **A2 Bloom** | 2h | Massive -- spell effects instantly look polished |
| 2 | **A3 Ambient Particles** | 3h | High -- arena feels alive instead of static |
| 3 | **A4 Fake Point Lights** | 2h | High -- warm glow pools add depth and mood |
| 4 | **A6 Better Fog** | 1h | Medium-high -- soft clouds vs. floating boxes |
| 5 | **A1 Color Grading** | 1d | Medium-high -- ties everything into a cohesive look |
| 6 | **A5 Dense Vegetation** | 1h+assets | Medium -- fills out the empty feeling |
| 7 | **A8 Ground Decals** | 2h | Medium -- environmental storytelling, combat feel |
| 8 | **A7 Improved Vignette** | 15min | Low (superseded by A1 shader vignette) |

**Total for all of PATH A:** ~2-3 days of focused work.

---

## PATH B: 2.5D Upgrade (Larger Scope, Future)

This path moves terrain rendering to 3D meshes while keeping characters as billboarded sprites. The camera becomes an `OrthographicProjection` 3D camera instead of `Camera2d`.

### B1. Architecture Changes

**Camera:**
```
Camera2d + Transform  -->  Camera3d + OrthographicProjection + Transform
```

The camera would be positioned above and behind the arena, looking down at the standard isometric angle (30 degrees from horizontal, rotated 45 degrees):

```rust
fn spawn_camera_3d(mut commands: Commands) {
    let iso_rotation = Quat::from_euler(
        EulerRot::YXZ,
        std::f32::consts::FRAC_PI_4, // 45 degrees Y rotation
        -std::f32::consts::FRAC_PI_6, // 30 degrees X tilt (arctan(0.5) for 2:1)
        0.0,
    );

    commands.spawn((
        MainCamera,
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            near: -1000.0,
            far: 1000.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(Vec3::new(0.0, 200.0, 200.0))
            .with_rotation(iso_rotation),
    ));
}
```

**Coordinate system change:** `WorldPosition` would map to actual 3D positions instead of using `world_to_screen`. The isometric projection becomes implicit via the camera angle rather than explicit math.

```rust
// OLD: world_to_screen(wx, wy) -> screen Vec2, placed on 2D Transform
// NEW: place entities at Vec3(wx, 0.0, wy) in 3D space, camera handles projection
```

**What this eliminates:**
- `world_to_screen()` and `screen_to_world()` functions
- Manual z-layer depth sorting (3D depth buffer handles it)
- The `-world_y * 0.1` depth hack

**What this preserves:**
- `WorldPosition` component (just maps to xz plane instead of screen)
- All game logic, combat, movement (untouched)
- HUD and UI (stays in screen-space UI nodes)

---

### B2. Terrain Mesh

Replace the hundreds of floor tile sprites with a single terrain mesh:

```rust
fn generate_terrain_mesh(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    // Diamond-shaped heightfield mesh
    let radius = 7;
    let spacing = 88.0;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices on a grid
    for row in -radius..=radius {
        for col in -radius..=radius {
            if (row.abs() + col.abs()) > radius { continue; }

            let x = col as f32 * spacing;
            let z = row as f32 * spacing;
            let y = procedural_height(x, z); // slight height variation

            positions.push([x, y, z]);
            normals.push([0.0, 1.0, 0.0]); // recalculate after
            uvs.push([
                (col + radius) as f32 / (radius * 2) as f32,
                (row + radius) as f32 / (radius * 2) as f32,
            ]);
        }
    }

    // Triangulate and compute proper normals...
    // (standard heightfield triangulation)

    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices));

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/terrain_albedo.png")),
        normal_map_texture: Some(asset_server.load("textures/terrain_normal.png")),
        metallic: 0.0,
        perceptual_roughness: 0.85,
        ..default()
    });

    (meshes.add(mesh), material)
}
```

**Terrain blending:** Instead of per-tile tinting, use a splatmap texture with 4 channels (RGBA) encoding blend weights for 4 terrain types. A custom shader samples all 4 terrain textures and blends based on splatmap values. This completely eliminates the visible tile grid.

---

### B3. Billboard Sprites for Characters

Characters remain as 2D sprites but are rendered as billboarded quads in 3D space that always face the camera:

```rust
#[derive(Component)]
pub struct Billboard;

fn billboard_system(
    camera_query: Query<&Transform, With<MainCamera>>,
    mut billboards: Query<&mut Transform, (With<Billboard>, Without<MainCamera>)>,
) {
    let Ok(cam) = camera_query.single() else { return };

    for mut transform in &mut billboards {
        // Make the sprite face the camera
        let forward = (cam.translation - transform.translation).normalize();
        transform.look_to(-forward, Vec3::Y);
    }
}
```

Characters would be spawned with `Mesh3d` + `MeshMaterial3d` using an `AlphaMode::Blend` material with sprite textures, or using Bevy 0.18's billboard support if available.

---

### B4. Real Lighting

With 3D meshes, Bevy's PBR lighting pipeline activates automatically:

```rust
// Directional light (sun/moon)
commands.spawn((
    DirectionalLight {
        color: Color::srgb(0.9, 0.85, 0.75),
        illuminance: 800.0,
        shadows_enabled: true,
        ..default()
    },
    Transform::from_rotation(Quat::from_euler(
        EulerRot::XYZ, -0.8, 0.3, 0.0,
    )),
));

// Point lights for torches
commands.spawn((
    PointLight {
        color: Color::srgb(1.0, 0.7, 0.3),
        intensity: 3000.0,
        radius: 0.1,
        range: 15.0,
        shadows_enabled: true,
        ..default()
    },
    Transform::from_xyz(torch_x, 2.0, torch_z),
));

// Ambient light (very dim, moody)
commands.insert_resource(AmbientLight {
    color: Color::srgb(0.4, 0.35, 0.5),
    brightness: 50.0,
});
```

**Shadow mapping** comes free with `shadows_enabled: true` on directional/point lights. Characters (billboard sprites) would cast and receive shadows.

---

### B5. 3D Fog and Atmosphere

Bevy 0.18 provides `DistanceFog` (also called `FogSettings`) on the camera:

```rust
commands.spawn((
    Camera3d::default(),
    // ...
    DistanceFog {
        color: Color::srgb(0.05, 0.04, 0.06),
        falloff: FogFalloff::ExponentialSquared { density: 0.003 },
        ..default()
    },
));
```

This would naturally darken the arena edges and create depth cues without the sprite-based fog/vignette hacks currently in use.

Additionally, volumetric fog (if supported in Bevy 0.18) would add atmospheric shafts of light through gaps in the arena walls.

---

### B6. Migration Path

The transition from 2D to 2.5D can be done incrementally:

**Phase 1: Camera migration (1-2 days)**
- Switch `Camera2d` to `Camera3d` with orthographic projection
- Map `WorldPosition` to `Vec3(x, 0, y)` instead of `world_to_screen`
- All sprites still render via `Sprite` (Bevy renders 2D sprites with a 3D camera)
- Verify all gameplay still works identically

**Phase 2: Terrain mesh (2-3 days)**
- Replace floor tile sprites with a generated 3D mesh
- Create terrain albedo/normal textures
- Keep props, characters, and effects as sprites

**Phase 3: Lighting (1-2 days)**
- Add directional + point lights
- Enable shadows
- Replace fake light glow sprites with real point lights
- Tune ambient and light colors per biome

**Phase 4: Atmosphere (1 day)**
- Add `DistanceFog` to camera
- Remove sprite-based fog and vignette
- Add `Bloom` and `Tonemapping`

**Phase 5: Billboard characters (2-3 days)**
- Move character sprites to billboarded 3D quads
- Ensure animation atlas still works
- Adjust sprite rendering for 3D depth

**Total estimated: 8-12 days**

---

### B7. Systems That Change

| System | 2D (current) | 2.5D (target) |
|---|---|---|
| `depth_sort_system` | Manual z = -world_y * 0.1 | Removed (depth buffer) |
| `world_to_screen` | Explicit math | Removed (camera projection) |
| `screen_to_world` | Inverse math | `Camera::viewport_to_world` |
| `follow_player_system` | Screen-space lerp | 3D position lerp on xz plane |
| `fog_drift_system` | Animate sprite positions | Remove (use `DistanceFog`) or keep as 3D particle fog |
| `setup_run` terrain | Hundreds of sprite spawns | One mesh + material |
| `setup_run` props | Sprite spawns | Sprite spawns (unchanged, or 3D models later) |
| `camera_shake_system` | Translate x/y | Translate x/z (or x/y in camera-local) |
| `camera_zoom_pulse_system` | `ortho.scale` | Same (still orthographic) |
| `spawn_camera` | `Camera2d` | `Camera3d` + `OrthographicProjection` |
| HitFlash | Sprite color swap | Same (sprites still have color) |
| Particles | Sprite position + fade | Same or switch to Bevy's built-in particle system |
| HUD | Unchanged | Unchanged |

---

### B8. What NOT to Change

- **Game logic:** All combat, movement, AI, room progression stays identical.
- **`WorldPosition` component:** Still used, just maps differently.
- **HUD/UI:** All `Node`-based UI stays in screen space.
- **VFX message system:** `HitstopMsg`, `DamageNumberMsg`, etc. -- all untouched.
- **Asset loading:** Sprite textures still loaded the same way.
- **State machine:** `AppState` and `CombatPhase` untouched.

---

## Decision Matrix

| Criterion | PATH A (2D Polish) | PATH B (2.5D) |
|---|---|---|
| Time to ship | 2-3 days | 8-12 days |
| Visual ceiling | Good (7/10) | Excellent (9/10) |
| Risk | Very low | Medium (rendering regressions) |
| Real shadows | No (faked) | Yes |
| Real lighting | No (faked) | Yes |
| Depth fog | No (sprite fog) | Yes (GPU fog) |
| Terrain quality | Sprite tiles (limited) | Mesh + splatmap (seamless) |
| Performance cost | Low (sprite batching) | Medium (3D pipeline, shadows) |
| Reversibility | Full (each item independent) | Low (fundamental change) |

**Recommendation:** Ship PATH A first. Every item in PATH A improves the game immediately and none of them conflict with a future PATH B migration. Bloom (A2), ambient particles (A3), and fake lights (A4) together transform the visual quality in under a day of work. Color grading (A1) and better fog (A6) finish the polish. Then evaluate whether PATH B's real lighting and terrain justify the migration effort.

---

## File Map (PATH A)

New files to create:

```
client/
  assets/
    shaders/
      color_grading.wgsl           (A1)
    sprites/
      effects/
        light_glow.png             (A4 - or generate procedurally)
        fog_cloud.png              (A6)
      tiles/
        vegetation_grass_01.png    (A5)
        vegetation_grass_02.png    (A5)
        vegetation_fern.png        (A5)
        vegetation_bush.png        (A5)
        vegetation_dead_branch.png (A5)
        rubble_stones.png          (A5)
        rubble_bones.png           (A5)
  src/
    rendering/
      post_process.rs              (A1)
    plugins/
      ambient_particles.rs         (A3)
      fake_lights.rs               (A4)
      decals.rs                    (A8)
```

Files to modify:

```
client/src/rendering/mod.rs        -- add post_process module
client/src/plugins/mod.rs          -- add ambient_particles, fake_lights, decals
client/src/plugins/camera.rs       -- add Bloom + hdr to camera (A2), remove sprite vignette
client/src/plugins/run.rs          -- better fog spawns (A6), vegetation scatter (A5), torch lights (A4)
client/src/plugins/vfx.rs          -- HDR colors on particles (A2), bloom pulse on hitstop (A2)
client/src/plugins/combat.rs       -- decal spawns on kills (A8)
client/src/rendering/sprites.rs    -- add new texture handles
```
