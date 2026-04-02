# Architecture Plan

## Purpose

This document defines the target architecture for `Path of Taxation` as a game that can grow without becoming brittle, inconsistent, or full of one-off feature code.

The immediate goal is not maximum abstraction. The immediate goal is a structure that:

- keeps combat iteration fast
- keeps content data-driven
- allows new mechanics without rewriting existing systems
- keeps rules readable in one place
- supports deterministic playtests and internal tooling

The current code is a prototype. It proves that the repo can render a room, move a character, spawn enemies, and run a minimal combat loop. It is not yet organized for long-term extension.

## Architectural Diagnosis

### What is good in the current code

- The code is still small enough to rework without high cost.
- The game is already split into broad concerns: `world`, `player`, `combat`, `ui`, `debug`, `state`, `art`.
- There is already a local playtest harness and debug instrumentation.
- The current prototype uses ECS in a direct, understandable way.

### What is structurally wrong today

1. State is too screen-driven.

`ScreenState` currently drives almost everything: world spawning, player spawning, combat setup, UI, transitions. That is acceptable for a prototype, but it will break down once the run itself contains multiple rooms, encounters, events, and bosses.

2. Systems own too much policy.

Examples:

- `combat.rs` owns spawn layout, enemy behavior, hit rules, run victory, and failure flow.
- `world.rs` owns rendering transforms, arena collision data, depth sorting, and room art placement.
- `player.rs` owns input mapping, movement model, dash rules, invulnerability, and visual sync.

This makes iteration possible now, but it means a single change in mechanics will spread through unrelated code.

3. Data is code-only.

Enemy stats, spawn positions, room layout, cooldowns, health values, and attack rules are hardcoded in systems. That will cause content growth to become expensive and inconsistent.

4. Presentation and gameplay are mixed.

Logical position, render transform, sprite tint, and collision are intertwined inside the same systems. That blocks future improvements like animation, effects layers, alternate skins, and richer telegraphs.

5. There is no gameplay domain model yet.

There is no formal concept of:

- a run
- a room
- an encounter
- an ability
- a hit
- a modifier
- a reward
- a faction/enemy archetype

Without those concepts, content will be added as exceptions instead of systems.

## Design Principles

These rules should govern the rework.

### 1. Separate simulation from presentation

Gameplay truth should live in simulation components/resources.

Examples:

- position in gameplay space
- velocity
- health
- cooldowns
- hitboxes
- intentions
- buffs/debuffs

Rendering systems should only read simulation state and produce:

- transforms
- sprite selection
- tint/flash
- UI text
- VFX

### 2. Prefer domain modules over feature buckets

The code should not only be split into `player`, `combat`, `world`, `ui`.

It should be reorganized around game domains:

- app
- core
- run
- combat
- actors
- world
- presentation
- debug
- content

This avoids dumping all future gameplay into `combat.rs`.

### 3. Data defines content, systems define rules

Systems should not know that one enemy happens to have `78` speed or starts at `(12, 196)`.

Instead:

- data defines archetypes, room layouts, and encounter recipes
- systems interpret that data and run the rules consistently

### 4. Own transitions centrally

Screen transitions, room transitions, encounter transitions, and run transitions should not be scattered across unrelated systems.

There should be explicit orchestration for:

- app flow
- run flow
- encounter flow

### 5. Testability is a first-class requirement

Any architecture decision that makes deterministic testing harder is suspect.

The game should be able to:

- load a specific room
- spawn a specific encounter
- replay a scripted input sequence
- write telemetry
- capture a deterministic frame

## Target High-Level Structure

Recommended directory structure:

