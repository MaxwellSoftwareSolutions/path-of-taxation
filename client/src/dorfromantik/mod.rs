use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::math::primitives::{Cuboid, Extrusion, RegularPolygon, Sphere};
use bevy::prelude::*;
use hexx::{Hex, HexLayout, HexOrientation};

pub struct DorfromantikSandboxPlugin;

impl Plugin for DorfromantikSandboxPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DirectionalLightShadowMap { size: 4096 })
            .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.06)))
            .add_systems(
                Startup,
                (
                    setup_lighting,
                    setup_camera,
                    setup_scene,
                    setup_overlay,
                )
                    .chain(),
            )
            .add_systems(Update, update_diorama_camera);
    }
}

#[derive(Resource, Clone)]
pub struct BoardState {
    #[allow(dead_code)]
    pub layout: HexLayout,
    pub tiles: Vec<BoardTileData>,
}

#[derive(Clone)]
pub struct BoardTileData {
    pub coord: Hex,
    pub kind: TileKind,
    pub surface_height: f32,
}

#[derive(Clone)]
struct TileMaterialSet {
    top: Handle<StandardMaterial>,
    side: Handle<StandardMaterial>,
    rim: Handle<StandardMaterial>,
}

#[derive(Clone)]
struct FeatureMeshes {
    trunk: Handle<Mesh>,
    branch: Handle<Mesh>,
    wall: Handle<Mesh>,
    tower: Handle<Mesh>,
    hut: Handle<Mesh>,
    roof: Handle<Mesh>,
    rock: Handle<Mesh>,
    crystal: Handle<Mesh>,
    water_patch: Handle<Mesh>,
    path: Handle<Mesh>,
    reed: Handle<Mesh>,
    flame: Handle<Mesh>,
}

#[derive(Clone)]
struct FeatureMaterials {
    dead_wood: Handle<StandardMaterial>,
    soot_patch: Handle<StandardMaterial>,
    ruin_stone: Handle<StandardMaterial>,
    crystal: Handle<StandardMaterial>,
    muddy_stone: Handle<StandardMaterial>,
    water: Handle<StandardMaterial>,
    reed: Handle<StandardMaterial>,
    settlement_stone: Handle<StandardMaterial>,
    settlement_roof: Handle<StandardMaterial>,
    torch_flame: Handle<StandardMaterial>,
}

#[derive(Clone)]
struct BridgeMeshes {
    root: Handle<Mesh>,
    path: Handle<Mesh>,
    wall: Handle<Mesh>,
}

#[derive(Component)]
struct BoardTile;

#[derive(Component)]
struct DioramaCamera {
    focus: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TileKind {
    AncientForest,
    CorruptedRuins,
    Riverbank,
    FortifiedSettlement,
}

impl TileKind {
    fn top_color(self) -> Color {
        // Darker, richer colors matching the dark fantasy reference.
        match self {
            TileKind::AncientForest => Color::srgb(0.12, 0.18, 0.13),
            TileKind::CorruptedRuins => Color::srgb(0.20, 0.15, 0.24),
            TileKind::Riverbank => Color::srgb(0.18, 0.20, 0.15),
            TileKind::FortifiedSettlement => Color::srgb(0.24, 0.23, 0.22),
        }
    }

    fn side_color(self) -> Color {
        match self {
            TileKind::AncientForest => Color::srgb(0.05, 0.08, 0.06),
            TileKind::CorruptedRuins => Color::srgb(0.08, 0.06, 0.10),
            TileKind::Riverbank => Color::srgb(0.08, 0.07, 0.05),
            TileKind::FortifiedSettlement => Color::srgb(0.09, 0.09, 0.09),
        }
    }

    fn rim_base(self) -> Color {
        match self {
            TileKind::AncientForest => Color::srgb(0.10, 0.18, 0.12),
            TileKind::CorruptedRuins => Color::srgb(0.25, 0.10, 0.30),
            TileKind::Riverbank => Color::srgb(0.10, 0.18, 0.24),
            TileKind::FortifiedSettlement => Color::srgb(0.26, 0.20, 0.15),
        }
    }

    fn rim_emissive(self) -> Color {
        match self {
            TileKind::AncientForest => Color::srgb(0.02, 0.08, 0.04),
            TileKind::CorruptedRuins => Color::srgb(0.18, 0.04, 0.25),
            TileKind::Riverbank => Color::srgb(0.02, 0.07, 0.12),
            TileKind::FortifiedSettlement => Color::srgb(0.12, 0.07, 0.03),
        }
    }
}

fn setup_lighting(mut commands: Commands) {
    // Dim moonlit ambient -- dark fantasy atmosphere.
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.35, 0.38, 0.48),
        brightness: 55.0,
        ..default()
    });

    // Cold directional "moonlight" from high angle.
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.55, 0.58, 0.70),
            illuminance: 5_500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(16.0, 30.0, 10.0).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 24.0,
            maximum_distance: 120.0,
            ..default()
        }
        .build(),
    ));

    // Warm point light at the settlement area (like a campfire glow).
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.65, 0.30),
            intensity: 25_000.0,
            range: 18.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(8.0, 3.0, -1.0),
    ));

    // Faint purple glow at the corrupted ruins.
    commands.spawn((
        PointLight {
            color: Color::srgb(0.55, 0.20, 0.70),
            intensity: 12_000.0,
            range: 14.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-2.0, 2.5, 2.0),
    ));
}

