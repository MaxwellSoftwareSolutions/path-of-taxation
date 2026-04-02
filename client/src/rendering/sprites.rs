use bevy::prelude::*;

/// Holds handles to all sprite textures loaded from disk.
#[derive(Resource)]
pub struct SpriteAssets {
    pub player: Handle<Image>,
    pub enemy_tax_collector: Handle<Image>,
    pub enemy_undead_accountant: Handle<Image>,
    pub floor_stone: Handle<Image>,
    pub floor_wood: Handle<Image>,
    pub floor_dirt: Handle<Image>,
    pub wall: Handle<Image>,
    pub projectile: Handle<Image>,
    pub slash: Handle<Image>,
    pub boundary_marker: Handle<Image>,
    pub torch: Handle<Image>,
}

/// Atlas layout for FLARE-format character spritesheets (8x8 grid of 256x256 cells).
#[derive(Resource)]
pub struct CharacterAtlasLayout {
    pub layout: Handle<TextureAtlasLayout>,
}

pub struct SpriteGenPlugin;

impl Plugin for SpriteGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_all_sprites);
    }
}

fn load_all_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Character spritesheets (FLARE format)
    let player = asset_server.load("sprites/characters/player_with_weapon.png");
    let enemy_tax_collector = asset_server.load("sprites/characters/enemy_skeleton.png");
    let enemy_undead_accountant = asset_server.load("sprites/characters/enemy_zombie.png");

    // Tiles
    let floor_stone = asset_server.load("sprites/tiles/stone_floor.png");
    let floor_wood = asset_server.load("sprites/tiles/dark_tile_01.png");
    let floor_dirt = asset_server.load("sprites/tiles/dirt_floor.png");
    let wall = asset_server.load("sprites/tiles/stone_column.png");
    let boundary_marker = asset_server.load("sprites/tiles/stone_tile.png");

    // Effects
    let projectile = asset_server.load("sprites/effects/arcane_bolt.png");
    let slash = asset_server.load("sprites/effects/fireball.png");
    let torch: Handle<Image> = boundary_marker.clone();

    commands.insert_resource(SpriteAssets {
        player,
        enemy_tax_collector,
        enemy_undead_accountant,
        floor_stone,
        floor_wood,
        floor_dirt,
        wall,
        projectile,
        slash,
        boundary_marker,
        torch,
    });

    // FLARE character atlas: 2048x2048 sheet, 8 cols x 8 rows, 256x256 per cell
    let layout = TextureAtlasLayout::from_grid(UVec2::new(256, 256), 8, 8, None, None);
    let layout_handle = atlas_layouts.add(layout);
    commands.insert_resource(CharacterAtlasLayout {
        layout: layout_handle,
    });
}