```text
src/
  app/
    mod.rs
    plugin.rs
    state.rs
    schedule.rs
  core/
    mod.rs
    math.rs
    time.rs
    tags.rs
    layers.rs
  content/
    mod.rs
    loader.rs
    archetypes.rs
    rooms.rs
    abilities.rs
    modifiers.rs
  run/
    mod.rs
    plugin.rs
    model.rs
    flow.rs
    room.rs
    rewards.rs
  actors/
    mod.rs
    plugin.rs
    components.rs
    player.rs
    enemies.rs
    spawn.rs
  combat/
    mod.rs
    plugin.rs
    abilities.rs
    hit_detection.rs
    damage.rs
    ai.rs
    status.rs
    telegraph.rs
  world/
    mod.rs
    plugin.rs
    map.rs
    collision.rs
    navigation.rs
    camera.rs
  presentation/
    mod.rs
    plugin.rs
    sprites.rs
    animation.rs
    vfx.rs
    ui.rs
    depth.rs
  debug/
    mod.rs
    plugin.rs
    telemetry.rs
    playtest.rs
    overlay.rs
```

This is the target. It does not need to appear in one commit.

## Target Runtime Layers

The runtime should be understood as five layers.

### App Layer

Responsibilities:

- boot
- screen state
- schedule configuration
- plugin composition
- top-level transitions

It should not own gameplay details.

### Run Layer

Responsibilities:

- run state
- room progression
- encounter progression
- reward timing
- death/victory routing

This is the layer that understands what a “run” is.

### Simulation Layer

Responsibilities:

- actors
- abilities
- AI
- movement
- collision
- hits
- damage
- status effects

This layer should be screen-agnostic.

### Content Layer

Responsibilities:

- enemy archetypes
- room definitions
- encounter recipes
- item/reward definitions
- ability definitions

This is where numbers and authored content live.

### Presentation Layer

Responsibilities:

- camera
- sprite selection
- render transforms
- UI
- effects
- sounds later

This layer should observe the simulation, not define it.

## Core Domain Model

These types should become explicit.

### App State

Use app state only for top-level screens:

- `Boot`
- `Title`
- `Run`
- `Intermission`

Avoid using app state for per-room flow.

### Run State

Create a `RunState` resource that owns:

- run id / seed
- current biome
- current room index
- room sequence
- current loadout
- collected modifiers
- run currency
- current encounter state
- end condition

This is the source of truth for a run.

### Room Definition

A room should be defined by data:

- biome tag
- room id
- bounds
- collision blockers
- decoration placement
- player spawn
- encounter recipe
- exits / follow-up routing

### Encounter Definition

An encounter should be a data object that answers:

- which enemies spawn
- where they spawn
- when waves trigger
- what completion condition applies

### Actor Model

Any player or enemy should be composed from common gameplay components:

- `Actor`
- `Faction`
- `LogicalPosition`
- `Velocity`
- `Collider`
- `Health`
- `Stats`
- `Facing`
- `Intent`
- `AbilityLoadout`
- `HitReceiver`

This matters because enemies and players will share more systems than the current prototype assumes.

### Ability Model

Every active action should be described as an ability, even if the implementation is simple.

Minimum model:

- `AbilityId`
- owner
- trigger type
- cooldown
- cast time / windup
- hit shape
- damage payload
- movement modifier
- presentation cue

Examples:

- base slash
- dash
- enemy strike
- beam
- burst
- boss phase attacks

This prevents special-case attack logic from being copied per actor type.

### Modifier Model

Run modifiers should not directly mutate arbitrary systems.

They should express targeted changes:

- stat modifier
- event hook
- ability replacement
- on-hit effect
- spawn table effect

That will keep build systems composable later.

## Schedules And System Sets

Right now systems are mostly attached directly to `Update`. That will not scale.

Use explicit ordered sets:

1. `InputSet`
2. `DecisionSet`
3. `MovementSet`
4. `AbilitySet`
5. `HitDetectionSet`
6. `DamageSet`
7. `RunResolutionSet`
8. `PresentationSyncSet`
9. `UiSet`
10. `TelemetrySet`

Benefits:

- deterministic ordering
- easier debugging
- safer future refactors
- better playtest reproducibility

## Recommended Plugin Boundaries

### `AppPlugin`

Owns:

- app state
- boot config
- plugin registration
- schedule setup

### `RunPlugin`

Owns:

- room lifecycle
- encounter lifecycle
- transitions between title/run/intermission
- restart/death/victory routing

### `ActorPlugin`

Owns:

