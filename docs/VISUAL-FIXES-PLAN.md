# Visual Fixes Plan -- Path of Taxation

**Date:** 2026-04-02
**Status:** In progress

## Priority Order

### Fix 1: BALANCE (S) -- BLOCKS ALL TESTING
- `client/src/components/player.rs`: HP already 300 (done)
- `client/src/components/enemy.rs`: Add `EnemyDamage(pub f32)` component
- `client/src/plugins/enemies.rs`:
  - In `enemy_spawn_system`: add `EnemyDamage(msg.damage)` to spawn bundle
  - In `enemy_attack_hitbox_system`: query `&EnemyDamage`, use its value instead of hardcoded `10.0`
- `client/src/plugins/run.rs`: Change `let radius = 200.0` to `350.0`
- `shared/src/constants.rs`: Change `MAX_ACTIVE_ATTACKERS` from 8 to 4

### Fix 4: CHARACTERS BIGGER (S)
- `client/src/plugins/player.rs`: Change `custom_size` from `Vec2::new(256.0, 256.0)` to `Vec2::new(384.0, 384.0)`
- `client/src/plugins/enemies.rs`: Same change for enemy sprites
- `client/src/plugins/camera.rs`: In `spawn_camera`, set initial ortho scale to 0.85
- `client/src/plugins/ui.rs`: Adjust enemy health bar y-offset from +20 to +35

### Fix 7: RED FLASH TONED DOWN (S)
- `client/src/plugins/combat.rs`: Change player-hit ScreenFlashMsg alpha from 0.8 to 0.3
- `client/src/plugins/vfx.rs`: Change screen flash start_alpha from 0.8 to 0.35

### Fix 6: UI TEXT LAYERING (S)
- `client/src/plugins/ui.rs`: In `death_screen_system`, despawn any RoomClearUI entities before spawning death screen
- `client/src/plugins/run.rs`: Add ZIndex(10) to room clear UI root node

### Fix 2: OUTDOOR TILES (L)
- Copy OFDN dark tiles 01,03,05 to `assets/sprites/tiles/ground_dark_01.png` etc
- Copy dark-ruins pieces to `assets/sprites/tiles/ruin_wall_01.png` etc
- Copy rock sprites from more-isometric-parts to `assets/sprites/tiles/rock_01.png` etc
- `client/src/plugins/run.rs`: Replace floor_tiles array with OFDN ground tiles + dirt
- Adjust tint to warm muddy-brown (base * 0.85, base * 0.80, base * 0.65)
- Increase tile_display to Vec2::new(140.0, 280.0) for more overlap
- Reduce tile_spacing from 88.0 to 78.0

### Fix 3: BREAK GRID (M)
- `client/src/plugins/run.rs`: Increase jitter multiplier from 2.5 to 6.0
- Increase rotation range from 0.012 to 0.025
- Increase scale variation from 0.01 to 0.03
- Add 30-40 fill tiles at 0.6x scale between main tiles
- Increase ground detail overlays from 45 to 80

### Fix 5: DEPTH LAYERS (L)
- Copy bigtree/shrub sprites to assets/sprites/tiles/
- Spawn 8-12 tall trees at FOREGROUND z-layer (500.0) around arena edge
- Spawn 10-15 depth-sorted rocks WITH WorldPosition + ArenaEntity
- Spawn 12-16 dark background tree silhouettes at BG_FAR z-layer
- Replace colored-rectangle fog with actual fog texture sprites

### Fix 8: COMBAT VFX (M)
- `client/src/plugins/vfx.rs`: Damage number font 22/32 (was 16/24)
- Particle size 5.0 (was 3.0), lifetime 20 frames (was 12)
- Particle speed 120.0 (was 80.0)
- Brighter HDR colors on hit particles
