# Path of Taxation -- Dorfromantik-Style Restart Plan

**Date:** 2026-04-02
**Decision:** Start over at the client/environment layer. Keep the workspace, but stop building on the current 2D isometric combat scene.

## Goal

Build a new environment-first foundation that feels visually closer to **Dorfromantik**:

- miniature diorama world
- hex-based terrain
- soft stylized low-poly look
- readable, calm composition
- clean camera and placement rules

This is a **hard pivot** from the current prototype.

## What This Means

If the target is Dorfromantik-like, then these assumptions must be dropped:

- no diamond-tile isometric ground
- no pixel-art-first rendering strategy
- no dark-fantasy ARPG environment as the baseline
- no combat-led development order

The current client is built around a 2D sprite/isometric pipeline. Dorfromantik-like presentation is much better served by a **3D hex-diorama renderer with an orthographic camera**.

## Keep / Archive / Discard

### Keep

- workspace structure: [Cargo.toml](/home/hex/path-of-taxation/Cargo.toml)
- Bevy client crate: [client/Cargo.toml](/home/hex/path-of-taxation/client/Cargo.toml)
- automation and screenshots:
  - [test_game.sh](/home/hex/path-of-taxation/test_game.sh)
  - [game_agent/README.md](/home/hex/path-of-taxation/game_agent/README.md)
- docs as historical context

### Archive But Do Not Build On

- current 2D gameplay plugins under [client/src/plugins](/home/hex/path-of-taxation/client/src/plugins)
- current 2D rendering code under [client/src/rendering](/home/hex/path-of-taxation/client/src/rendering)
- current combat/player/enemy component model under [client/src/components](/home/hex/path-of-taxation/client/src/components)

### Discard As Core Direction

- sprite-stacked floor generation
- diamond/isometric tile world
- current ARPG combat as the first milestone
- existing terrain texture strategy from [docs/FRESH-START-PLAN.md](/home/hex/path-of-taxation/docs/FRESH-START-PLAN.md)

That older plan is fine for hiding seams in a 2D ground sheet. It is the wrong plan for a Dorfromantik-style target.

## Target Technical Direction

### Rendering Model

Use **Bevy 3D** with:

- orthographic camera
- PBR materials
- actual hex prism meshes
- simple directional light + ambient light
- soft shadowing where affordable

Do **not** fake this with a single 2D baked terrain sprite. Dorfromantik reads as physical tabletop volume, not a flat sheet.

### Core World Representation

The world should be built from:

- axial hex coordinates `(q, r)`
- tile definitions
- tile stack / elevation value
- edge compatibility rules
- optional feature payloads like trees, houses, rail, river, field

### Art Direction

Aim for:

- warm, readable palette
- low noise
- soft edge transitions
- chunky silhouettes
- toy-like scale

Avoid:

- noisy phototextures
- pixel-art dithering
- grimdark contrast
- heavy VFX
- dense UI

## The First Real Product

Do not rebuild the whole game first.

The first success state is:

**A small Dorfromantik-like sandbox with beautiful hex terrain, hover feedback, tile placement, and one satisfying environment loop.**

No combat is required for the first milestone.

## Vertical Slice Definition

The first vertical slice should contain exactly this:

- a fixed-size hex board
- 5 to 7 terrain tile archetypes
- tile hovering and selection
- preview placement of the next tile
- adjacency validation
- basic scoring feedback
- one decorative biome look
- smooth camera movement

If this does not look good in screenshots, do not add gameplay systems.

## Architecture For The Restart

Replace the current client scene graph with these modules:

### App Layer

- boot
- loading
- main sandbox state

### World Layer

- hex coordinates
- tile definitions
- board storage
- tile placement rules
- board generation helpers

### Rendering Layer

- hex mesh generation
- terrain materials
- feature prop spawning
- lighting rig
- outline / hover highlight

### Camera Layer

- orthographic framing
- zoom limits
- pan controls
- smooth follow to board center or pointer focus

### Input Layer

- hover raycast
- click selection
- rotate tile input
- place / cancel input

### Game Layer

- next-tile queue
- scoring
- placement validation
- biome progression

### Debug Layer

- show hex coordinates
- show adjacency state
- show invalid edges
- screenshot hotkey

## Proposed New Client Layout

The cleanest path is to stop extending the current files and add a new layout like this:

```text
client/src/
  main.rs
  app/
    mod.rs
    state.rs
  camera/
    mod.rs
    rig.rs
  input/
    mod.rs
    pointer.rs
  world/
    mod.rs
    coords.rs
    tile.rs
    board.rs
    generation.rs
    placement.rs
  render/
    mod.rs
    hex_mesh.rs
    materials.rs
    terrain.rs
    features.rs
    highlight.rs
  game/
    mod.rs
    queue.rs
    scoring.rs
  debug/
    mod.rs
```

