# Path of Taxation -- AAA-Quality Vertical Slice Roadmap

**Date:** 2026-04-01
**Scope:** "AA scope with AAA polish" -- one complete outdoor zone + boss + hub loop at final quality
**Prerequisite:** GAME-DEV-PLAN.md (source of truth for design)

---

## Current State Audit

### What Works (Verified in Code)

| System | Status | Key Files |
|---|---|---|
| Bevy 0.18 app shell | Working | `client/src/main.rs` |
| State machine (Boot/Menu/Hub/Run) | Working, auto-skips to Run | `client/src/app_state.rs` |
| Sub-state machine (CombatPhase) | Scaffolded, auto-advances RoomSelect->Combat | `client/src/app_state.rs` |
| WASD movement | Working, world-space with iso projection | `client/src/plugins/player.rs` |
| Dodge roll (Shift) | Working, 6 active + 3 recovery frames, cancel at 7 | `client/src/plugins/player.rs` |
| Facing system | Working, 8-dir from movement or mouse | `client/src/plugins/player.rs` |
| Ability pipeline | Working, Anticipation->Active->Recovery->Idle | `client/src/plugins/combat.rs` |
| Input buffering | Working, 4-frame ring buffer | `client/src/plugins/input.rs` |
| Cancel windows | Working, recovery cancelable into dodge/ability | `client/src/components/combat.rs` |
| Hit detection | Working, circle-circle collision | `client/src/plugins/combat.rs` |
| Damage application | Working, with knockback insert | `client/src/plugins/combat.rs` |
| Knockback | Working, ease-out curve (1 - t^2) | `client/src/components/combat.rs` |
| Projectile system | Scaffolded, ticks lifetime + moves | `client/src/plugins/combat.rs` |
| AoE system | Scaffolded, ticks lifetime | `client/src/plugins/combat.rs` |
| Enemy AI state machine | Working, Idle->Chase->Windup->Attack->Recover | `client/src/plugins/enemies.rs` |
| Crowd management | Working, max 8 active attackers, others orbit | `client/src/plugins/enemies.rs` |
| Attack telegraphs | Working, red ground circles during Windup | `client/src/plugins/enemies.rs` |
| Stagger system | Working, 3-frame hitstun on enemies | `client/src/plugins/enemies.rs` |
| Enemy death + dissolution | Working, fade-out alpha over 20 frames | `client/src/plugins/enemies.rs` |
| FLARE spritesheet animation | Working, 8x8 atlas, directional facing | `client/src/rendering/sprites.rs` |
| Weapon swing arc | Working, spawns on attack anticipation | `client/src/plugins/player.rs` |
| Hitstop | Working, freeze-frame on hit (3f normal, 5f crit) | `client/src/plugins/vfx.rs` |
| Hit flash | Working, white flash for 2 frames | `client/src/plugins/vfx.rs` |
| Damage numbers | Working, float upward + fade | `client/src/plugins/vfx.rs` |
| Particle bursts | Working, directional with gravity + fade | `client/src/plugins/vfx.rs` |
| Screen shake | Working, directional, exponential decay, 12px cap | `client/src/plugins/camera.rs` |
| Camera smooth follow | Working, lerp toward player | `client/src/plugins/camera.rs` |
| Camera zoom pulse | Working, temporary zoom effect | `client/src/plugins/camera.rs` |
| Kill slow-mo | Working, time scale on last enemy death | `client/src/plugins/vfx.rs` |
| HUD (health/mana/cooldowns) | Working, UI nodes with percentage fill | `client/src/plugins/ui.rs` |
| Enemy health bars | Working, world-space sprites above enemies | `client/src/plugins/ui.rs` |
| Death screen | Working, "YOUR TAX RETURN HAS BEEN REJECTED" | `client/src/plugins/ui.rs` |
| Room title display | Working, updates per room number | `client/src/plugins/ui.rs` |
| Minimap placeholder | Exists as colored rectangle | `client/src/plugins/ui.rs` |
| Arena terrain | Working, layered tiles + jitter + walls + props | `client/src/plugins/run.rs` |
| Fog drift | Working, sinusoidal animated fog sprites | `client/src/plugins/run.rs` |
| Vignette | Working, 4 dark border rectangles | `client/src/plugins/run.rs` |
| Hub scene | Working, minimal: title + floor tiles + pillars | `client/src/plugins/hub.rs` |
| Room clear detection | Working, advances CombatPhase on zero enemies | `client/src/plugins/run.rs` |
| Room transitions | Working, Enter/Space to proceed | `client/src/plugins/run.rs` |
| Win/lose detection | Working, death or all rooms cleared | `client/src/plugins/run.rs` |
| Debug hitbox visualization | Working, F1 toggle | `client/src/plugins/combat.rs` |
| RON content files | 14 files across abilities/enemies/feel/items/rooms/debate | `content/` |
| Isometric projection | Working, 2:1 dimetric, depth sorting | `client/src/rendering/isometric.rs` |
| SpacetimeDB server module | Scaffolded, 7 table modules + 4 reducer modules | `server/src/` |

### What Is Missing or Placeholder

| Gap | Current State | Impact |
|---|---|---|
| **RON loading at runtime** | Ability frame data is hardcoded in `ability_input_system` | All combat tuning requires recompile |
| **Ability differentiation** | All abilities spawn the same melee hitbox | No projectiles, AoE, shield, beam, or teleport actually work |
| **Mana consumption** | Mana component exists but is never deducted | Abilities are free, no resource management |
| **Enemy variety** | Only 1 AI behavior (Chase), 2 sprite alternates | All enemies act identically |
| **Boss fights** | No boss system at all | Cannot complete a run |
| **Room selection** | Auto-advances, no door UI | No player agency in pathing |
| **Loot/drops** | No drop system, no items, no inventory | No build progression |
| **Passive tree** | Empty directory `content/passive_tree/` | No character customization |
| **Hub NPCs** | No NPC entities, no dialogue | Hub is a static scene |
| **SpacetimeDB connection** | Server is scaffolded, client has no network code | No persistence, no saves |
| **Gamepad input** | Only keyboard + mouse | No controller support |
| **Audio** | No audio plugin, no SFX, no music | Silent game |
| **Post-processing** | No bloom, no color grading, vignette is rectangles | Missing atmosphere |
| **Pause menu** | Escape key detected but unused | Cannot pause |
| **Settings** | None | No volume, no keybinding, no options |
| **Tilemap from Tiled** | Arena is procedurally placed sprites | No repeatable room design workflow |

---

## Phase 0: Data-Driven Foundation