fn setup_camera(mut commands: Commands) {
    let controller = DioramaCamera {
        focus: Vec3::new(2.0, 0.6, 1.0),
        radius: 26.0,
        yaw: -0.55,
        pitch: 0.68,
    };

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 27.0_f32.to_radians(),
            near: 0.1,
            far: 300.0,
            ..default()
        }),
        camera_transform(&controller),
        controller,
    ));
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let layout = HexLayout {
        orientation: HexOrientation::Flat,
        scale: Vec2::splat(2.9),
        ..Default::default()
    };
    let tiles = starter_tiles();

    spawn_atmosphere(&mut commands, &mut meshes, &mut materials);

    let ancient_forest = TileMaterialSet {
        top: materials.add(tile_material(TileKind::AncientForest.top_color(), 0.98, 0.0)),
        side: materials.add(tile_material(TileKind::AncientForest.side_color(), 1.0, 0.0)),
        rim: materials.add(rim_material(
            TileKind::AncientForest.rim_base(),
            TileKind::AncientForest.rim_emissive(),
        )),
    };
    let corrupted_ruins = TileMaterialSet {
        top: materials.add(tile_material(TileKind::CorruptedRuins.top_color(), 0.95, 0.03)),
        side: materials.add(tile_material(TileKind::CorruptedRuins.side_color(), 1.0, 0.0)),
        rim: materials.add(rim_material(
            TileKind::CorruptedRuins.rim_base(),
            TileKind::CorruptedRuins.rim_emissive(),
        )),
    };
    let riverbank = TileMaterialSet {
        top: materials.add(tile_material(TileKind::Riverbank.top_color(), 0.93, 0.0)),
        side: materials.add(tile_material(TileKind::Riverbank.side_color(), 1.0, 0.0)),
        rim: materials.add(rim_material(
            TileKind::Riverbank.rim_base(),
            TileKind::Riverbank.rim_emissive(),
        )),
    };
    let fortified_settlement = TileMaterialSet {
        top: materials.add(tile_material(
            TileKind::FortifiedSettlement.top_color(),
            0.90,
            0.0,
        )),
        side: materials.add(tile_material(
            TileKind::FortifiedSettlement.side_color(),
            1.0,
            0.0,
        )),
        rim: materials.add(rim_material(
            TileKind::FortifiedSettlement.rim_base(),
            TileKind::FortifiedSettlement.rim_emissive(),
        )),
    };

    let feature_meshes = FeatureMeshes {
        trunk: meshes.add(Cuboid::new(0.14, 0.62, 0.14)),
        branch: meshes.add(Cuboid::new(0.10, 0.42, 0.10)),
        wall: meshes.add(Cuboid::new(0.84, 0.36, 0.20)),
        tower: meshes.add(Cuboid::new(0.26, 0.62, 0.26)),
        hut: meshes.add(Cuboid::new(0.52, 0.30, 0.42)),
        roof: meshes.add(Cuboid::new(0.62, 0.14, 0.50)),
        rock: meshes.add(Sphere::new(0.22).mesh().uv(20, 12)),
        crystal: meshes.add(Cuboid::new(0.16, 0.54, 0.16)),
        water_patch: meshes.add(Cuboid::new(1.10, 0.05, 1.40)),
        path: meshes.add(Cuboid::new(0.90, 0.03, 0.30)),
        reed: meshes.add(Cuboid::new(0.04, 0.34, 0.04)),
        flame: meshes.add(Sphere::new(0.08).mesh().uv(14, 8)),
    };
    let feature_materials = FeatureMaterials {
        dead_wood: materials.add(tile_material(Color::srgb(0.19, 0.17, 0.14), 1.0, 0.0)),
        soot_patch: materials.add(tile_material(Color::srgb(0.10, 0.12, 0.10), 0.98, 0.0)),
        ruin_stone: materials.add(tile_material(Color::srgb(0.34, 0.32, 0.36), 0.97, 0.0)),
        crystal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.42, 0.11, 0.55),
            emissive: Color::srgb(0.22, 0.04, 0.28).into(),
            perceptual_roughness: 0.40,
            metallic: 0.08,
            ..default()
        }),
        muddy_stone: materials.add(tile_material(Color::srgb(0.30, 0.28, 0.24), 0.96, 0.0)),
        water: materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.22, 0.27),
            emissive: Color::srgb(0.01, 0.04, 0.06).into(),
            perceptual_roughness: 0.16,
            metallic: 0.02,
            ..default()
        }),
        reed: materials.add(tile_material(Color::srgb(0.24, 0.29, 0.20), 0.95, 0.0)),
        settlement_stone: materials.add(tile_material(
            Color::srgb(0.39, 0.38, 0.38),
            0.98,
            0.0,
        )),
        settlement_roof: materials.add(tile_material(
            Color::srgb(0.22, 0.18, 0.16),
            0.92,
            0.0,
        )),
        torch_flame: materials.add(StandardMaterial {
            base_color: Color::srgb(1.00, 0.56, 0.20),
            emissive: Color::srgb(0.90, 0.30, 0.08).into(),
            perceptual_roughness: 0.32,
            ..default()
        }),
    };
    let bridge_meshes = BridgeMeshes {
        root: meshes.add(Cuboid::new(0.22, 0.10, 1.68)),
        path: meshes.add(Cuboid::new(0.80, 0.04, 1.86)),
        wall: meshes.add(Cuboid::new(0.22, 0.18, 1.70)),
    };

    // Hex polygon radii sized to nearly touch at layout scale 2.9.
    // hexx layout scale is the distance between hex centers.
    // For flat-top hexes: the horizontal distance between centers = scale.x * sqrt(3).
    // The polygon circumradius needs to be ~scale / sqrt(3) to fill the space.
    // 2.9 / 1.732 ≈ 1.67. Use 1.62 for a tiny gap (beveled edge look).
    let body_mesh = meshes.add(
        Extrusion::new(RegularPolygon::new(1.58, 6), 0.92)
            .mesh()
            .build(),
    );
    let skirt_mesh = meshes.add(
        Extrusion::new(RegularPolygon::new(1.66, 6), 0.26)
            .mesh()
            .build(),
    );
    let rim_mesh = meshes.add(
        Extrusion::new(RegularPolygon::new(1.62, 6), 0.08)
            .mesh()
            .build(),
    );
    let cap_meshes = [
        meshes.add(
            Extrusion::new(RegularPolygon::new(1.52, 6), 0.12)
                .mesh()
                .build(),
        ),
        meshes.add(
            Extrusion::new(RegularPolygon::new(1.48, 6), 0.14)
                .mesh()
                .build(),
        ),
        meshes.add(
            Extrusion::new(RegularPolygon::new(1.44, 6), 0.16)
                .mesh()
                .build(),
        ),
    ];

    for (index, tile) in tiles.iter().enumerate() {
        let world = layout.hex_to_world_pos(tile.coord);
        let tile_materials = match tile.kind {
            TileKind::AncientForest => ancient_forest.clone(),
            TileKind::CorruptedRuins => corrupted_ruins.clone(),
            TileKind::Riverbank => riverbank.clone(),
            TileKind::FortifiedSettlement => fortified_settlement.clone(),
        };
        let variant = index % 5;
        let shell_rotation =
            Quat::from_rotation_y(variant as f32 * (PI / 6.0)) * Quat::from_rotation_x(-FRAC_PI_2);

        commands
            .spawn((
                BoardTile,
                Transform::from_xyz(world.x, 0.0, world.y),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Name::new(format!("Tile {index}")),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(skirt_mesh.clone()),
                    MeshMaterial3d(tile_materials.side.clone()),
                    Transform::from_xyz(0.0, -0.16, 0.0)
                        .with_rotation(shell_rotation),
                ));
                parent.spawn((
                    Mesh3d(body_mesh.clone()),
                    MeshMaterial3d(tile_materials.side.clone()),
                    Transform::from_xyz(0.0, tile.surface_height * 0.44 - 0.10, 0.0)
                        .with_rotation(shell_rotation),
                ));
                parent.spawn((
                    Mesh3d(rim_mesh.clone()),
                    MeshMaterial3d(tile_materials.rim.clone()),
                    Transform::from_xyz(0.0, tile.surface_height + 0.03, 0.0)
                        .with_rotation(shell_rotation),
                ));
                parent.spawn((
                    Mesh3d(cap_meshes[variant % 3].clone()),
                    MeshMaterial3d(tile_materials.top.clone()),
                    Transform::from_xyz(0.0, tile.surface_height + 0.08, 0.0)
                        .with_rotation(shell_rotation),
                ));

                spawn_tile_feature(parent, tile, variant, &feature_meshes, &feature_materials);
            });
    }

    spawn_tile_connectors(
        &mut commands,
        &layout,
        &tiles,
        &bridge_meshes,
        &feature_materials,
    );

    commands.insert_resource(BoardState { layout, tiles });
}

