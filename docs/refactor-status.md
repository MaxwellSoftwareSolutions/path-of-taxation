# Refactor Status

## Completed

### Slice 1: Run And Content Separation

Implemented:

- `content.rs` for room and enemy definitions
- `run.rs` for `RunState`, room progress, and room outcome ownership
- enemy spawn ownership moved out of `combat.rs`
- player spawn now reads the active room definition
- telemetry now includes `room_id`

Impact:

- room identity is no longer implicit
- content starts existing outside behavior systems
- room clear and death routing are no longer combat-owned

### Slice 2: Ordered System Sets

Implemented:

- `core.rs` with shared `GameSet` schedule sets
- central update ordering configured in one place
- movement, combat, resolution, presentation, input, and UI systems assigned to sets

Impact:

- deterministic execution order
- clearer system responsibilities
- safer future refactors

## Current State

The game remains playable and the scripted playtest still works after both slices.

Verified with:

- `cargo check`
- `./scripts/playtest.sh smoke`

## Recommended Next Slices

### Slice 3: Actor Model Extraction

Goals:

- introduce shared actor bundles/components
- stop treating player and enemies as unrelated special cases
- formalize collider, health, stats, and faction concepts

### Slice 4: Ability Pipeline

Goals:

- make base slash, dash, and enemy strike formal abilities
- separate input/intent from action execution
- prepare for additional skills without copy-paste combat branches

### Slice 5: Presentation Split

Goals:

- move sprite sync, hit flash, slash VFX, and depth sync into a clearer presentation layer
- reduce rendering logic inside gameplay modules

### Slice 6: External Content Definitions

Goals:

- move stable room and enemy data out of Rust code into authored files
- keep systems reading ids and definitions instead of hardcoded values