**Goal:** Stop hardcoding, start loading. Every tuning value comes from RON files at runtime.
**Duration:** 3-5 days
**Testable outcome:** Change a value in a RON file, restart, see the change.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 0.1 | **Create RON loader system** -- use `bevy_asset_loader` or manual `ron::de` to load `content/feel/combat_feel.ron` into a `CombatFeelConfig` resource at startup | M | New: `client/src/content/mod.rs`, `client/src/content/combat_feel.rs`. Edit: `client/src/main.rs` | -- |
| 0.2 | **Create AbilityDefs resource** -- load `content/abilities/refund_witch.ron` into a `Vec<AbilityDef>` resource. Wire `ability_input_system` to read frame data from this resource instead of the hardcoded match statement | M | Edit: `client/src/plugins/combat.rs` (lines 93-101). New: `client/src/content/abilities.rs` | 0.1 |
| 0.3 | **Create EnemyDefs resource** -- load `content/enemies/act1_basic.ron` into a resource. Wire `spawn_room_enemies` and `enemy_spawn_system` to read HP/damage/speed/aggro/attack range from defs | M | Edit: `client/src/plugins/enemies.rs`, `client/src/plugins/run.rs` (lines 458-486). New: `client/src/content/enemies.rs` | 0.1 |
| 0.4 | **Wire combat feel params** -- replace hardcoded hitstop frames (3/5), particle counts (8/15), shake intensities (2.0/4.0) in `damage_application_system` with values from `CombatFeelConfig` | S | Edit: `client/src/plugins/combat.rs` (lines 279-306) | 0.1 |
| 0.5 | **Add Bevy asset watcher** -- enable `AssetPlugin` file watcher for hot-reload during development | S | Edit: `client/src/main.rs` (DefaultPlugins config) | -- |

**Parallelizable:** 0.1 must go first. Then 0.2, 0.3, 0.4 can run in parallel. 0.5 is independent.

---

## Phase 1: Complete the Combat Kit