- shared actor bundles
- player actor spawn
- enemy actor spawn
- common actor components

### `CombatPlugin`

Owns:

- ability execution
- AI decisions
- hit detection
- damage
- status effects

It should not decide when the run is won or which room to load next.

### `WorldPlugin`

Owns:

- camera
- map collision
- room layout spawn
- navigation helpers

It should not own enemy combat rules.

### `PresentationPlugin`

Owns:

- sprite sync
- camera polish
- hit flashes
- slash effects
- UI rendering

### `DebugPlugin`

Owns:

- debug commands
- overlay
- telemetry
- playtest integration

## Data Strategy

This project needs to move toward authored data, but not all at once.

### Phase 1

Keep content in Rust structs, but move it out of systems.

Examples:

- `EnemyArchetype`
- `RoomTemplate`
- `EncounterRecipe`
- `AbilityDef`

These can live in `content/*.rs`.

### Phase 2

Move stable content into external data files.

Suggested formats:

- `ron` for authored gameplay definitions
- image assets in `assets/`
- optional JSONL/CSV for telemetry only

### Rule

Systems should read content definitions through ids and registries, not hardcoded match statements spread across the codebase.

## Presentation Strategy

The current visual layer is good enough for prototyping but should be formalized.

### Camera

The game should commit to a stable world-space model:

- gameplay space is 2D logical space
- rendering projects it into isometric screen space
- camera framing is owned by one camera system

Do not let gameplay code decide camera transforms.

### Depth

Depth sorting should be unified. Do not recalculate depth differently in multiple modules.

There should be one place that turns logical position into:

- render translation
- z ordering
- optional shadow offset

### Visual Identity

Even placeholder visuals need rules:

- one palette family per biome
- one silhouette rule per faction
- one shadow treatment across actors and props
- one telegraph language for danger

This consistency matters more than fidelity.

## Combat Architecture

The combat system should evolve into four parts.

### 1. Input / Intent

Player input becomes intent:

- move vector
- attack trigger
- dash trigger
- selected ability

Enemy AI also becomes intent:

- chase
- strafe
- windup
- strike
- recover

This lets player and AI both feed into the same action pipeline.

### 2. Action Resolution

Actions should be created from intents if legal:

- cooldown ready
- resource available
- state allows cast

The result is a formal action or cast event.

### 3. Hit Resolution

Hits should be driven by hitboxes/hurtboxes or shape overlap events, not bespoke pairwise logic per ability forever.

Even a simple first version should separate:

- hit shape generation
- overlap detection
- damage application
- knockback/stagger

### 4. Outcome Resolution

Room clear, death, elite triggers, boss phase changes, reward unlocks all belong after combat resolution, not inside ability execution.

## World Architecture

The world should support authored rooms, not just one hardcoded arena.

### Room Runtime

Each loaded room should produce:

- terrain entities
- blocker entities
- decoration entities
- spawn anchors
- navigation or movement constraints

### Collision

Collision should become its own subsystem.

The current `ArenaObstacles` resource is a valid temporary bridge, but the target should be:

- collider components on entities
- room collision registry built from authored room data
- shared collision query helpers

### Navigation

Do not build a full navmesh system now.

For the first slice:

- simple steering
- blocker avoidance
- distance bands

is enough.

## UI Architecture

UI should split into:

- top-level screens
- in-run HUD
- debug overlay
- post-room reward screen

Avoid one module that mixes screen transitions with gameplay HUD logic.

Recommended split later:

- `presentation/ui/title.rs`
- `presentation/ui/hud.rs`
- `presentation/ui/reward.rs`
- `presentation/ui/debug.rs`

## Tooling And Test Architecture

The existing playtest harness is the correct direction. Extend it intentionally.

### Required capabilities

- deterministic room load
- scripted input sequences
- telemetry events
- frame capture
- scenario-based smoke tests

### Recommended scenarios

1. `spawn_smoke`
   Verifies the room loads, player exists, enemies exist, no immediate panic.

2. `combat_smoke`
   Move, attack, dash, capture frame, verify no panic.