fn spawn_atmosphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let abyss_mesh = meshes.add(Cuboid::new(120.0, 0.55, 120.0));
    let fog_mesh = meshes.add(Sphere::new(1.0).mesh().uv(20, 12));
    let abyss = materials.add(tile_material(Color::srgb(0.04, 0.04, 0.05), 1.0, 0.0));
    let island_shadow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.07, 0.07, 0.08),
        emissive: Color::srgb(0.01, 0.01, 0.02).into(),
        perceptual_roughness: 1.0,
        ..default()
    });
    let forest_mist = materials.add(StandardMaterial {
        base_color: Color::srgba(0.11, 0.18, 0.14, 0.10),
        emissive: Color::srgb(0.01, 0.02, 0.01).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let ruin_mist = materials.add(StandardMaterial {
        base_color: Color::srgba(0.18, 0.08, 0.20, 0.11),
        emissive: Color::srgb(0.03, 0.00, 0.04).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let river_mist = materials.add(StandardMaterial {
        base_color: Color::srgba(0.08, 0.16, 0.18, 0.09),
        emissive: Color::srgb(0.01, 0.02, 0.03).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let ember_mist = materials.add(StandardMaterial {
        base_color: Color::srgba(0.20, 0.10, 0.06, 0.08),
        emissive: Color::srgb(0.03, 0.01, 0.00).into(),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(abyss_mesh),
        MeshMaterial3d(abyss),
        Transform::from_xyz(0.0, -0.30, 0.0),
    ));
    commands.spawn((
        Mesh3d(fog_mesh.clone()),
        MeshMaterial3d(island_shadow),
        Transform::from_xyz(1.2, -0.20, 0.6).with_scale(Vec3::new(28.0, 0.6, 24.0)),
    ));
    commands.spawn((
        Mesh3d(fog_mesh.clone()),
        MeshMaterial3d(forest_mist.clone()),
        Transform::from_xyz(-9.6, 0.06, 4.6).with_scale(Vec3::new(6.2, 0.22, 4.2)),
    ));
    commands.spawn((
        Mesh3d(fog_mesh.clone()),
        MeshMaterial3d(ruin_mist.clone()),
        Transform::from_xyz(-6.8, 0.12, 2.2).with_scale(Vec3::new(4.8, 0.22, 3.4)),
    ));
    commands.spawn((
        Mesh3d(fog_mesh.clone()),
        MeshMaterial3d(forest_mist.clone()),
        Transform::from_xyz(-4.2, 0.08, 3.0).with_scale(Vec3::new(5.0, 0.20, 3.6)),
    ));
    commands.spawn((
        Mesh3d(fog_mesh.clone()),
        MeshMaterial3d(ruin_mist.clone()),
        Transform::from_xyz(-0.8, 0.14, 3.0).with_scale(Vec3::new(5.4, 0.24, 4.0)),
    ));
    commands.spawn((
        Mesh3d(fog_mesh.clone()),
        MeshMaterial3d(river_mist.clone()),
        Transform::from_xyz(2.8, 0.05, 1.0).with_scale(Vec3::new(5.6, 0.18, 3.8)),
    ));
    commands.spawn((
        Mesh3d(fog_mesh.clone()),
        MeshMaterial3d(river_mist),
        Transform::from_xyz(6.0, 0.06, 0.0).with_scale(Vec3::new(5.0, 0.16, 3.4)),
    ));
    commands.spawn((
        Mesh3d(fog_mesh),
        MeshMaterial3d(ember_mist),
        Transform::from_xyz(10.2, 0.08, -0.8).with_scale(Vec3::new(5.2, 0.20, 3.4)),
    ));
}

fn spawn_tile_feature(
    parent: &mut ChildSpawnerCommands,
    tile: &BoardTileData,
    variant: usize,
    meshes: &FeatureMeshes,
    materials: &FeatureMaterials,
) {
    let surface_y = tile.surface_height + 0.16;

    match tile.kind {
        TileKind::AncientForest => spawn_forest_feature(parent, surface_y, variant, meshes, materials),
        TileKind::CorruptedRuins => spawn_ruin_feature(parent, surface_y, variant, meshes, materials),
        TileKind::Riverbank => spawn_river_feature(parent, surface_y, variant, meshes, materials),
        TileKind::FortifiedSettlement => {
            spawn_settlement_feature(parent, surface_y, variant, meshes, materials)
        }
    }
}

fn spawn_forest_feature(
    parent: &mut ChildSpawnerCommands,
    surface_y: f32,
    variant: usize,
    meshes: &FeatureMeshes,
    materials: &FeatureMaterials,
) {
    let tree_layouts: [[(f32, f32, f32, f32, f32); 3]; 5] = [
        [
            (-0.34, -0.08, 1.00, 0.28, -0.18),
            (0.18, 0.22, 0.82, -0.22, 0.24),
            (0.36, -0.26, 0.68, 0.16, -0.14),
        ],
        [
            (-0.26, 0.04, 0.94, 0.22, -0.12),
            (0.30, 0.18, 0.74, -0.28, 0.20),
            (0.10, -0.30, 0.64, 0.12, -0.20),
        ],
        [
            (-0.38, 0.10, 0.78, 0.18, 0.10),
            (0.22, 0.28, 0.90, -0.24, 0.26),
            (0.34, -0.18, 0.58, 0.10, -0.16),
        ],
        [
            (-0.20, -0.18, 0.96, 0.30, -0.22),
            (0.26, 0.26, 0.70, -0.18, 0.18),
            (0.42, -0.06, 0.62, 0.08, -0.10),
        ],
        [
            (-0.32, 0.04, 0.86, 0.24, -0.08),
            (0.14, 0.24, 0.78, -0.20, 0.14),
            (0.36, -0.28, 0.66, 0.14, -0.18),
        ],
    ];
    let floor_patch_scales = [
        Vec3::new(1.8, 0.25, 1.2),
        Vec3::new(1.4, 0.22, 1.0),
        Vec3::new(1.6, 0.18, 0.8),
        Vec3::new(1.3, 0.28, 1.1),
        Vec3::new(1.9, 0.20, 0.95),
    ];

    for (dx, dz, height, lean, twist) in tree_layouts[variant] {
        parent.spawn((
            Mesh3d(meshes.trunk.clone()),
            MeshMaterial3d(materials.dead_wood.clone()),
            Transform::from_xyz(dx, surface_y + 0.30 * height, dz)
                .with_rotation(Quat::from_rotation_z(lean) * Quat::from_rotation_y(twist))
                .with_scale(Vec3::new(0.90, height, 0.90)),
        ));
        parent.spawn((
            Mesh3d(meshes.branch.clone()),
            MeshMaterial3d(materials.dead_wood.clone()),
            Transform::from_xyz(dx + 0.05, surface_y + 0.56 * height, dz)
                .with_rotation(
                    Quat::from_rotation_x(0.32 + lean * 0.20)
                        * Quat::from_rotation_z(-0.36 + twist),
                )
                .with_scale(Vec3::new(0.75, 1.0, 0.75)),
        ));
    }

    parent.spawn((
        Mesh3d(meshes.rock.clone()),
        MeshMaterial3d(materials.soot_patch.clone()),
        Transform::from_xyz(-0.04 + variant as f32 * 0.02, surface_y + 0.08, 0.08)
            .with_scale(floor_patch_scales[variant]),
    ));

    if variant >= 2 {
        parent.spawn((
            Mesh3d(meshes.branch.clone()),
            MeshMaterial3d(materials.dead_wood.clone()),
            Transform::from_xyz(0.12, surface_y + 0.12, -0.26)
                .with_rotation(Quat::from_rotation_y(-0.64) * Quat::from_rotation_z(0.22))
                .with_scale(Vec3::new(1.0, 0.8, 1.8)),
        ));
    }
}

fn spawn_ruin_feature(
    parent: &mut ChildSpawnerCommands,
    surface_y: f32,
    variant: usize,
    meshes: &FeatureMeshes,
    materials: &FeatureMaterials,
) {
    let path_angles = [0.24, 0.34, 0.48, 0.14, 0.28];
    parent.spawn((
        Mesh3d(meshes.path.clone()),
        MeshMaterial3d(materials.ruin_stone.clone()),
        Transform::from_xyz(0.06, surface_y + 0.01, -0.02)
            .with_rotation(Quat::from_rotation_y(path_angles[variant]))
            .with_scale(Vec3::new(1.55, 1.0, 2.2)),
    ));

    let wall_sets: [[(f32, f32, f32, f32); 3]; 5] = [
        [(-0.28, 0.16, 0.18, 1.0), (0.24, -0.18, -0.26, 0.82), (0.18, 0.28, 0.08, 0.68)],
        [(-0.22, 0.18, 0.12, 0.90), (0.30, -0.10, -0.20, 0.74), (0.08, 0.30, 0.04, 0.72)],
        [(-0.24, 0.08, 0.26, 0.88), (0.18, -0.24, -0.32, 0.78), (0.28, 0.22, 0.12, 0.60)],
        [(-0.30, 0.20, 0.08, 1.02), (0.18, -0.16, -0.18, 0.70), (0.26, 0.26, 0.18, 0.64)],
        [(-0.20, 0.12, 0.20, 0.96), (0.26, -0.14, -0.24, 0.80), (0.12, 0.32, 0.10, 0.58)],
    ];
    let crystal_sets: [[(f32, f32, f32); 2]; 5] = [
        [(-0.12, -0.08, 1.0), (0.22, 0.08, 0.74)],
        [(-0.06, -0.10, 0.86), (0.26, 0.14, 0.80)],
        [(-0.18, -0.04, 0.92), (0.18, 0.12, 0.68)],
        [(-0.10, -0.14, 0.80), (0.28, 0.04, 0.84)],
        [(-0.14, -0.10, 0.88), (0.20, 0.12, 0.72)],
    ];

    for (dx, dz, rot, scale) in wall_sets[variant] {
        parent.spawn((
            Mesh3d(meshes.wall.clone()),
            MeshMaterial3d(materials.ruin_stone.clone()),
            Transform::from_xyz(dx, surface_y + 0.20 * scale, dz)
                .with_rotation(Quat::from_rotation_y(rot))
                .with_scale(Vec3::splat(scale)),
        ));
    }
    for (dx, dz, scale) in crystal_sets[variant] {
        parent.spawn((
            Mesh3d(meshes.crystal.clone()),
            MeshMaterial3d(materials.crystal.clone()),
            Transform::from_xyz(dx, surface_y + 0.28 * scale, dz)
                .with_rotation(Quat::from_rotation_z(-0.18 + scale * 0.12))
                .with_scale(Vec3::new(1.0, 1.0 + scale * 0.4, 1.0)),
        ));
    }
    if variant != 0 {
        parent.spawn((
            Mesh3d(meshes.rock.clone()),
            MeshMaterial3d(materials.ruin_stone.clone()),
            Transform::from_xyz(-0.26, surface_y + 0.06, 0.24)
                .with_scale(Vec3::new(0.95, 0.36, 0.70)),
        ));
    }
}

fn spawn_river_feature(
    parent: &mut ChildSpawnerCommands,
    surface_y: f32,
    variant: usize,
    meshes: &FeatureMeshes,
    materials: &FeatureMaterials,
) {
    let water_angles = [0.38, 0.52, 0.30, 0.64, 0.44];
    parent.spawn((
        Mesh3d(meshes.water_patch.clone()),
        MeshMaterial3d(materials.water.clone()),
        Transform::from_xyz(0.18, surface_y - 0.06, 0.02)
            .with_rotation(Quat::from_rotation_y(water_angles[variant])),
    ));

    let reed_groups: [[(f32, f32, usize); 3]; 5] = [
        [(-0.34, 0.22, 3), (0.42, -0.10, 4), (0.10, 0.30, 2)],
        [(-0.26, 0.18, 4), (0.36, -0.08, 3), (0.18, 0.34, 2)],
        [(-0.30, 0.28, 2), (0.46, -0.06, 4), (0.06, 0.24, 3)],
        [(-0.38, 0.20, 3), (0.32, -0.14, 4), (0.16, 0.30, 2)],
        [(-0.24, 0.26, 3), (0.40, -0.12, 3), (0.10, 0.34, 3)],
    ];
    for (dx, dz, count) in reed_groups[variant] {
        for blade in 0..count {
            let blade_offset = blade as f32 * 0.06;
            parent.spawn((
                Mesh3d(meshes.reed.clone()),
                MeshMaterial3d(materials.reed.clone()),
                Transform::from_xyz(dx + blade_offset, surface_y + 0.12, dz - blade_offset * 0.4)
                    .with_rotation(Quat::from_rotation_z(0.10 + blade_offset * 0.5))
                    .with_scale(Vec3::new(1.0, 0.9 + blade_offset, 1.0)),
            ));
        }
    }

    parent.spawn((
        Mesh3d(meshes.rock.clone()),
        MeshMaterial3d(materials.muddy_stone.clone()),
        Transform::from_xyz(-0.20, surface_y + 0.05, -0.18)
            .with_scale(Vec3::new(1.10, 0.42, 0.90)),
    ));
    if variant % 2 == 0 {
        parent.spawn((
            Mesh3d(meshes.path.clone()),
            MeshMaterial3d(materials.muddy_stone.clone()),
            Transform::from_xyz(-0.18, surface_y + 0.02, 0.18)
                .with_rotation(Quat::from_rotation_y(-0.48))
                .with_scale(Vec3::new(0.65, 1.0, 1.4)),
        ));
    }
}

fn spawn_settlement_feature(
    parent: &mut ChildSpawnerCommands,
    surface_y: f32,
    variant: usize,
    meshes: &FeatureMeshes,
    materials: &FeatureMaterials,
) {
    let wall_scales = [1.2, 1.0, 1.1, 0.9, 1.25];
    let house_rotations = [-0.18, -0.10, -0.24, -0.04, -0.14];

    parent.spawn((
        Mesh3d(meshes.path.clone()),
        MeshMaterial3d(materials.settlement_stone.clone()),
        Transform::from_xyz(0.02, surface_y + 0.01, 0.08)
            .with_rotation(Quat::from_rotation_y(house_rotations[variant]))
            .with_scale(Vec3::new(1.0, 1.0, 1.8)),
    ));
    parent.spawn((
        Mesh3d(meshes.wall.clone()),
        MeshMaterial3d(materials.settlement_stone.clone()),
        Transform::from_xyz(-0.10, surface_y + 0.22, -0.16)
            .with_rotation(Quat::from_rotation_y(0.10))
            .with_scale(Vec3::new(wall_scales[variant], 1.0, 1.0)),
    ));
    parent.spawn((
        Mesh3d(meshes.wall.clone()),
        MeshMaterial3d(materials.settlement_stone.clone()),
        Transform::from_xyz(0.22, surface_y + 0.22, 0.14)
            .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
            .with_scale(Vec3::new(0.9, 1.0, 1.0)),
    ));
    parent.spawn((
        Mesh3d(meshes.tower.clone()),
        MeshMaterial3d(materials.settlement_stone.clone()),
        Transform::from_xyz(-0.36, surface_y + 0.34, -0.28),
    ));
    parent.spawn((
        Mesh3d(meshes.tower.clone()),
        MeshMaterial3d(materials.settlement_stone.clone()),
        Transform::from_xyz(0.36, surface_y + 0.34, 0.26),
    ));
    parent.spawn((
        Mesh3d(meshes.hut.clone()),
        MeshMaterial3d(materials.settlement_stone.clone()),
        Transform::from_xyz(0.02, surface_y + 0.16, 0.00)
            .with_rotation(Quat::from_rotation_y(house_rotations[variant])),
    ));
    parent.spawn((
        Mesh3d(meshes.roof.clone()),
        MeshMaterial3d(materials.settlement_roof.clone()),
        Transform::from_xyz(0.02, surface_y + 0.38, 0.00)
            .with_rotation(Quat::from_rotation_y(house_rotations[variant] + 0.02)),
    ));

    if variant != 0 {
        for (dx, dz) in [(-0.08, 0.28), (0.22, -0.24)] {
            parent.spawn((
                Mesh3d(meshes.flame.clone()),
                MeshMaterial3d(materials.torch_flame.clone()),
                Transform::from_xyz(dx, surface_y + 0.46, dz),
            ));
        }
    }
    if variant >= 3 {
        parent.spawn((
            Mesh3d(meshes.wall.clone()),
            MeshMaterial3d(materials.settlement_stone.clone()),
            Transform::from_xyz(-0.02, surface_y + 0.22, 0.30)
                .with_rotation(Quat::from_rotation_y(0.34))
                .with_scale(Vec3::new(0.76, 1.0, 1.0)),
        ));
    }
}

fn spawn_tile_connectors(
    commands: &mut Commands,
    layout: &HexLayout,
    tiles: &[BoardTileData],
    bridge_meshes: &BridgeMeshes,
    materials: &FeatureMaterials,
) {
    let lookup: HashMap<Hex, &BoardTileData> = tiles.iter().map(|tile| (tile.coord, tile)).collect();
    let forward_neighbors = [Hex::new(1, 0), Hex::new(0, 1), Hex::new(-1, 1)];

    for tile in tiles {
        for offset in forward_neighbors {
            let neighbor_coord = tile.coord + offset;
            let Some(other) = lookup.get(&neighbor_coord) else {
                continue;
            };
            if other.kind != tile.kind {
                continue;
            }

            let start = layout.hex_to_world_pos(tile.coord);
            let end = layout.hex_to_world_pos(other.coord);
            let midpoint = (start + end) * 0.5;
            let yaw = (end.x - start.x).atan2(end.y - start.y);
            let bridge_y = (tile.surface_height + other.surface_height) * 0.5 + 0.14;
            let lateral = Vec2::new(-(end.y - start.y), end.x - start.x);
            let lateral = if lateral.length_squared() > 0.0 {
                lateral.normalize()
            } else {
                Vec2::ZERO
            };
            let parity = (tile.coord.x + tile.coord.y).rem_euclid(2) as f32;
            let side_offset = (parity - 0.5) * 0.22;

            match tile.kind {
                TileKind::AncientForest => {
                    commands.spawn((
                        Mesh3d(bridge_meshes.root.clone()),
                        MeshMaterial3d(materials.dead_wood.clone()),
                        Transform::from_xyz(
                            midpoint.x + lateral.x * side_offset,
                            bridge_y,
                            midpoint.y + lateral.y * side_offset,
                        )
                            .with_rotation(Quat::from_rotation_y(yaw) * Quat::from_rotation_z(0.12)),
                    ));
                    if parity > 0.0 {
                        commands.spawn((
                            Mesh3d(bridge_meshes.root.clone()),
                            MeshMaterial3d(materials.dead_wood.clone()),
                            Transform::from_xyz(
                                midpoint.x - lateral.x * 0.14,
                                bridge_y - 0.04,
                                midpoint.y - lateral.y * 0.14,
                            )
                            .with_rotation(
                                Quat::from_rotation_y(yaw + 0.24) * Quat::from_rotation_z(-0.08),
                            )
                            .with_scale(Vec3::new(0.72, 0.8, 0.54)),
                        ));
                    }
                }
                TileKind::CorruptedRuins => {
                    if parity > 0.0 {
                        continue;
                    }
                    commands.spawn((
                        Mesh3d(bridge_meshes.path.clone()),
                        MeshMaterial3d(materials.ruin_stone.clone()),
                        Transform::from_xyz(
                            midpoint.x + lateral.x * 0.10,
                            bridge_y - 0.04,
                            midpoint.y + lateral.y * 0.10,
                        )
                        .with_rotation(Quat::from_rotation_y(yaw + 0.16))
                        .with_scale(Vec3::new(0.58, 1.0, 0.58)),
                    ));
                }
                TileKind::Riverbank => {
                    commands.spawn((
                        Mesh3d(bridge_meshes.path.clone()),
                        MeshMaterial3d(materials.water.clone()),
                        Transform::from_xyz(midpoint.x, bridge_y - 0.06, midpoint.y)
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_scale(Vec3::new(0.62, 1.0, 0.90)),
                    ));
                    if parity == 0.0 {
                        commands.spawn((
                            Mesh3d(bridge_meshes.root.clone()),
                            MeshMaterial3d(materials.reed.clone()),
                            Transform::from_xyz(
                                midpoint.x + lateral.x * 0.18,
                                bridge_y + 0.02,
                                midpoint.y + lateral.y * 0.18,
                            )
                            .with_rotation(Quat::from_rotation_y(yaw) * Quat::from_rotation_z(0.20))
                            .with_scale(Vec3::new(0.24, 1.2, 0.18)),
                        ));
                    }
                }
                TileKind::FortifiedSettlement => {
                    if parity > 0.0 {
                        continue;
                    }
                    commands.spawn((
                        Mesh3d(bridge_meshes.path.clone()),
                        MeshMaterial3d(materials.settlement_stone.clone()),
                        Transform::from_xyz(midpoint.x, bridge_y - 0.03, midpoint.y)
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_scale(Vec3::new(0.54, 1.0, 0.54)),
                    ));
                    commands.spawn((
                        Mesh3d(bridge_meshes.wall.clone()),
                        MeshMaterial3d(materials.settlement_stone.clone()),
                        Transform::from_xyz(
                            midpoint.x + lateral.x * 0.10,
                            bridge_y + 0.08,
                            midpoint.y + lateral.y * 0.10,
                        )
                            .with_rotation(Quat::from_rotation_y(yaw))
                            .with_scale(Vec3::new(0.28, 1.0, 0.46)),
                    ));
                }
            }
        }
    }
}

fn update_diorama_camera(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut cameras: Query<(&mut DioramaCamera, &mut Transform), With<Camera3d>>,
) {
    for (mut controller, mut transform) in &mut cameras {
        let dt = time.delta_secs();
        let radius = controller.radius;
        let mut pan = Vec3::ZERO;
        let forward = Vec3::new(-controller.yaw.sin(), 0.0, -controller.yaw.cos()).normalize();
        let right = Vec3::new(forward.z, 0.0, -forward.x);

        if keyboard.pressed(KeyCode::KeyW) {
            pan += forward;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            pan -= forward;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            pan += right;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            pan -= right;
        }

        if pan != Vec3::ZERO {
            controller.focus += pan.normalize() * dt * radius * 0.48;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            controller.yaw += dt * 0.95;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            controller.yaw -= dt * 0.95;
        }
        if keyboard.pressed(KeyCode::KeyR) {
            controller.pitch = (controller.pitch + dt * 0.65).clamp(0.55, 1.20);
        }
        if keyboard.pressed(KeyCode::KeyF) {
            controller.pitch = (controller.pitch - dt * 0.65).clamp(0.55, 1.20);
        }

        if mouse_buttons.pressed(MouseButton::Left) {
            controller.yaw -= mouse_motion.delta.x * 0.006;
            controller.pitch = (controller.pitch - mouse_motion.delta.y * 0.004).clamp(0.55, 1.20);
        }
        if mouse_buttons.pressed(MouseButton::Right) {
            controller.focus +=
                (-right * mouse_motion.delta.x + forward * mouse_motion.delta.y) * radius * 0.010;
        }
        if mouse_scroll.delta.y != 0.0 {
            controller.radius = (controller.radius - mouse_scroll.delta.y * 1.15).clamp(18.0, 48.0);
        }

        *transform = camera_transform(&controller);
    }
}

fn setup_overlay(mut commands: Commands, board: Option<Res<BoardState>>) {
    let tile_count = board.as_ref().map_or(25, |board| board.tiles.len());

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(18.0),
                left: px(18.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.0),
                padding: UiRect::all(px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.03, 0.04, 0.70)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Path of Taxation\nDark Hex Sandbox"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.90, 0.87, 0.82)),
            ));
            parent.spawn((
                Text::new(format!(
                    "{tile_count} authored tiles\nLeft drag: orbit  |  Right drag: pan  |  Scroll: zoom"
                )),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.67, 0.62)),
            ));
            parent.spawn((
                Text::new("WASD pan  |  Q/E rotate  |  R/F pitch"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.67, 0.62)),
            ));
        });
}

