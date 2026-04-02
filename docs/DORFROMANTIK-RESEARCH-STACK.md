# Dorfromantik-Style Build Research

**Research date:** April 2, 2026  
**Goal:** identify existing Bevy-compatible infrastructure and external asset sources that make a restart faster and less fragile.

## Bottom Line

For a Dorfromantik-like restart, the best stack is:

- Bevy 0.18.x 3D
- `hexx` for hex coordinates + hex mesh generation
- Bevy `bevy_picking` / `MeshPickingPlugin` for hover and click on 3D tiles
- `bevy_panorbit_camera` for immediate usable camera controls
- Bevy glTF pipeline for models and scenes
- `bevy_asset_loader` for clean loading states
- `bevy-inspector-egui` for live tuning
- `leafwing-input-manager` for robust input actions
- `leafwing_manifest` or equivalent for tile/biome/content definitions

For art, the strongest starting point is:

- **KayKit Medieval Hexagon Pack** as the base kit
- **KayKit Forest Nature Pack** for environment variation
- **Tiny Treats** for cozy decorative props
- **Quaternius** and **Kenney** as CC0 support libraries
- **Synty** only if you decide to spend money for speed and consistency

## 1. Bevy Infrastructure Worth Adding

## 1. `hexx`

Best fit for this game.

Why:

- built specifically for hex coordinates
- includes coordinate math, neighbors, rings, rotation, layouts
- includes pathfinding / movement / visibility helpers
- includes procedural mesh builders for hex planes, columns, and height maps
- has Bevy support and Bevy examples, including 3D columns and 3D picking

Why it matters here:

- it removes the need to hand-roll the board math
- it gives you a strong domain model for tile rotation and adjacency
- it can generate physical hex columns directly, which fits the diorama target

Recommended use:

- use `hexx` as the canonical board coordinate system from day one
- use its mesh utilities for prototype tiles before importing authored meshes

Sources:

