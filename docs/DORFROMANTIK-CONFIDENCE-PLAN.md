# Dorfromantik Restart -- Confidence-Adjusted Plan

**Purpose:** raise execution confidence by reducing early scope and separating rendering proof from asset integration and gameplay.

## Why The Previous Plan Was Too Risky

The earlier restart plan was directionally right, but still bundled too many unknowns into the first pass:

- new art direction
- new rendering model
- new asset pipeline
- new gameplay loop
- new content model

That makes it too easy to fail ambiguously.

## New Rule

Every phase must answer exactly one question.

If a phase does not answer a single clear question, it is too broad.

## Phase A -- Can We Make A Good-Looking Diorama Sandbox?

Question:

- can the repo launch into a clean 3D hex world that already looks promising without imported assets?

Scope:

- no combat
- no queue system
- no scoring
- no external asset packs required
- procedural tiles and primitive props only

Success:

- the client launches reliably
- the camera is pleasant
- the board reads as a miniature world

This is the first implementation phase and should be completed before anything else.

## Phase B -- Can We Integrate One Asset Family Cleanly?

Question:

- can one external art family be imported into the sandbox without style drift or pipeline chaos?

Scope:

- one chosen pack family only
- likely KayKit
- `.glb` only
- replace only a few procedural props at first

Success:

- imported assets match the board scale
- materials and lighting look coherent
- no runtime asset-pipeline confusion

## Phase C -- Can We Place Tiles Cleanly?

Question:

- can the player hover, rotate, preview, and place a tile with confidence?

Scope:

- hover highlight
- preview ghost tile
- rotate input
- place / reject placement

Success:

- placement feels legible with no debug explanation

## Phase D -- Can The Sandbox Become A Game?

Question:

- does placement quality matter enough to create a loop?

Scope:

- adjacency rules
- simple score feedback
- next tile queue

Success:

- there is a reason to think before placing

## What This Changes Technically

The first implementation slice should use the minimum dependable stack:

- Bevy 0.18
- `hexx`
- `bevy_panorbit_camera`

Delayed until Phase B or C:

- imported assets
- picking stack
- inspector
- asset loader
- content manifests

Those are still useful, but not necessary to prove the visual foundation.

## Confidence Estimate

With this narrower order:

- `9/10` confidence on Phase A
- `8.5/10` confidence on Phase B
- `8.5/10` confidence on Phase C
- lower confidence only returns when art taste and design iteration become the bottleneck

That is the correct way to earn a great game here: prove the scene, then prove the art pipeline, then prove interaction, then prove game loop.