fn starter_tiles() -> Vec<BoardTileData> {
    // Generate a larger board (radius 5) with biome zones that blend organically.
    // Center: CorruptedRuins, West: AncientForest, East: FortifiedSettlement,
    // South: Riverbank, with mixing at boundaries.
    let mut tiles = Vec::new();

    for coord in Hex::ZERO.range(5) {
        let q = coord.x();
        let r = coord.y();
        let dist = coord.unsigned_distance_to(Hex::ZERO);

        // Biome selection based on position for organic zones.
        let kind = if q <= -2 {
            // West: deep forest
            TileKind::AncientForest
        } else if q >= 2 && r <= 1 {
            // East: settlement
            TileKind::FortifiedSettlement
        } else if r >= 2 || (r >= 1 && q >= 0) {
            // South/southeast: riverbank/swamp
            if (q + r * 7) % 3 == 0 { TileKind::AncientForest } else { TileKind::Riverbank }
        } else if dist <= 2 {
            // Center: corrupted ruins
            TileKind::CorruptedRuins
        } else {
            // Transition zones: mix based on hash
            let h = (q.wrapping_mul(7919) as u32).wrapping_mul(31) ^ (r.wrapping_mul(104729) as u32);
            match h % 4 {
                0 => TileKind::AncientForest,
                1 => TileKind::CorruptedRuins,
                2 => TileKind::Riverbank,
                _ => TileKind::AncientForest,
            }
        };

        // Height variation: forest is tallest, riverbank lowest, ruins and settlement mid.
        let base_height = match kind {
            TileKind::AncientForest => 0.78,
            TileKind::CorruptedRuins => 0.72,
            TileKind::Riverbank => 0.55,
            TileKind::FortifiedSettlement => 0.70,
        };
        // Add per-tile variation for organic feel.
        let h = ((q.wrapping_mul(7919_i32)) as u32) ^ ((r.wrapping_mul(104729_i32)) as u32);
        let variation = ((h % 20) as f32 - 10.0) * 0.012;
        let surface_height = base_height + variation;

        tiles.push(BoardTileData { coord, kind, surface_height });
    }

    tiles
}

fn camera_transform(controller: &DioramaCamera) -> Transform {
    let horizontal = controller.radius * controller.pitch.cos();
    let translation = controller.focus
        + Vec3::new(
            horizontal * controller.yaw.cos(),
            controller.radius * controller.pitch.sin(),
            horizontal * controller.yaw.sin(),
        );

    Transform::from_translation(translation).looking_at(controller.focus, Vec3::Y)
}

fn tile_material(base_color: Color, perceptual_roughness: f32, metallic: f32) -> StandardMaterial {
    StandardMaterial {
        base_color,
        perceptual_roughness,
        metallic,
        ..default()
    }
}

fn rim_material(base_color: Color, emissive: Color) -> StandardMaterial {
    StandardMaterial {
        base_color,
        emissive: emissive.into(),
        perceptual_roughness: 0.84,
        metallic: 0.04,
        ..default()
    }
}