Do not try to preserve the old plugin file boundaries. They reflect the wrong game.

## Phase Plan

## Phase 0 -- Hard Reset

Goal: clear the path.

Tasks:

- reduce [client/src/main.rs](/home/hex/path-of-taxation/client/src/main.rs) to a minimal bootstrap
- stop registering the old combat/player/enemy/hub plugins
- keep old code in place temporarily, but make it unreachable
- create a fresh plugin stack for the sandbox

Done when:

- the client launches into an empty 3D scene
- there is one camera, one light, and no legacy systems running

## Phase 1 -- Hex Board Foundation

Goal: make a correct board model.

Tasks:

- implement axial hex coordinates
- implement neighbor lookup
- implement board storage keyed by coordinate
- implement elevation per tile
- implement tile rotation
- define tile archetypes

Minimum tile archetypes:

- grass
- forest
- wheat field
- village
- water
- rail

Done when:

- the code can place and query tiles deterministically
- neighbor and rotation rules are correct
- board state is serializable

## Phase 2 -- Hex Rendering

Goal: make one placed tile look excellent.

Tasks:

- generate a hex prism mesh
- create top + side materials
- add subtle bevel feeling through geometry or shading
- spawn one tile with elevation
- add a clean light rig
- add ambient background and fog color

Camera target:

- orthographic
- downward tilt roughly 35-45 degrees
- slight diagonal yaw
- fixed pleasing framing before adding controls

Done when:

- a screenshot of one tile already reads as polished and intentional

## Phase 3 -- Board Composition

Goal: a small world, not a single tile.

Tasks:

- spawn a starter cluster of 20-40 tiles
- vary elevation slightly
- add water depressions and raised land
- add simple feature prefabs:
  - tree clumps
  - houses
  - wheat patches
  - rail segments
- ensure features align to tile type and rotation

Done when:

- the board looks like a miniature world from a static screenshot

## Phase 4 -- Hover, Select, Place

Goal: the game starts to feel interactive.

Tasks:

- raycast or otherwise map cursor to hovered hex
- highlight hovered tile
- show the next tile as a ghost preview
- rotate preview with keyboard or mouse wheel
- click to place if valid
- reject placement cleanly if invalid

Done when:

- a player can place tiles without reading debug instructions

## Phase 5 -- Rules And Score Feedback

Goal: the sandbox becomes a game loop.

Tasks:

- implement adjacency scoring
- connect matching edges
- add score popups or counters
- add tile queue
- add limited inventory or turn count

Done when:

- the player has a reason to care about placement quality

## Phase 6 -- Visual Polish

Goal: make it feel close to the reference mood.

Tasks:

- soften materials
- improve sky/background composition
- add subtle floating-camera drift only if it helps
- add prop variation
- add shoreline / river polish
- add tasteful animation:
  - tree sway
  - water shimmer
  - hover pulse

Done when:

- screenshots are consistently attractive at any moment during normal play

## Systems Rules

These rules should constrain every new agent working on this restart:

1. Environment quality comes before gameplay complexity.
2. No combat systems are allowed into the new build before the board looks good.
3. All world logic must be hex-coordinate driven, not screen-position driven.
4. Tile definitions must be data-driven.
5. Rendering decisions should serve readability first, not realism.
6. Avoid temporary hacks that assume the old 2D client architecture.

## Asset Strategy

Do not start by generating hundreds of assets.

Start with a tiny reusable kit:

- one hex tile mesh
- 6 material variants
- 4-6 prop families
- 1 clean lighting rig

If AI-assisted assets are used later, use them for:

- concept exploration
- prop silhouette variants
- texture palette studies

Do not use AI assets to cover for missing shape language or bad lighting.

## What To Reuse From The Current Repo

Reusable:

- workspace and build setup
- Bevy dependency setup
- screenshot/test harness
- content file approach as a concept

Not reusable as-is:

- current input model
- current rendering model
- current player/combat/enemy systems
- current room/run structure

## Acceptance Criteria For The Restart

The restart is on the right track only if these are true:

1. A screenshot of the sandbox clearly resembles a stylized hex-diorama game.
2. A placed tile feels like a physical piece in a miniature world.
3. Hover and placement are legible instantly.
4. The codebase no longer depends on the current 2D isometric assumptions.

If any of those are false, do not move on to more systems.

## Immediate Next Step

The next implementation task should be:

**Phase 0 + Phase 1 only**

Concrete output:

- new minimal plugin stack
- empty 3D scene
- hex coordinate module
- board resource
- one spawned test hex tile

That is the correct place to restart. Not combat. Not AI. Not content breadth.