- [hexx GitHub](https://github.com/ManevilleF/hexx)
- [hexx docs.rs](https://docs.rs/crate/hexx/latest)

Notes from source:

- the README says `hexx` can “manipulate hexagon coordinates,” “generate hexagonal maps,” and “generate hexagon meshes”
- the docs show Bevy examples for `3d_columns`, `3d_picking`, `mesh_builder`, and `heightmap_builder`
- current docs.rs metadata shows `hexx 0.24.0` depends on Bevy 0.18 dev crates

## 2. Bevy picking (`bevy_picking` / `MeshPickingPlugin`)

Best fit for hover/select on the new board.

Why:

- it is part of the Bevy 0.18 ecosystem
- Bevy ships a dedicated 3D mesh picking example
- it gives pointer hover/click behavior without writing your own raycast stack first

Why it matters here:

- tile hover, preview placement, and click-to-place are central to the whole game
- this removes one of the easiest places to write brittle custom code

Recommended use:

- use Bevy mesh picking for the first sandbox
- attach hover/select behavior to hex tile entities
- only replace it later if you need a more specialized backend

Sources:

- [Bevy mesh picking example](https://bevy.org/examples-webgpu/picking/mesh-picking/)
- [bevy_picking docs.rs](https://docs.rs/crate/bevy_picking/0.18.1/source/)

Notes from source:

- the Bevy example says `MeshPickingPlugin` is “a simple starting point”
- it also says `bevy::picking` can compose backends together and that `bevy_ui` / `bevy_sprite` picking backends can be enabled at the same time
- docs.rs lists `bevy_picking 0.18.1` as “Provides screen picking functionality for Bevy Engine”

## 3. `bevy_panorbit_camera`

Best short-term camera infrastructure.

Why:

- orbit, pan, and zoom are already solved
- works with orthographic cameras
- good fit for a miniature tabletop / diorama camera
- removes a lot of low-value camera-control work at the start

Why it matters here:

- you want the restart to prove the world look quickly
- camera polish is important, but writing the controller from scratch is not the highest-leverage early task

Recommended use:

- use it for the prototype and first sandbox
- replace or wrap it later only if game-specific camera behavior requires it

Source:

- [bevy_panorbit_camera GitHub](https://github.com/Plonq/bevy_panorbit_camera)

Notes from source:

- the README says it “works with orthographic camera projection”
- the compatibility table lists Bevy `0.18` with `bevy_panorbit_camera 0.34`

## 4. `bevy_asset_loader`

Best loading-state infrastructure.

Why:

- clean loading screens / loading states
- asset collections keep strong handles alive
- good for booting a sandbox with meshes, materials, fonts, and icons

Why it matters here:

- once you move to 3D models, you want deterministic loading instead of scattered `AssetServer::load` calls everywhere

Recommended use:

- use it for startup collections:
  - base tile meshes
  - vegetation models
  - UI fonts/icons
  - scene prefabs

Source:

- [bevy_asset_loader GitHub](https://github.com/NiklasEi/bevy_asset_loader)

Notes from source:

- the compatibility table says Bevy `0.18` works with `bevy_asset_loader 0.25 - 0.26`

## 5. `bevy-inspector-egui`

Best live-debugging / tuning tool.

Why:

- inspect resources, assets, and entities live
- great for camera tuning, tile hover state, board state, and art parameters

Why it matters here:

- a board-builder game needs rapid tuning more than hidden complexity
- being able to inspect tile neighbors, hover state, and biome parameters live will save time immediately

Recommended use:

- use only in dev builds
- inspect:
  - board resource
  - tile queue
  - camera config
  - scoring config
  - selected / hovered tile

Source:

- [bevy-inspector-egui GitHub](https://github.com/jakobhellermann/bevy-inspector-egui)

Notes from source:

- the support table lists Bevy `0.18` with `bevy-inspector-egui 0.36`

## 6. `leafwing-input-manager`

Best input abstraction layer.

Why:

- action-based input
- keyboard + mouse + gamepad support
- easier rebinding and cleaner gameplay code

Why it matters here:

- hover, rotate, place, cancel, pan, and zoom become clean actions instead of ad-hoc device checks

Recommended use:

- define actions like:
  - `PlaceTile`
  - `RotateCW`
  - `RotateCCW`
  - `Pan`
  - `Zoom`
  - `ToggleDebug`

Source:

- [leafwing-input-manager docs.rs](https://docs.rs/crate/leafwing-input-manager/latest)

Notes from source:

- docs.rs metadata for `leafwing-input-manager 0.20.0` shows a dependency on `bevy ^0.18.0-rc.2`

## 7. `leafwing_manifest`

Best content-definition helper if you want data-driven tiles early.

Why:

- maps IDs to in-memory item definitions
- designed to turn “assets on disk” into robust game data

Why it matters here:

- tile archetypes, biome rules, and feature definitions should not live as hardcoded Rust constants if this game is going to grow

Recommended use:

- use manifests for:
  - tile types
  - feature prefabs
  - adjacency rules
  - biome weights
  - scoring rules

Source:

- [leafwing_manifest docs.rs](https://docs.rs/leafwing_manifest/latest/leafwing_manifest/)

Notes from source:

- docs say it transforms “assets on disk” into flexible, robust objects in a Bevy game
- compatibility table lists Bevy `0.18` with `leafwing_manifest 0.6`

## 8. Built-in Bevy 3D/glTF pipeline

Use this, do not fight it.

Why:

- Bevy calls glTF its primary 3D format
- official examples already cover loading scenes, environment maps, and shadowed lighting
- Bevy 0.18 improved glTF extension handling

Why it matters here:

- for a stylized 3D board game, glTF/GLB is the right transport format from Blender / Blockbench into Bevy
- it keeps hierarchy, materials, and scene structure intact

Recommended use:

- standardize on `.glb` for imported assets
- later, use Blender custom properties or glTF extras/extensions for tile metadata if needed

Sources:

- [Bevy load_gltf example](https://bevy.org/examples/3d-rendering/load-gltf/)
- [Bevy 0.18 release notes](https://bevy.org/news/bevy-0-18/)

Notes from source:

- Bevy’s load example uses `SceneRoot`, `EnvironmentMapLight`, `DirectionalLight`, and shadow configuration
- Bevy 0.18 says glTF “serves as Bevy’s primary 3D format”

## 9. Optional later: `bevy_hanabi`

Only for later polish.

Why:

- placement dust, little score bursts, subtle water sparkles

Why not now:

- the restart does not need particles to prove itself
- environment shape, lighting, and board readability matter more first

Source:

- [bevy_hanabi docs.rs](https://docs.rs/crate/bevy_hanabi/latest)

Notes from source:

- compatibility table lists `bevy_hanabi 0.18` with Bevy `0.18`

## 2. Infrastructure To Avoid Right Now

## `bevy_editor_pls`

Do not plan the restart around this.

Why:

- its README support table only goes up to Bevy `0.14`
- the current repo is on Bevy `0.18`

Source:

- [bevy_editor_pls GitHub](https://github.com/jakobhellermann/bevy_editor_pls)

Inference:

- unless you want to vendor/fork and update it, this is not a safe core dependency for the restart

## Custom camera controls from scratch

Avoid at first.

Why:

- low leverage
- easy to burn time on “feel” while the environment still lacks a board and asset pipeline

## Physics engines for tile placement

Avoid for the first milestone.

Why:

- Dorfromantik-like placement is discrete, not physics-driven
- board rules and picking matter more than rigidbodies

## 3. Asset Research

## Best direct fit: KayKit Medieval Hexagon Pack

This is the strongest base pack for the new target.

Why:

- already hexagonal
- already stylized
- already built for cozy builders / RTS / top-down strategy
- includes tiles, rivers, lakes, coasts, buildings, trees, rocks, hills, mountains, and clouds
- glTF included
- CC0

Source:

- [KayKit Medieval Hexagon Pack](https://kaylousberg.com/game-assets/medieval-hexagon)

Notes from source:

- the page says it has “over 200 stylised medieval hexagonal tiles, buildings, and props”
- it explicitly calls out “roads, rivers, oceans/lakes, and coasts”
- it says it is “free for personal and commercial use, no attribution required. (CC0 Licensed)”
- it includes `.GLTF`, `.FBX`, and `.OBJ`

This is the pack I would test first.

## Best environment expansion: KayKit Forest Nature Pack

Best companion pack.

Why:

- same family / compatible style
- huge vegetation library
- can fill the board without style mismatch

Source:

- [KayKit Forest Nature Pack](https://kaylousberg.com/game-assets/forest-nature-pack)

Notes from source:

- the page says it has `200+ unique` stylised forest assets and `1500+ total models including recolours`
- it includes trees, bushes, rocks, grass, and terrain
- it is CC0 and includes `.GLTF`

## Best cozy support pack: Tiny Treats

Best for warmth and charm.

Why:

- designed to fit KayKit’s scale and technical specs
- stronger “cute/cozy” tone than most general low-poly packs
- good for micro-props and scene dressing

Sources:

- [Tiny Treats overview](https://tinytreats.itch.io/)
- [Tiny Treats House Plants](https://tinytreats.itch.io/house-plants)

Notes from source:

- Tiny Treats says its packs are designed to match KayKit technically
- House Plants says the models come in `.OBJ`, `.FBX`, and `.GLTF`
- it also says they are “free for personal and commercial use, no attribution required. (CC0 Licensed)”

## Best free support libraries: Quaternius and Kenney

Use these as support libraries, not the main style anchor.

### Quaternius

Good for:

- low-poly nature fillers
- modular props
- backup buildings or decorations

Source:

- [Quaternius Ultimate Stylized Nature Pack](https://quaternius.com/packs/ultimatestylizednature.html)

Notes from source:

- the pack includes `60+` nature assets
- it includes `FBX`, `OBJ`, `glTF`, and `Blend`
- the page says it is free for personal and commercial projects

### Kenney

Good for:

- prototyping
- broad free model coverage
- quick filler assets

Source:

- [Kenney Nature Kit](https://kenney.nl/assets/nature-kit)

Notes from source:

- the page lists `330×` files
- the license is `Creative Commons CC0`

## Poly Pizza

Useful, but only with discipline.

Why:

- lots of low-poly models
- direct GLTF download

Risk:

- license mixes are common
- many bundles include both CC0 and CC-BY models

Source:

- [Poly Pizza City Pack example](https://poly.pizza/bundle/City-Pack-q11onRvPoJ)

Notes from source:

- the bundle page explicitly lists both `Public Domain (CC0)` and `Creative Commons Attribution`
- individual items in the same bundle are mixed, including Quaternius CC0 items and CC-BY items

Recommendation:

- only use Poly Pizza if you either:
  - filter to CC0 only, or
  - keep a proper attribution manifest in the repo

## Paid fast-track option: Synty

Best if you want speed and are willing to pay.

Why:

- high consistency
- strong pack breadth
- proven low-poly commercial pipeline

Sources:

- [Synty main site](https://www.syntystudios.com/)
- [Synty licensing overview](https://syntystore.com/pages/licences-overview)
- [Synty one-time purchase licence](https://syntystore.com/pages/one-time-purchase-licence)

Notes from source:

- Synty positions itself as a large commercial low-poly asset library
- the licensing overview distinguishes one-time purchase vs subscription
- the one-time purchase licence explicitly allows incorporating assets into commercial products under the licence terms
- Synty also has separate restrictions around generative AI / NFT / metaverse use

Recommendation:

- if budget exists and you want fast consistent output, Synty is the cleanest commercial option
- if budget is low, KayKit + Tiny Treats + Quaternius is the better fit

## 4. Modeling Tools Research

## Blender

Best all-around production tool.

Why:

- strongest cleanup, kitbashing, material, and scene-authoring option
- ideal for merging packs into one coherent kit
- best place to make final edits before export

Source:

- [Blender pipeline page](https://www.blender.org/features/pipeline/)

## Blockbench

Best fast tool for simple stylized low-poly props.

Why:

- lower barrier than Blender
- good for making custom tile props quickly
- exports glTF

Sources:

- [Blockbench 3D export guide](https://www.blockbench.net/wiki/guides/export-formats/)
- [Blockbench rendering guide](https://www.blockbench.net/wiki/guides/model-rendering/)

Notes from source:

- the export guide recommends `glTF`/`glb` as a modern format with good hierarchy and animation support
- the rendering guide explicitly says to export as glTF via `File > Export > Export as glTF`

## 5. Recommended Art Pipeline

## Art pipeline choice

Use:

- **GLB/GLTF as the only runtime import format**
- Blender as the cleanup / kitbash / export tool
- Blockbench only for fast simple custom props

Do not use:

- FBX as the main runtime path
- mixed raw formats directly in Bevy assets
- AI-generated 3D as the main production asset source

Inference from sources:

- Bevy’s official 3D examples and release notes make glTF the safest long-term format
- using one runtime format will reduce pipeline friction and import bugs

## 6. Practical Plan

## Plan A: Fastest high-quality free route

1. Base the board on KayKit Medieval Hexagon.
2. Add Forest Nature Pack for vegetation.
3. Add Tiny Treats only for cozy scene dressing.
4. Import everything as `.glb`.
5. Build the new client around `hexx` + `bevy_panorbit_camera`.

This is the plan I recommend.

## Plan B: Highest consistency with budget

1. Buy or subscribe to Synty.
2. Pick one narrow theme and stick to it.
3. Use Blender to convert/export only the required assets as `.glb`.

This is faster if budget matters less than time.

## Plan C: Mixed free marketplace route

1. Use KayKit as the anchor style.
2. Fill gaps with Quaternius and Kenney.
3. Use Poly Pizza only when the model is clearly CC0 or attribution is acceptable.
4. Normalize scale/materials in Blender before importing.

This works, but requires stronger art discipline.

## 7. Recommended Next Implementation Step

If you continue the restart now, the stack should be:

- `hexx`
- Bevy mesh picking
- `bevy_panorbit_camera`
- `bevy_asset_loader`
- `bevy-inspector-egui`
- `leafwing-input-manager`
- `leafwing_manifest`

And the first asset target should be:

- **KayKit Medieval Hexagon Pack**

Then build this exact milestone:

1. Load one hex tile as a real 3D scene or mesh.
2. Spawn a 7-19 tile starter board.
3. Add orbit/pan/zoom camera.
4. Add hover highlighting.
5. Add preview placement for the next tile.

Only after that should you add score rules and content breadth.