**Goal:** All 6 Refund Witch abilities work distinctly. Mana is consumed. Combat has real decision-making.
**Duration:** 1-2 weeks
**Testable outcome:** Player can use Tax Bolt (projectile), Audit Storm (AoE field), Refund Shield (absorb), Depreciation Beam (channel), Form 1040 Barrage (multi-projectile), Capital Loss Teleport (blink + field). Mana constrains usage.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 1.1 | **Implement projectile spawning from ability data** -- when `ability_pipeline_system` enters Active phase and the ability type is `Projectile`, spawn a `Projectile` entity with hitbox, velocity from `AbilityDef.projectile_speed`, direction from `Facing`. Tax Bolt fires 1 bolt; Form 1040 Barrage fires 5 in a spread arc | L | Edit: `client/src/plugins/combat.rs` (`ability_pipeline_system`). Edit: `client/src/components/combat.rs` (ensure `Projectile` has `Hitbox` + `Damage`) | 0.2 |
| 1.2 | **Implement AoE field spawning** -- when ability type is `AoE`, spawn an `AoeZone` entity at the target position (mouse world pos). Audit Storm: damage ticks every 500ms for 4s. Capital Loss Teleport: spawn field at origin position | M | Edit: `client/src/plugins/combat.rs`. New component: `AoeVisual` for rendering the zone circle | 0.2 |
| 1.3 | **Implement teleport ability** -- Capital Loss Teleport: move player WorldPosition to mouse world pos (clamped to max range 256), spawn AoE at origin. Needs `GameInput.mouse_world_pos` converted to world space via `screen_to_world` | M | Edit: `client/src/plugins/combat.rs`, `client/src/plugins/player.rs` | 0.2 |
| 1.4 | **Implement shield ability** -- Refund Shield: add `ShieldState` component to player. Shield absorbs damage (intercept in `damage_application_system` before HP reduction). When shield breaks or expires, heal 30% of absorbed | M | New: `client/src/components/combat.rs` (`ShieldState`). Edit: `client/src/plugins/combat.rs` (`damage_application_system`) | 0.2 |
| 1.5 | **Implement channel ability** -- Depreciation Beam: while attack button held and ability is Active phase, keep phase alive (don't advance to Recovery). Drain mana per tick. Spawn a beam visual (line from player to target direction). Apply damage per tick interval | L | Edit: `client/src/plugins/combat.rs` (`ability_pipeline_system`, `ability_input_system`). New: beam rendering system | 0.2 |
| 1.6 | **Wire mana consumption** -- deduct `AbilityDef.mana_cost` from `Mana.current` when ability starts. Prevent ability start if insufficient mana. Add mana regeneration system (5/sec from RON base_stats) | S | Edit: `client/src/plugins/combat.rs` (`ability_input_system`). New: `mana_regen_system` in `client/src/plugins/player.rs` | 0.2 |
| 1.7 | **Per-ability cooldown values** -- replace uniform 60-frame cooldowns with per-slot values from `AbilityDef.cooldown_ms` converted to frames | S | Edit: `client/src/components/combat.rs` (`Cooldowns::default`), `client/src/plugins/combat.rs` | 0.2 |
| 1.8 | **Ability VFX differentiation** -- each ability gets distinct particle colors based on damage type. Use `damage_type_particles` from `combat_feel.ron`. Tax Bolt = orange sparks, Audit Storm = purple ink, etc. | M | Edit: `client/src/plugins/combat.rs` (particle color in `damage_application_system`), `client/src/plugins/vfx.rs` | 0.4 |
| 1.9 | **AoE zone rendering** -- render AoE zones as pulsing translucent circles on the ground layer. Audit Storm = dark purple cloud. Capital Loss field = bureaucratic red. Pulse alpha on tick intervals | M | New: `client/src/plugins/vfx.rs` (add `aoe_visual_system`) | 1.2 |

**Parallelizable:** 1.1, 1.2, 1.3, 1.4, 1.5 can be developed in parallel after 0.2. 1.6 and 1.7 are small and independent. 1.8 depends on 0.4. 1.9 depends on 1.2.

---

## Phase 2: Enemy Variety and AI Behaviors

**Goal:** All 7 enemy archetypes from `act1_basic.ron` have distinct behaviors. Elite prefix system works.
**Duration:** 1-2 weeks
**Testable outcome:** Fight rooms with mixed enemy compositions where each type demands a different response.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 2.1 | **Refactor AI to behavior-dispatch** -- replace single `enemy_ai_system` with behavior-specific logic based on `EnemyBehavior` enum from the enemy def. Current code only implements `Chase`. Need: `Shamble` (slower chase, wider swings), `Swarm` (group cohesion, faster, flanking), `Ranged` (fire projectile, stay at range), `Stationary` (passive aura damage), `Kiter` (attack then reposition away), `Debuffer` (cast slow/silence from range) | XL | Edit: `client/src/plugins/enemies.rs` (`enemy_ai_system`). Edit: `client/src/components/enemy.rs` (add `EnemyBehavior` component) | 0.3 |
| 2.2 | **Enemy projectile system** -- Ink Crawler and Enforcement Agent fire projectiles at the player. Reuse `Projectile` component. Enemy projectiles have `Faction::Enemy` hitboxes. Ink Crawler projectiles leave DoT ground pools | L | Edit: `client/src/plugins/enemies.rs` (`enemy_attack_hitbox_system`). Reuse: `client/src/plugins/combat.rs` (`projectile_system`) | 2.1 |
| 2.3 | **Stationary aura damage** -- Bureaucratic Bramble deals damage in a radius every tick without moving. No windup needed. Render thorny aura circle | M | Edit: `client/src/plugins/enemies.rs` (new system: `aura_damage_system`) | 2.1 |
| 2.4 | **Debuff system** -- Red Tape Weaver applies `SlowDebuff` and `SilenceDebuff` components to the player. Slow reduces `MovementSpeed` by 40% for 3s. Silence prevents ability activation for 2s. Visual: player sprite tinted with debuff color | L | New: `client/src/components/combat.rs` (`SlowDebuff`, `SilenceDebuff`). Edit: `client/src/plugins/player.rs` (movement reads slow), `client/src/plugins/combat.rs` (ability_input checks silence) | 2.1 |
| 2.5 | **Kiter repositioning** -- Enforcement Agent: after attacking, strafe perpendicular to player direction to maintain range. If player gets within 60% of attack range, retreat | M | Edit: `client/src/plugins/enemies.rs` (`enemy_ai_system`, Kiter branch) | 2.1 |
| 2.6 | **Swarm cohesion** -- Paper Shredders: group behavior. Spawn in packs of 3-5. Share a loose formation. Flanking: try to approach from different angles rather than all stacking on the same point | M | Edit: `client/src/plugins/enemies.rs` (Swarm branch). New component: `SwarmGroup(u32)` in `client/src/components/enemy.rs` | 2.1 |
| 2.7 | **Elite prefix system** -- load `content/enemies/act1_elites.ron`. When spawning an enemy, 15% chance to roll an elite prefix. Apply HP/damage multipliers. Add glowing outline sprite effect + name plate. Prefixes: "Auditing" (DoT aura), "Taxing" (damage boost), "Regulatory" (debuff on hit), "Penalizing" (damage reflect) | L | New: `client/src/content/elites.rs`. Edit: `client/src/plugins/enemies.rs` (`enemy_spawn_system`). Edit: `client/src/components/enemy.rs` | 0.3 |
| 2.8 | **Enemy composition per zone** -- define room encounter compositions in `content/rooms/templates.ron`. Zone 1 (Clearfile): Undead Accountants + Paper Shredders. Zone 2 (Mud Bureau): Tax Collectors + Ink Crawlers. Zone 3 (Red Ink Vale): all types with Brambles + Weavers + Agents. Load composition at room spawn time | M | Edit: `client/src/plugins/run.rs` (`spawn_room_enemies`). New: `client/src/content/rooms.rs` | 0.3, 2.1 |

**Parallelizable:** 2.2, 2.3, 2.4, 2.5, 2.6 are all independent branches of 2.1. 2.7 is independent after 0.3. 2.8 depends on 2.1 + 0.3.

---

## Phase 3: Run Structure and Room Selection

**Goal:** A complete run loop: choose rooms via doors, fight through varied encounters, reach boss.
**Duration:** 1-2 weeks
**Testable outcome:** Start a run, see 3 doors with reward previews, choose rooms, clear 5-7 rooms, reach boss room.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 3.1 | **Room selection UI** -- replace `auto_advance_to_combat` with a door selection screen. Spawn 2-3 door UI elements showing: room type icon, reward type text, threat level. Player clicks a door or presses 1/2/3 to choose. Transition to Combat on selection | L | Edit: `client/src/plugins/run.rs` (replace `auto_advance_to_combat`). New: `client/src/plugins/run.rs` (`room_select_ui_system`, `room_select_input_system`). Edit: `client/src/app_state.rs` (CombatPhase::RoomSelect now has real UI) | -- |
| 3.2 | **Room type implementation** -- implement all 8 room types from the design doc. Combat and EliteCombat spawn enemies. Treasure spawns a chest entity. Shop spawns vendor UI. Event shows narrative choice. Challenge adds timer. Rest heals 30%. IRS Audit is hard combat with premium rewards | XL | Edit: `client/src/plugins/run.rs`. New: `client/src/plugins/run.rs` (separate spawn functions per room type). Content: `content/rooms/templates.ron` | 3.1 |
| 3.3 | **Run state tracking** -- track Deductions (in-run currency) in `RunStateRes`. Display Deductions earned per room. Show run stats: rooms cleared, enemies killed, damage dealt | S | Edit: `client/src/plugins/run.rs` (`RunStateRes`), `client/src/plugins/ui.rs` | -- |
| 3.4 | **Room transition scene** -- when entering RoomClear, show a brief transition: "Room Cleared" text, loot summary, satirical loading tip from `content/feel/loading_tips.ron`. 2-second minimum display before doors appear | M | Edit: `client/src/plugins/run.rs`. New: loading tip renderer | 0.1, 3.1 |
| 3.5 | **Event room system** -- load `content/rooms/events.ron`. Display narrative text with 2-3 choices. Each choice has stat effects (heal, damage, currency, apply buff/debuff for rest of run). Satirical writing per GAME-DEV-PLAN.md section 9.4 | L | New: `client/src/plugins/events.rs`. Content: `content/rooms/events.ron` (already exists, wire it) | 3.2 |
| 3.6 | **Shop room UI** -- mid-run vendor. Display 3-4 items for sale, priced in Deductions. Items: healing potion (30% HP), random Tax Form (ability upgrade), random stat boost. Click to buy, deduct currency | L | New: `client/src/plugins/shop.rs`. Edit: `client/src/plugins/run.rs` | 3.2, 3.3 |
| 3.7 | **Arena generation from room templates** -- replace the single hardcoded arena in `setup_run` with per-room terrain generation. Each room type and zone has a terrain template: tile set, prop placement, size, wall configuration. Load from RON | L | Edit: `client/src/plugins/run.rs` (`setup_run` becomes `generate_room_terrain`). Content: expand `content/rooms/templates.ron` | 0.3 |

**Parallelizable:** 3.1 is the critical path. 3.3 is independent. 3.4, 3.5, 3.6 depend on 3.1/3.2. 3.7 is independent after 0.3.

---

## Phase 4: Boss Fight System

**Goal:** One fully playable multi-phase boss: The Bloated Filer (tutorial boss).
**Duration:** 1-2 weeks
**Testable outcome:** Reach the boss room, fight a multi-phase boss with distinct attack patterns, telegraphs, phase transitions with screen flash and voice barks.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 4.1 | **Boss framework** -- create `BossState` component with phase tracking, HP thresholds, attack pattern sequencer. Boss reads phase data from `BossDef` loaded from RON. Phase transitions trigger: screen flash, hitstop, bark text, potential arena change | XL | New: `client/src/plugins/boss.rs`. New: `client/src/components/boss.rs`. Content: `content/enemies/bosses/bloated_filer.ron` (already exists) | 0.3 |
| 4.2 | **Boss intro cutscene** -- when entering `CombatPhase::BossIntro`: camera zooms to boss position, boss name + title appears ("THE BLOATED FILER / He stamps, therefore you aren't"), hold 2s, zoom back, transition to BossFight | M | Edit: `client/src/plugins/run.rs` or new `client/src/plugins/boss.rs`. Uses `CameraZoomPulse` | 4.1 |
| 4.3 | **Boss attack patterns** -- implement attack pattern system. Boss cycles through attacks defined in `BossPhase.attacks`. Each attack: telegraph (ground indicator + boss animation), execute (spawn hitbox/projectile/AoE), recover. For Bloated Filer: "rubber_stamp_slam" (big circle AoE), "paperwork_barrage" (projectile spread), "audit_charge" (dash toward player) | XL | Edit: `client/src/plugins/boss.rs`. Reuse hitbox/projectile/AoE spawning from combat plugin | 4.1, Phase 1 |
| 4.4 | **Phase transition logic** -- when boss HP crosses threshold, trigger: 12-frame hitstop, screen flash (white overlay 4 frames), time dilation (0.2 speed for 500ms), bark text display, arena hazard spawn/change. Bloated Filer at 50%: "YOUR FILING IS INCOMPLETE" + adds ink pools to arena floor | L | Edit: `client/src/plugins/boss.rs`. Uses `HitstopMsg`, `KillSlowMoMsg` | 4.1 |
| 4.5 | **Boss health bar** -- large health bar at top of screen (not world-space). Shows boss name + phase indicator. Smooth damage animation (fast bar follows HP, slow red "damage taken" bar catches up) | M | Edit: `client/src/plugins/ui.rs`. New UI component: `BossHealthBar` | 4.1 |
| 4.6 | **Boss bark text system** -- display boss voice lines as large centered text that fades after 3s. Triggered by phase transitions and specific attacks. "You are audited, stupid citizen!" on first phase transition | S | New: bark rendering in `client/src/plugins/ui.rs` or `client/src/plugins/boss.rs` | 4.1 |
| 4.7 | **Boss death and run completion** -- when boss HP reaches 0: extended kill cam (zoom in 8%, 500ms slow-mo at 0.2 speed), boss dissolution (larger particle burst, 40+ particles), victory text, transition to Results state | M | Edit: `client/src/plugins/boss.rs`. Edit: `client/src/plugins/run.rs` (`win_lose_detection_system`) | 4.1 |

**Parallelizable:** 4.1 is the critical path. 4.2, 4.5, 4.6 can develop in parallel after 4.1. 4.3 needs Phase 1 complete. 4.4 needs 4.1. 4.7 needs 4.1.

---

## Phase 5: Loot, Items, and Build Identity

**Goal:** Items drop from enemies. Player can equip gear. Builds feel different. Legislative Amendments change abilities mid-run.
**Duration:** 2 weeks
**Testable outcome:** Kill enemies, see loot drop, pick up items, open inventory, equip weapon, see stats change. Find a Legislative Amendment that changes how Tax Bolt works.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 5.1 | **Loot drop system** -- on enemy death, roll drop chance from `EnemyDef.item_drop_chance`. Spawn a loot entity at death position with a ground sprite and pickup radius. Drop Deductions (always) + chance for item | L | New: `client/src/plugins/loot.rs`. New: `client/src/components/items.rs`. Edit: `client/src/plugins/enemies.rs` (`enemy_death_system`) | 0.3 |
| 5.2 | **Item generation** -- generate items with random rarity (Normal/Magic/Rare). Roll affixes from `content/items/currency.ron` and a new `content/items/affixes.ron`. Magic = 1 affix, Rare = 3-4 affixes. Affix values roll between min/max from `AffixDef` | L | New: `client/src/content/items.rs`. Content: new `content/items/affixes.ron` | 0.1 |
| 5.3 | **Pickup system** -- walk over loot entity to pick up. Add to player inventory resource. Play pickup sound (placeholder). Currency auto-collects. Items go to inventory | M | Edit: `client/src/plugins/loot.rs`. New: `client/src/components/items.rs` (`Inventory` resource) | 5.1 |
| 5.4 | **Inventory UI** -- press Tab to open inventory overlay. Show equipment slots (10 slots per design doc). Show backpack grid of picked-up items. Click item to see tooltip. Click equipped slot to equip. Satirical tooltips with legalese from `AffixDef.legalese` | XL | New: `client/src/plugins/inventory_ui.rs`. Edit: `client/src/plugins/ui.rs` | 5.2, 5.3 |
| 5.5 | **Stat calculation from equipment** -- equipped items modify player stats. Sum all affix values. Apply to damage, HP, mana, speed, etc. Recalculate on equip/unequip | M | New: `client/src/systems/stats.rs`. Edit: `client/src/components/player.rs` (add computed stats) | 5.4 |
| 5.6 | **Legislative Amendments** -- load `content/abilities/legislative_amendments.ron`. Implement 3 amendments for the vertical slice: "Retroactive Refund" (Tax Bolt returns on hit), "Full Refund" (shield explodes on break), "Compound Interest" (stacking damage per bolt on same target). Drop as rare reward from elite kills or room completion | XL | New: `client/src/systems/amendments.rs`. Edit: `client/src/plugins/combat.rs` (ability behavior hooks). Content: already exists in `content/abilities/legislative_amendments.ron` | Phase 1, 5.1 |
| 5.7 | **Currency drops and display** -- Deductions drop from all enemies (always). Display Deductions counter in HUD. Other currencies (Audit Notices, Premium Filing Fees) drop rarely. Stack in inventory | M | Edit: `client/src/plugins/loot.rs`, `client/src/plugins/ui.rs` | 5.1, 5.3 |

**Parallelizable:** 5.1 and 5.2 can develop in parallel. 5.3 needs 5.1. 5.4 needs 5.2 + 5.3. 5.5 needs 5.4. 5.6 is largely independent (needs Phase 1 + 5.1). 5.7 is independent after 5.1.

---

## Phase 6: Hub, NPCs, and Meta-Progression

**Goal:** Hub is a real place with NPCs, vendors, and permanent upgrades. Post-run summary hooks the next run.
**Duration:** 2 weeks
**Testable outcome:** Return from a run to a populated hub. Talk to Renly, buy a weapon upgrade. Spend Compliance Credits at the Filing Cabinet. See relationship progress. Start next run with permanent bonuses.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 6.1 | **Hub NPC entities** -- spawn 4 NPC entities (Renly, Una, Finn, Hooded Advisor) at fixed positions in the hub. FLARE spritesheets with idle animation. Interaction radius: walk near + press E to talk | L | Edit: `client/src/plugins/hub.rs`. New: `client/src/components/npc.rs` | -- |
| 6.2 | **Dialogue system** -- display dialogue in a bottom-of-screen text box. NPC portrait + name. Text advances on click/Enter. Branch based on relationship rank and run history. Load dialogue from RON | L | New: `client/src/plugins/dialogue.rs`. Content: new `content/dialogue/` directory with per-NPC RON files | 6.1 |
| 6.3 | **Post-run summary screen** -- on entering `AppState::Results`: show rooms cleared, enemies defeated, damage dealt, Deductions earned, Compliance Credits earned, unlock progress bars, NPC relationship bars. "FILE AGAIN" and "RETURN TO OFFICE" buttons | L | Edit: `client/src/plugins/ui.rs` or new `client/src/plugins/results_ui.rs`. Edit: `client/src/app_state.rs` | Phase 3 |
| 6.4 | **Meta-currency: Compliance Credits** -- earn Credits based on rooms cleared and boss kills (persists between runs). Store in a `MetaProgression` resource. Display in hub | M | New: `client/src/resources/meta.rs`. Edit: `client/src/plugins/run.rs` | 6.3 |
| 6.5 | **Filing Cabinet (permanent upgrades)** -- hub object. Interact to open upgrade UI. Show 5-6 upgrades with rank costs (per GAME-DEV-PLAN.md section 11.1). "Extended Filing Period" (+5% HP/rank), "Speed Filing" (+5% speed/rank), etc. Spend Credits to unlock ranks | L | New: `client/src/plugins/filing_cabinet.rs`. Edit: `client/src/plugins/hub.rs` | 6.4 |
| 6.6 | **Apply meta-upgrades to runs** -- when starting a run, apply all purchased Filing Cabinet upgrades to player base stats. Store unlocked ranks in `MetaProgression` | M | Edit: `client/src/plugins/player.rs` (`spawn_player`), `client/src/resources/meta.rs` | 6.5 |
| 6.7 | **NPC relationship tracking** -- each NPC has a relationship rank (0-5). Advances by talking after runs and gifting items. Store in `MetaProgression`. Each rank unlocks a gameplay benefit per GAME-DEV-PLAN.md section 11.2 | M | Edit: `client/src/components/npc.rs`, `client/src/resources/meta.rs` | 6.1, 6.2 |
| 6.8 | **Hub vendor (Renly)** -- interact with Renly to open shop UI. Sells weapons and armor for Deductions/Credits. Inventory refreshes per run. Prices affected by relationship rank | M | Edit: `client/src/plugins/hub.rs`. Reuse shop UI from Phase 3 | 6.1, 5.4 |

**Parallelizable:** 6.1 and 6.3 can develop in parallel. 6.2 needs 6.1. 6.4 needs 6.3. 6.5 needs 6.4. 6.6 needs 6.5. 6.7 needs 6.1 + 6.2. 6.8 needs 6.1 + Phase 5.

---

## Phase 7: Audio Foundation

**Goal:** The game has sound. Hits are punchy. Music sets the mood.
**Duration:** 1 week
**Testable outcome:** Every hit plays an impact sound. Background music plays. UI clicks have feedback. Boss has a unique track.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 7.1 | **Audio plugin scaffold** -- create `AudioPlugin`. Load SFX and music assets. Use Bevy's built-in `AudioPlayer` / `AudioSink`. Create `SfxEvent` message for triggering sounds from any system | M | New: `client/src/plugins/audio.rs`. Edit: `client/src/main.rs` | -- |
| 7.2 | **Impact SFX** -- play layered hit sounds on every `HitMsg`. 3-4 variants per hit type, random selection, +/-5% pitch variation. Impact + reaction layers per `combat_feel.ron` sound spec | M | Edit: `client/src/plugins/audio.rs`. Assets: `assets/audio/sfx/hit_*.ogg` (need to source/generate) | 7.1 |
| 7.3 | **Ability cast SFX** -- each ability plays a cast sound on Anticipation frame 0. Tax Bolt = arcane whoosh, Audit Storm = paper rustle + thunder, Refund Shield = glass shimmer, etc. | M | Edit: `client/src/plugins/audio.rs`, `client/src/plugins/combat.rs` | 7.1 |
| 7.4 | **Ambient music** -- adaptive music system with 2 layers: ambient (hub/exploration) and combat (during fights). Crossfade between layers based on `AppState` and enemy proximity. 1 hub track, 1 combat track for the vertical slice | L | Edit: `client/src/plugins/audio.rs`. Assets: `assets/audio/music/` | 7.1 |
| 7.5 | **UI SFX** -- menu clicks, item pickup (cash register), currency pickup, door selection, room clear fanfare, death screen stamp | S | Edit: `client/src/plugins/audio.rs` | 7.1 |
| 7.6 | **Silence-before-impact** -- for boss attacks: dip music + ambient volume to 40% for 250ms before the hit lands, per `combat_feel.ron` sound spec | M | Edit: `client/src/plugins/audio.rs`, `client/src/plugins/boss.rs` | 7.1, Phase 4 |
| 7.7 | **Boss music** -- unique track for the Bloated Filer fight. Triggers on `CombatPhase::BossFight` enter. Transitions from combat music with a 1-beat crossfade | M | Edit: `client/src/plugins/audio.rs` | 7.4, Phase 4 |

**Parallelizable:** 7.1 first. Then 7.2, 7.3, 7.4, 7.5 all in parallel. 7.6 and 7.7 need Phase 4.

---

## Phase 8: Rendering Polish and Atmosphere

**Goal:** The game looks atmospheric. Post-processing sells the mood. Terrain feels alive.
**Duration:** 1-2 weeks
**Testable outcome:** Vignette is smooth radial gradient. Color grading gives a dark fantasy tone. Bloom on magical effects. Terrain has parallax vegetation.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 8.1 | **Replace rectangle vignette with shader** -- current vignette is 4 opaque rectangles. Replace with a proper radial vignette using a full-screen quad with a fragment shader or a pre-baked vignette texture. Smooth falloff from clear center to dark edges | M | Edit: `client/src/plugins/run.rs` (remove rectangle vignette). New: vignette shader or sprite | -- |
| 8.2 | **Color grading** -- add a LUT-based color grading pass or apply a tint to the camera. Target: desaturated dark fantasy with warm highlights (similar to Children of Morta). Different LUT per zone (cold blue for Clearfile, warm brown for Mud Bureau, blood red for Red Ink Vale) | L | New: `client/src/rendering/post_process.rs`. Bevy 0.18 post-processing pipeline | -- |
| 8.3 | **Bloom on magic effects** -- add bloom/glow to ability VFX, elite enemy outlines, boss attacks. Use Bevy's built-in bloom plugin (`bevy::core_pipeline::bloom::Bloom`) or HDR emissive sprites | M | Edit: `client/src/main.rs` (add Bloom plugin). Edit: VFX sprite colors to use HDR values > 1.0 | -- |
| 8.4 | **Vegetation and atmospheric props** -- add foreground grass tufts, background trees, drifting leaves. Z-sorted between terrain and fog layers. Gentle sway animation (sinusoidal offset). 3-4 vegetation sprite types from existing downloaded assets | L | Edit: `client/src/plugins/run.rs` (add vegetation layer in `setup_run`). New vegetation sprites | -- |
| 8.5 | **Dynamic lighting overlay** -- Children of Morta technique: render a light map at lower resolution, multiply over the scene. Player emits a warm radial light. Abilities emit colored lights on cast (Tax Bolt = orange flash, Audit Storm = purple glow). Torches in arena emit steady warm light | XL | New: `client/src/rendering/lighting.rs`. This is a significant rendering feature | -- |
| 8.6 | **Screen flash effects** -- white flash (2-4 frames) on boss phase transition, room clear, crit kills. Red flash on player damage. Implement as a full-screen overlay sprite with rapid alpha fade | S | New: system in `client/src/plugins/vfx.rs` | -- |
| 8.7 | **Improved death dissolution** -- replace alpha fade with particle dissolution. Enemy sprite breaks into 10-15 pixel clusters that scatter outward + gravity + fade. Per-damage-type themed: paper scraps for accountants, ink splatter for crawlers | L | Edit: `client/src/plugins/enemies.rs` (`dying_dissolution_system`). Edit: `client/src/plugins/vfx.rs` | -- |

**Parallelizable:** All tasks in this phase are independent of each other. 8.5 is the riskiest and largest. 8.1, 8.3, 8.6 are quick wins. Prioritize 8.1 + 8.3 + 8.6 first, then 8.4 + 8.7, then 8.2 + 8.5.

---

## Phase 9: Satire Systems

**Goal:** The game is funny through mechanics, not just text. Patch Notes, Compliance Meter, and crafting bench work.
**Duration:** 1-2 weeks
**Testable outcome:** Mid-run "Patch Notes" event actually modifies player stats. Compliance Meter affects gameplay. Crafting bench has bureaucratic form-filling UI.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 9.1 | **Mid-run Patch Notes** -- 10% chance on room transition: display fake patch notes UI (per GAME-DEV-PLAN.md section 10.2). The stat changes are REAL. Parse modifications from a RON file. "Tax Bolt damage reduced by 15%. This is a buff." Player must click "ACCEPT AND CONTINUE" | L | New: `client/src/plugins/patch_notes.rs`. Content: new `content/satire/patch_notes.ron` | Phase 3 |
| 9.2 | **Compliance Meter** -- persistent HUD element. Starts at 50%. Using "legitimate" abilities: +compliance. Exploiting loopholes (using Amendments, gambling at Finn's): -compliance. High compliance = enemies stronger, shops cheaper. Low compliance = better drops, IRS Agent random encounters | L | New: `client/src/components/compliance.rs`. Edit: `client/src/plugins/ui.rs` (HUD display). Edit: `client/src/plugins/run.rs` (apply effects) | Phase 3, Phase 5 |
| 9.3 | **IRS Agent mini-boss** -- when compliance drops below 20%, 30% chance per room: an IRS Agent spawns mid-combat. Uses Enforcement Agent base AI but stronger (2x HP, 1.5x damage). Defeating it drops premium loot + "Plea Bargain" item. Losing to it = instant run end | M | Edit: `client/src/plugins/enemies.rs`. New enemy def in `content/enemies/act1_elites.ron` | 9.2, 2.5 |
| 9.4 | **Crafting bench** -- hub object. Select item + currency. Fill out a satirical form (radio buttons with meaningless options like "Reason for modification: A) Required by law, B) Clerical error, C) Other"). 2-second "processing" animation with progress bar. Random result displayed as official notice. "DENIED" outcome possible (lose currency, item unchanged) | XL | New: `client/src/plugins/crafting.rs`. Content: new `content/items/crafting.ron` | Phase 5, Phase 6 |
| 9.5 | **Loading screen tips** -- load `content/feel/loading_tips.ron` (already exists). Display a random tip during room transitions and loading. Render as centered italic text over dark background | S | Edit: `client/src/plugins/run.rs` (room transition). Content: already exists | 3.4 |
| 9.6 | **Death screen expansion** -- expand death screen to show: cause of death (last enemy/ability that hit), time filed, rooms completed, deductions claimed. "Still solvent, Taxpayer?" quote. "FILE AGAIN" restarts run, "RETURN TO OFFICE" goes to hub | M | Edit: `client/src/plugins/ui.rs` (`death_screen_system`) | Phase 3 |

**Parallelizable:** 9.1, 9.2, 9.4, 9.5, 9.6 can all develop in parallel. 9.3 depends on 9.2.

---

## Phase 10: SpacetimeDB Integration (Persistence)

**Goal:** Runs are saved. Inventory persists. Meta-progression survives restart.
**Duration:** 1-2 weeks
**Testable outcome:** Close the game, reopen it, see your hub upgrades and inventory intact.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 10.1 | **SpacetimeDB client connection** -- connect to local SpacetimeDB instance on startup. Create thin bridge layer: `NetworkBridge` resource that holds the SpacetimeDB connection handle. Subscribe to relevant tables | L | New: `client/src/plugins/network.rs`. Edit: `client/src/main.rs`. Edit: `server/src/reducers/lifecycle.rs` | -- |
| 10.2 | **Player identity and auth** -- on first launch, create a player record via `init_player` reducer. On subsequent launches, reconnect with stored identity. Store identity token in local file (`~/.path-of-taxation/identity`) | M | Edit: `server/src/reducers/lifecycle.rs`. Edit: `client/src/plugins/network.rs` | 10.1 |
| 10.3 | **Persist meta-progression** -- sync `MetaProgression` resource to SpacetimeDB `Player` table. Filing Cabinet purchases, NPC relationships, unlocked content. Write on change, read on connect | M | Edit: `server/src/tables/player.rs`. Edit: `client/src/plugins/network.rs`. Edit: `client/src/resources/meta.rs` | 10.1, Phase 6 |
| 10.4 | **Persist inventory** -- sync player inventory to SpacetimeDB `Item` + `CurrencyStack` tables. Write on pickup/equip/sell. Read on connect | M | Edit: `server/src/tables/items.rs`. Edit: `client/src/plugins/network.rs`. Edit: `client/src/components/items.rs` | 10.1, Phase 5 |
| 10.5 | **Run history** -- when a run ends, write a `RunHistory` record to SpacetimeDB: rooms cleared, enemies killed, boss defeated, time, cause of failure. Display in hub as "Tax Filing History" | M | Edit: `server/src/tables/run.rs`, `server/src/reducers/run_flow.rs`. Edit: `client/src/plugins/network.rs` | 10.1, Phase 3 |
| 10.6 | **Crash recovery** -- if the client disconnects mid-run, the SpacetimeDB `Run` table retains state. On reconnect, detect active run and offer to resume or abandon | L | Edit: `server/src/reducers/run_flow.rs`. Edit: `client/src/plugins/network.rs` | 10.1, 10.5 |

**Parallelizable:** 10.1 and 10.2 first. Then 10.3, 10.4, 10.5 in parallel. 10.6 last.

---

## Phase 11: Production UI and Input

**Goal:** Gamepad works. Settings menu exists. Controls are rebindable. UI is readable.
**Duration:** 1 week
**Testable outcome:** Plug in a controller, navigate menus, play the game entirely with gamepad. Open settings, change volume, rebind keys.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 11.1 | **Gamepad input** -- read gamepad axes for movement (left stick) and aiming (right stick). Map buttons: A=dodge, X=attack, bumpers=cycle ability, triggers=ability cast. Use Bevy's `Gamepad` resource | L | Edit: `client/src/plugins/input.rs` (`gather_input_system`). Add gamepad branch alongside keyboard | -- |
| 11.2 | **Pause menu** -- press Escape (or Start on gamepad) to pause. Freeze all Update systems (use Bevy's `Time<Virtual>::pause()`). Show overlay: Resume, Settings, Quit to Hub, Quit to Desktop. Run stats visible | M | New: `client/src/plugins/pause.rs`. Edit: `client/src/main.rs` | -- |
| 11.3 | **Settings menu** -- accessible from pause menu and main menu. Categories: Video (resolution, fullscreen, vsync, pixel scaling), Audio (master/music/SFX sliders), Controls (key rebinding display), Gameplay (screen shake intensity 0-100%, loot filter toggle) | L | New: `client/src/plugins/settings.rs`. New: `client/src/resources/settings.rs` (persisted to local file) | 11.2 |
| 11.4 | **Key rebinding** -- store input mappings in a `KeyBindings` resource loaded from `~/.path-of-taxation/keybindings.ron`. Settings UI shows current bindings. Click a binding + press new key to rebind. Write changes back to file | L | Edit: `client/src/plugins/input.rs`. New: `client/src/resources/keybindings.rs` | 11.3 |
| 11.5 | **UI readability pass** -- ensure all UI elements have sufficient contrast. Health bar has a dark border. Damage numbers have a 1px dark outline (text shadow). Cooldown slots show ability icons (placeholder colored squares until art). Enemy health bars scale with camera zoom | M | Edit: `client/src/plugins/ui.rs` (all UI systems) | -- |
| 11.6 | **Screen shake intensity setting** -- multiply all shake intensities by user setting (0.0 to 1.0). Apply in `camera_shake_system`. Accessibility feature for motion-sensitive players | S | Edit: `client/src/plugins/camera.rs` (`camera_shake_system`). Read from settings resource | 11.3 |

**Parallelizable:** 11.1 is independent. 11.2 first for settings chain (11.3 -> 11.4). 11.5 and 11.6 independent.

---

## Phase 12: Performance and Stability

**Goal:** Locked 60fps. No hitches during combat. Sub-50ms input-to-visual. No crashes.
**Duration:** 1 week
**Testable outcome:** Run stress test: 30 enemies + 4 AoE zones + 50 particles + boss. Frame time stays under 16.67ms.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 12.1 | **Entity culling** -- do not render entities outside the camera viewport. Skip `depth_sort_system` transform updates for off-screen entities. Use Bevy's `VisibilityPlugin` frustum culling or manual AABB check | M | Edit: `client/src/rendering/isometric.rs` (`depth_sort_system`) | -- |
| 12.2 | **Particle pooling** -- current particle system spawns + despawns entities every frame. Replace with an entity pool: pre-spawn N particle entities, reuse by toggling visibility. Target: zero allocations during combat | L | Edit: `client/src/plugins/vfx.rs` (`particle_burst_spawn_system`, `particle_system`) | -- |
| 12.3 | **Sprite batching audit** -- verify that sprites using the same texture atlas are batched by Bevy's renderer. Group enemies by sprite type. Ensure all floor tiles share atlases. Target: under 100 draw calls per frame | M | Audit: `client/src/rendering/sprites.rs`, `client/src/plugins/run.rs` | -- |
| 12.4 | **Async room generation** -- generate next room's terrain data on a background thread during current room's combat. Use Bevy's `AsyncComputeTaskPool`. Room transition instantaneous from player's perspective | L | Edit: `client/src/plugins/run.rs` | Phase 3 |
| 12.5 | **Frame time budget monitoring** -- add a debug overlay (F2) showing: frame time, entity count, particle count, draw calls. Use Bevy's `DiagnosticsPlugin` + custom overlay | S | Edit: `client/src/plugins/input.rs` (F2 toggle). New: debug overlay system | -- |
| 12.6 | **Stress test scene** -- add a debug command (F3) to spawn 30 enemies simultaneously for performance testing. Verify frame time stays under 16.67ms with full VFX (hitstop + particles + shake + damage numbers) | S | New: stress test system in debug module | Phases 1-8 |

**Parallelizable:** All tasks are independent.

---

## Phase 13: Polish and Playtesting

**Goal:** Everything works together. The run loop is fun. "One more run" pull exists.
**Duration:** 2 weeks
**Testable outcome:** External playtester can complete a full loop (hub -> 5 rooms -> boss -> results -> hub -> next run) and immediately want to play again.

### Tasks

| # | Task | Size | Files | Depends On |
|---|---|---|---|---|
| 13.1 | **Full loop integration test** -- play through the complete loop 10+ times. Document every hitch, crash, softlock, unclear UI moment, unfun encounter. Create issue list | L | All files | All phases |
| 13.2 | **Combat feel tuning pass** -- adjust every value in `combat_feel.ron` based on playtesting. Does hitstop feel right? Is screen shake too much? Are particles too sparse? Iterate with hot-reload | M | Edit: `content/feel/combat_feel.ron` | 0.1 |
| 13.3 | **Balance pass** -- adjust all enemy HP/damage in `content/enemies/act1_basic.ron`. Adjust ability damage/cooldowns in `content/abilities/refund_witch.ron`. Target: rooms take 30-60 seconds, boss takes 2-3 minutes, full run takes 15-20 minutes | M | Edit: RON files in `content/` | All phases |
| 13.4 | **Satirical writing pass** -- review every text string in the game. NPC dialogue, item tooltips, loading tips, death screen, patch notes, boss barks. Ensure PoE-specific references land. Ensure non-PoE players still find it funny | L | Edit: all RON content files, dialogue files | Phase 9 |
| 13.5 | **Visual consistency audit** -- check all sprites against the style bible. Same palette? Same outline weight? Same proportions? At 1080p, can you identify every enemy by silhouette alone at 50% zoom? | M | Review: all sprite assets in `assets/sprites/` | -- |
| 13.6 | **Edge case hardening** -- handle: player dies during boss phase transition, room clear with projectiles still in flight, ability cast while debuffed, equipment change during combat, disconnect during run | L | Edit: multiple systems across all plugins | All phases |
| 13.7 | **Accessibility review** -- verify screen shake disable works, verify colorblind palette mode (if implemented), verify damage numbers are readable, verify boss telegraphs are clear at 400ms+ minimum | M | Review + edit: multiple UI and VFX systems | Phase 11 |

---

## Dependency Graph (Critical Path)

```
Phase 0 (Data-Driven)
  |
  +---> Phase 1 (Combat Kit) ---> Phase 4 (Boss) ---> Phase 7 (Audio)
  |        |
  |        +---> Phase 5 (Loot/Items) ---> Phase 6 (Hub/NPCs) ---> Phase 9 (Satire)
  |
  +---> Phase 2 (Enemy AI) ---> Phase 3 (Run Structure)
  |
  +---> Phase 8 (Rendering) [independent, can parallelize with anything]
  |
  +---> Phase 10 (Persistence) [can start after Phase 5+6]
  |
  +---> Phase 11 (Production UI) [can start after Phase 3]
  |
  +---> Phase 12 (Performance) [can start after Phase 8]
  |
  +---> Phase 13 (Polish) [after everything else]
```

**Critical path:** 0 -> 1 -> 4 -> 7 -> 13 (combat must feel good before anything else)
**Secondary path:** 0 -> 2 -> 3 -> 9 (run structure must work for content to matter)
**Parallel track A:** Phase 8 (rendering) can develop alongside any phase
**Parallel track B:** Phase 10 (persistence) can develop alongside Phases 7-9
**Parallel track C:** Phase 11 (input/UI) can develop alongside Phases 5-9

---

## Milestone Checkpoints

### Milestone 1: "Does Hitting Feel Good?" (After Phase 1)

Play the game. Use all 6 abilities. If hitting an enemy with Tax Bolt does not produce a visceral "thwack" sensation through hitstop + screen shake + particles + sound, STOP and iterate on combat feel before proceeding. Nothing else matters if combat is flat.

**Gate criteria:**
- Every hit produces 3+ simultaneous feedback channels
- Tax Bolt projectile visually distinct from melee swing
- Audit Storm AoE field visible and readable
- Mana constrains ability spam
- Dodge cancel out of recovery feels responsive

### Milestone 2: "Is the Run Fun?" (After Phase 3 + 4)

Play a complete run: hub -> door selection -> 5 rooms -> boss. If the run feels samey or the boss is unfair, iterate on room composition and boss patterns.

**Gate criteria:**
- Room selection creates meaningful choices (not obvious best pick)
- Mixed enemy compositions create varied tactical puzzles
- Boss fight has at least 2 distinct phases with readable telegraphs
- Run takes 15-20 minutes
- Player dies to avoidable mistakes, not unclear mechanics

### Milestone 3: "One More Run" (After Phase 6 + 9)

Complete 3 runs back-to-back. If there is no pull to start a 4th run, the meta-progression is broken.

**Gate criteria:**
- Post-run summary shows tangible progress toward a specific unlock
- NPC dialogue references what happened in the run
- Filing Cabinet upgrade makes next run noticeably different
- Legislative Amendment made at least one run feel mechanically distinct
- At least 2 satirical moments made you laugh (Patch Notes, Compliance Meter, crafting bench)

### Milestone 4: "Ship It" (After Phase 13)

External playtester (5-10 people who have never seen the game) plays the full demo. Track: completion rate, average run count, "would you play more?" rating, moments they laughed.

**Gate criteria:**
- 80%+ complete at least one full run
- Average 3+ runs per session
- 60%+ say they would play more
- Locked 60fps with no frame drops during combat
- Zero crashes across all test sessions

---

## Estimated Total Timeline

| Phase | Duration | Can Overlap With |
|---|---|---|
| Phase 0: Data-Driven | 3-5 days | -- |
| Phase 1: Combat Kit | 1-2 weeks | -- |
| Phase 2: Enemy AI | 1-2 weeks | Phase 1 (late) |
| Phase 3: Run Structure | 1-2 weeks | Phase 2 (late) |
| Phase 4: Boss Fight | 1-2 weeks | Phase 3 |
| Phase 5: Loot/Items | 2 weeks | Phase 4 |
| Phase 6: Hub/NPCs | 2 weeks | Phase 5 (late) |
| Phase 7: Audio | 1 week | Phase 4+ |
| Phase 8: Rendering | 1-2 weeks | Any phase |
| Phase 9: Satire | 1-2 weeks | Phase 5+ |
| Phase 10: Persistence | 1-2 weeks | Phase 6+ |
| Phase 11: Production UI | 1 week | Phase 3+ |
| Phase 12: Performance | 1 week | Phase 8+ |
| Phase 13: Polish | 2 weeks | After all |

**Sequential (worst case):** ~20 weeks
**With parallelization (realistic):** ~12-14 weeks
**With aggressive overlap and focused scope:** ~10-12 weeks

---

## Files Index (Quick Reference)

All paths relative to `/home/hex/path-of-taxation/`.

**New files to create:**
- `client/src/content/mod.rs` -- content loading module
- `client/src/content/combat_feel.rs` -- CombatFeelConfig resource
- `client/src/content/abilities.rs` -- AbilityDefs resource
- `client/src/content/enemies.rs` -- EnemyDefs resource
- `client/src/content/elites.rs` -- ElitePrefixDefs resource
- `client/src/content/items.rs` -- item generation from defs
- `client/src/content/rooms.rs` -- room template loading
- `client/src/plugins/audio.rs` -- audio plugin
- `client/src/plugins/boss.rs` -- boss fight system
- `client/src/plugins/crafting.rs` -- crafting bench
- `client/src/plugins/dialogue.rs` -- NPC dialogue system
- `client/src/plugins/events.rs` -- event room system
- `client/src/plugins/inventory_ui.rs` -- inventory overlay
- `client/src/plugins/loot.rs` -- loot drops and pickup
- `client/src/plugins/network.rs` -- SpacetimeDB bridge
- `client/src/plugins/patch_notes.rs` -- mid-run patch notes
- `client/src/plugins/pause.rs` -- pause menu
- `client/src/plugins/results_ui.rs` -- post-run summary
- `client/src/plugins/settings.rs` -- settings menu
- `client/src/plugins/shop.rs` -- mid-run and hub shop
- `client/src/plugins/filing_cabinet.rs` -- meta-progression upgrades
- `client/src/components/boss.rs` -- boss components
- `client/src/components/compliance.rs` -- compliance meter
- `client/src/components/items.rs` -- item/inventory components
- `client/src/components/npc.rs` -- NPC components
- `client/src/resources/meta.rs` -- meta-progression resource
- `client/src/resources/keybindings.rs` -- rebindable input
- `client/src/resources/settings.rs` -- persistent settings
- `client/src/systems/amendments.rs` -- legislative amendment effects
- `client/src/systems/stats.rs` -- stat calculation from gear
- `client/src/rendering/post_process.rs` -- post-processing pipeline
- `client/src/rendering/lighting.rs` -- dynamic lighting
- `content/satire/patch_notes.ron` -- mid-run patch note definitions
- `content/items/affixes.ron` -- item affix pool
- `content/items/crafting.ron` -- crafting outcome tables
- `content/dialogue/` -- per-NPC dialogue RON files

**Existing files with major edits:**
- `client/src/main.rs` -- add new plugins, asset watcher, bloom
- `client/src/plugins/combat.rs` -- ability differentiation, mana, shield, beam, RON loading
- `client/src/plugins/enemies.rs` -- behavior dispatch, projectiles, auras, death drops
- `client/src/plugins/player.rs` -- mana regen, debuff effects, stat application
- `client/src/plugins/run.rs` -- room selection UI, room types, terrain generation, transitions
- `client/src/plugins/ui.rs` -- boss HP bar, compliance meter, expanded death screen, readability
- `client/src/plugins/vfx.rs` -- screen flash, improved dissolution, AoE visuals, particle pooling
- `client/src/plugins/hub.rs` -- NPC entities, interaction, vendor
- `client/src/plugins/input.rs` -- gamepad support, rebinding
- `client/src/plugins/camera.rs` -- shake intensity setting
- `client/src/components/combat.rs` -- shield state, debuffs, computed stats
- `client/src/components/enemy.rs` -- swarm group, enemy behavior component
- `client/src/rendering/isometric.rs` -- frustum culling
- `client/src/rendering/sprites.rs` -- additional sprite assets
- `server/src/reducers/lifecycle.rs` -- player creation and auth
- `server/src/reducers/run_flow.rs` -- run history, crash recovery
- `server/src/tables/player.rs` -- meta-progression fields
- `server/src/tables/items.rs` -- persistent inventory
- `server/src/tables/run.rs` -- run history records
