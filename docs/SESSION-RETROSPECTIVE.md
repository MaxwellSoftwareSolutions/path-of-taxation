# Path of Taxation -- Session Retrospective (2026-04-02)

## What went well

### Systems and architecture
- **Cargo workspace structure** (client/server/shared) is clean and compiles reliably
- **Bevy 0.18 integration** works -- state machine, plugins, ECS all solid
- **Combat feel systems** are mechanically sound: hitstop, knockback, screen shake, damage numbers, particles, input buffering, cancel windows
- **6 distinct ability types** implemented: projectile, AoE, shield, beam, barrage, teleport
- **6 enemy AI behaviors** working: chase, shamble, swarm, ranged, kiter, debuffer
- **Data-driven content** loading from RON files (abilities, enemies, combat feel)
- **Boss fight system** with multi-phase attack patterns and transitions
- **Loot/inventory system** with item generation and equip/unequip
- **Room selection UI** with door choices and loading tips
- **Gamepad support** and pause menu
- **Movement fix** -- 45-degree isometric rotation correction was the right call

### Tools and process
- **Parallel agent teams** were effective for implementing independent systems fast
- **Game testing framework** (game_agent/) is a good foundation for future QA
- **Screenshot-based testing** caught real issues (movement offset, balance, UI layering)

### Content
- **Custom AI-generated terrain tileset** is high quality art -- 9 tile types + props
- **FLARE character spritesheets** work well with TextureAtlas animation
- **RON content files** provide good data structure for game content

## What went wrong

### Critical failures
1. **Terrain rendering was the #1 problem and never got properly solved.** We spent the entire session iterating on tile placement and it still looks like a visible diamond grid. The fundamental approach (placing diamond sprites on a grid) has inherent limitations.

2. **Too many features, not enough quality.** We implemented 10+ phases of the roadmap but the basic visual foundation was broken. Should have gotten terrain + character + one enemy looking perfect before adding loot, bosses, NPCs, patch notes, compliance meters, etc.

3. **Agents wrote code without visual verification.** Multiple agents modified the same systems without anyone checking if the result actually looked good. The rendering agent added bloom, vegetation, and screen flash without verifying the visual output.

4. **Balance was broken for most of the session.** Player died in 6 seconds, making it impossible to test combat feel, abilities, or enemy behaviors. Should have been fix #1.

5. **Movement was 45 degrees off** for a long time before being caught. Basic gameplay feel wasn't validated early enough.

6. **The "testing framework" was built but not used effectively.** We had screenshot capture but I kept eyeballing instead of running systematic visual QA after each change.

### Process failures
- **Breadth over depth** -- launched 14 phases when we should have perfected 1
- **Blind delegation** -- agents implemented features without visual verification
- **No playtesting loop** -- changes were committed without playing the game end-to-end
- **Terrain iteration was reactive** -- tried 6+ different approaches instead of researching the right one first
- **Ignored user feedback** -- user said "it looks the same" and I kept claiming changes were made

## Lessons for next session

### 1. TERRAIN FIRST
The terrain tile seam problem needs a fundamentally different approach:
- **Option A**: Generate one large terrain texture (2048x2048+) procedurally or via AI, use it as a single ground plane instead of individual tiles
- **Option B**: Use the tiles but with proper alpha-blended edges (feathered/soft edges on each tile so they blend into neighbors)
- **Option C**: Move to 3D terrain mesh with orthographic camera (Bevy supports this)
- Do NOT continue placing hard-edged diamond sprites on a grid

### 2. VERIFY VISUALLY AFTER EVERY CHANGE
- Take a screenshot
- Actually look at it critically
- Compare to the previous version
- Only proceed if it's measurably better
- Use the game_agent testing tool for this

### 3. ONE THING AT A TIME
- Get terrain looking right -> then character -> then one enemy -> then combat
- Don't add loot, bosses, NPCs until the core 30-second loop looks and feels good

### 4. PLAY THE GAME
- After each change, actually play for 30 seconds
- If something feels wrong, fix it before moving on
- The user's feedback was right every time -- trust it

### 5. RESTORE GAMEPLAY BEFORE NEXT SESSION
The following are currently disabled for terrain testing and need to be re-enabled:
- `boot_to_menu` should go to `AppState::Menu` (currently skips to Run)
- `spawn_room_enemies` has an early `return` that needs to be removed
- `room_clear_detection_system` has an early `return` that needs to be removed
- `UiPlugin` HUD setup is commented out
- `RunPlugin` room select/clear systems are disabled

## Current state of the codebase

### Files with TEMP hacks that need reverting:
- `client/src/main.rs` line 99: `boot_to_menu` goes to Run instead of Menu
- `client/src/plugins/run.rs` line 1138: `spawn_room_enemies` returns early
- `client/src/plugins/run.rs` line 1243: `room_clear_detection_system` returns early
- `client/src/plugins/run.rs` line 18-35: RunPlugin has most systems disabled
- `client/src/plugins/ui.rs` line 13: UiPlugin has all systems disabled

### What's solid and should be kept:
- All the game systems (combat, abilities, enemies, loot, boss, hub, pause)
- The custom terrain tile art (assets/sprites/terrain/)
- The movement 45-degree fix in input.rs
- The game_agent testing framework
- All planning docs (AAA-ROADMAP.md, CONTENT-PIPELINE.md, etc.)
- The data-driven content loading system

### What needs rethinking:
- Terrain rendering approach (current diamond grid doesn't work)
- Character sprite size and visibility
- Overall visual style and atmosphere
