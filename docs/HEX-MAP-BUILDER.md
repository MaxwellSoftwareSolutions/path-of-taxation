# Hex Map Builder -- Design

## Why hexagons

Diamonds (isometric rhombus) tiles have 4 corners where 4 tiles meet -- these create visible cross-shaped seam artifacts that are nearly impossible to hide. Hexagons have only 3 tiles meeting at each vertex, and each edge is shared by exactly 2 tiles. This makes seams far less visible and blending much easier.

## Approach

### 1. Hex tile preparation

Take the 9 existing terrain tile textures and mask them into hex shapes:
- Each hex tile is a regular hexagon (pointy-top or flat-top)
- The terrain art fills the hex, edges are alpha-feathered (5-10px soft blend)
- Output: 9 hex-shaped PNGs at consistent size (e.g., 256x222 for pointy-top)

### 2. Connection rules

Each hex has 6 edges. Define which terrain types can be adjacent:

```
COMPATIBLE = {
    "muddy_dirt":     ["muddy_dirt", "forest_floor", "muddy_puddle", "tree_roots", "gravel_stones"],
    "stone_path":     ["stone_path", "cracked_ruins", "gravel_stones", "muddy_dirt"],
    "forest_floor":   ["forest_floor", "muddy_dirt", "moss_grass", "tree_roots"],
    "muddy_puddle":   ["muddy_puddle", "muddy_dirt", "moss_grass"],
    "cracked_ruins":  ["cracked_ruins", "stone_path", "gravel_stones", "rubble"],
    "gravel_stones":  ["gravel_stones", "stone_path", "cracked_ruins", "muddy_dirt", "rubble"],
    "moss_grass":     ["moss_grass", "forest_floor", "muddy_puddle", "tree_roots"],
    "tree_roots":     ["tree_roots", "forest_floor", "moss_grass", "muddy_dirt"],
    "rubble":         ["rubble", "cracked_ruins", "gravel_stones", "stone_path"],
}
```

### 3. Map generation algorithm

Use **Wave Function Collapse (WFC) lite** or simple constraint propagation:

1. Create a hex grid (e.g., radius 12 = ~400 hexes)
2. Start from center -- pick a random terrain type (e.g., stone_path for an ancient road)
3. For each unplaced neighbor, filter to compatible types based on already-placed adjacent hexes
4. Pick randomly from valid options (weighted by biome preferences)
5. Repeat until all hexes are placed
6. If stuck (no valid options), backtrack

### 4. Biome zones

Control the overall feel with biome weights per region:

- **Center (radius 0-3)**: stone_path 40%, cracked_ruins 30%, gravel 20%, dirt 10%
- **Mid (radius 4-7)**: forest_floor 30%, muddy_dirt 25%, tree_roots 20%, moss 15%, puddle 10%
- **Edge (radius 8+)**: moss_grass 30%, tree_roots 25%, forest_floor 20%, muddy_dirt 25%

This creates a natural transition: ancient stone road in center -> forest around it -> overgrown edges.

### 5. Output options

**Option A: Pre-baked image**
- Render all hexes onto a single large PNG (e.g., 4096x4096)
- Load in Bevy as one sprite
- Pros: zero seams, simple rendering
- Cons: fixed size, no runtime variety

**Option B: Hex placement data**
- Output a JSON/RON file with hex positions + tile type
- Bevy loads and spawns individual hex sprites
- Pros: runtime variety, can generate per room
- Cons: more sprites to manage (but hex seams are much less visible)

**Option C: Hybrid**
- Generate the hex layout
- Pre-render to a single image
- Load as one sprite
- Generate different images per room type

Recommended: **Option C** -- best of both worlds.

### 6. Hex coordinate system

Use **axial coordinates** (q, r) which map cleanly to pixel positions:

**Pointy-top hexagons:**
```
pixel_x = hex_size * (sqrt(3) * q + sqrt(3)/2 * r)
pixel_y = hex_size * (3/2 * r)
```

Then apply isometric projection:
```
screen_x = pixel_x - pixel_y  (or just use pixel coords directly since hexes handle the look)
```

Actually -- for an isometric game, we might want **flat-top hexagons** rotated to look isometric, or we can use pointy-top hexes and skip the isometric projection entirely (hexes already look good top-down with a slight perspective tilt built into the art).

### 7. Prop placement

After the base hex grid:
- Place rocks at random positions within hex cells (1-3 per "rubble" or "gravel" hex)
- Place dead trees at hex cells near the edge
- Place ruined pillars at "cracked_ruins" hex cells (20% chance)
- Place fog clouds floating above the terrain

### 8. Tool structure

```
tools/
  hex_map_builder/
    __init__.py
    config.py          # hex size, grid radius, biome rules
    tile_prep.py        # cut source tiles into hex shapes
    hex_grid.py         # hex coordinate math
    map_generator.py    # WFC-lite placement algorithm  
    renderer.py         # render hex grid to single PNG
    main.py             # CLI entry point
```

Usage:
```bash
python -m tools.hex_map_builder --radius 12 --biome forest_ruins --output assets/sprites/terrain/map_01.png
```

## Implementation order

1. `tile_prep.py` -- mask tiles into hex shapes with feathered edges
2. `hex_grid.py` -- coordinate math, neighbor lookup
3. `map_generator.py` -- constraint-based placement
4. `renderer.py` -- compose hexes into final image
5. `main.py` -- CLI
6. Test: generate an image, look at it, iterate
7. Wire into Bevy: load the generated image as ground sprite
