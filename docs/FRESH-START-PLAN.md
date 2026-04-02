# Path of Taxation -- Fresh Start Plan

**Date:** 2026-04-02
**Lesson learned:** Environment first. Don't add gameplay until the world looks right.

## What we keep
- Game agent testing framework (`game_agent/`)
- Test script (`test_game.sh`)
- Custom terrain tile art (`assets/sprites/terrain/` -- 9 tiles + props from AI)
- FLARE character spritesheets (`assets/sprites/characters/`)
- All planning docs
- Shared crate types and definitions
- Content RON files

## What failed and why

**Diamond tile grid approach:**
- Hard-edged isometric diamond sprites placed on a regular grid
- No matter how much overlap, jitter, rotation, or overlay -- the grid is always visible
- The tile edges are opaque and create visible seam lines
- This is a fundamental limitation of the approach, not a tuning problem

## New approach: Single large ground texture

Instead of placing hundreds of small diamond tiles, we generate a single large terrain texture and display it as one sprite. This eliminates ALL tile seams.

### How it works:

1. **Pre-bake a terrain image** using Python + PIL:
   - Start with a 2048x2048 (or larger) blank canvas
   - Paint the 9 terrain tiles onto it at random positions with alpha blending
   - Use Perlin/Simplex noise to decide which tile type goes where
   - Blend tile edges with feathered alpha so there are NO hard seams
   - Save as a single PNG

2. **Load it in Bevy as one sprite:**
   - One `Sprite` entity with `custom_size` covering the whole arena
   - No grid, no seams, no tile management
   - Just one big ground texture

3. **Layer props on top:**
   - Rocks, pillars, dead trees placed individually
   - These are already working fine as individual sprites

4. **Fog and atmosphere on top of that:**
   - Already working (fog drift, dead tree silhouettes)

### Advantages:
- Zero visible seams -- it's one image
- Full control over blending between terrain types
- Can create natural-looking patches of mud, stone, moss, etc.
- Much simpler rendering code (1 sprite vs 200+ tiles)
- Can generate different terrain textures per room/zone

### Implementation steps:

**Step 1: Python terrain generator** (`tools/generate_terrain.py`)
- Takes the 9 tile PNGs as input
- Uses noise-based placement to decide tile type per region
- Paints tiles with feathered edges (gaussian blur on alpha) so they blend
- Outputs a single large PNG (e.g., 3072x1536 for a wide isometric view)
- Can generate multiple variants for different rooms

**Step 2: Minimal Bevy scene**
- Clear color: near-black
- One ground sprite (the generated texture) at z=-350
- One player sprite at z=0 (depth sorted)
- Camera follow
- WASD movement
- NOTHING ELSE until this looks right

**Step 3: Visual QA**
- Screenshot after each change
- Compare to reference (PoE2 Act 1 screenshots)
- Only proceed when the ground reads as "natural terrain" not "tiled grid"

**Step 4: Add props**
- Rocks, dead trees, ruined pillars on top of ground
- Depth-sorted so player walks behind tall objects
- Fog clouds drifting

**Step 5: Add player combat**
- Only after Steps 1-4 look good
- One enemy type
- One attack
- Verify it feels right

**Step 6: Re-enable all existing systems**
- The 10+ phases of gameplay code still exist and compile
- Just need to re-enable them in the plugin registration
- They don't need to be rewritten, just reconnected

## Success criteria for the environment

Before adding ANY gameplay, these must be true:
1. Take a screenshot of just terrain + player
2. Show it to someone who hasn't seen the project
3. They should NOT say "that's a tile grid"
4. They should say "that looks like dark fantasy terrain"

## File changes needed

### New files:
- `tools/generate_terrain.py` -- terrain texture generator
- `assets/sprites/terrain/generated_ground.png` -- output

### Modified files:
- `client/src/plugins/run.rs` -- replace entire terrain generation with single sprite
- `client/src/main.rs` -- revert boot flow to Menu (remove TEMP hack)
- `client/src/plugins/ui.rs` -- re-enable HUD (remove TEMP disable)

### Files that stay as-is:
- All gameplay systems (combat, enemies, boss, loot, hub, pause, etc.)
- All content files (RON)
- All shared types
- Game agent framework