3. `death_flow`
   Stand still until death, verify transition to hub.

4. `clear_flow`
   Script room completion, verify transition to post-room state.

5. `restart_flow`
   Restart repeatedly, verify no leaked entities.

### Telemetry contract

Emit stable events:

- `hub_entered`
- `run_started`
- `room_started`
- `room_cleared`
- `player_died`
- `run_finished`
- `ability_used`
- `damage_taken`

This will be more useful than ad hoc logging.

## Migration Plan

Do not do a giant rewrite with no playable checkpoint. Rework in vertical slices.

### Phase 0: Freeze Scope

Keep these constraints:

- one map
- one player kit
- one enemy archetype
- one room clear flow

No additional features until the architecture foundation exists.

### Phase 1: Introduce New Structure Without Changing Gameplay

Goal:

- create new module tree
- move current code into better boundaries
- keep behavior approximately the same

Deliverables:

- `app`, `run`, `actors`, `combat`, `world`, `presentation`, `debug`, `content`
- explicit system sets
- explicit `RunState`

### Phase 2: Extract Content Definitions

Goal:

- remove hardcoded room/enemy/ability data from systems

Deliverables:

- one enemy archetype definition
- one room definition
- one encounter recipe
- one player ability definition

### Phase 3: Introduce Formal Ability Pipeline

Goal:

- move from hand-coded attack logic to a reusable ability model

Deliverables:

- base slash as an ability
- dash as an ability
- enemy strike as an ability

### Phase 4: Introduce Formal Room Flow

Goal:

- room clear and restart stop being combat-owned logic

Deliverables:

- `RunState`
- `RoomState`
- encounter completion conditions
- death/clear transitions owned by run flow

### Phase 5: Presentation Cleanup

Goal:

- visuals are synced from state rather than embedded in simulation systems

Deliverables:

- sprite sync systems
- hit flash/VFX systems
- unified isometric projection helpers
- unified depth sorting

## Immediate Refactor Recommendation

If only one refactor slice is done next, do this:

### Slice: Build a real run/room model

Add:

- `RunState`
- `RoomDefinition`
- `EncounterDefinition`
- `RoomProgress`

Move responsibility:

- enemy spawn layout out of `combat.rs`
- room clear routing out of `combat.rs`
- restart/death transitions into `run` flow

Why this first:

- it prevents every future feature from coupling itself to `ScreenState`
- it creates the backbone required for rewards, elites, bosses, and multiple rooms
- it is the highest leverage architectural change

## Decision Rules

Use these rules during future implementation.

### Add a new system when

- multiple features depend on the same rule
- the rule has to be tested independently
- the code would otherwise be copied

### Add a new component when

- the state belongs to an entity
- multiple systems need to read or write it
- it has gameplay meaning outside presentation

### Add a new resource when

- the state is global to the run/session
- it does not belong to a specific entity
- deterministic test scenarios need direct control over it

### Add data definitions when

- designers will likely tune the values
- there will be more than one instance of the same pattern
- the system should stop knowing exact numbers

### Do not abstract yet when

- the game has only one instance and no second clear use case
- the abstraction would hide gameplay truth
- the code is still moving too quickly to settle

## Non-Negotiable Consistency Rules

These should be enforced going forward.

1. No system should both decide gameplay truth and render visuals for that truth.
2. No new content should be hardcoded in behavior systems once content structs exist.
3. No top-level screen state should be used as a substitute for run/room flow.
4. Every new combat action should become an ability or action definition, not a one-off branch.
5. Every new room should be data-defined.
6. Every major state transition should emit telemetry.
7. Every new system should belong to a named schedule set.

## Final Recommendation

Do not continue adding features onto the current structure.

The prototype has reached the exact point where more direct additions will create expensive cleanup later.

The next real milestone should be:

`Architecture Slice 1: Run/Room/Core Simulation Separation`

That is the foundation that will let the project support:

- more than one room
- more than one enemy archetype
- rewards and modifiers
- boss scripting
- deterministic tests
- consistent content authoring

Without that separation, every new feature will increase rewrite cost.
