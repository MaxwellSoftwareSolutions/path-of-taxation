# Path of Taxation

`Path of Taxation` is a satirical single-player action roguelite inspired by the culture of overbuilt ARPG systems, obsessive buildcraft, loot inflation, and patch-note trauma.

The current scope is intentionally narrow:

- One playable character
- One biome plus boss arena
- One strong real-time combat loop
- One turn-based satire minigame
- Short repeatable runs

## First Playable Goal

Ship a vertical slice where the player can:

1. Enter a short run.
2. Fight through a handful of rooms with readable action combat.
3. Pick strange but meaningful upgrades.
4. Defeat a satirical boss.
5. Return to the hub and play one turn-based minigame that changes the next run.
6. Unlock a new strategic option and immediately want another attempt.

## Stack

- Rust
- Bevy 0.18
- Single-player only for v1

`SpaceTimeDB` is intentionally out of scope for the first playable. If the game later adds seasonal ladders, ghosts, or cloud saves, it can be introduced after the core loop is proven.

## Status

This repository is currently scaffolded but not yet compiled in this environment because the Rust toolchain is not installed locally.

## Next Milestones

- Build the room-to-room combat prototype
- Implement one complete enemy set
- Implement one boss encounter
- Build the first turn-based minigame
- Connect the run loop to hub progression

See [docs/game-plan.md](/home/hex/path-of-taxation/docs/game-plan.md) and [docs/vertical-slice.md](/home/hex/path-of-taxation/docs/vertical-slice.md).

