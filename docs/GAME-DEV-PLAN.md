# Path of Taxation 2 -- Game Development Plan

**Date:** 2026-03-31 (revised 2026-04-01)
**Status:** Fresh start -- keeping ideas only, rebuilding from scratch
**Target:** AAA-quality pixelated satirical action roguelite parodying Path of Exile 1 & 2
**Revision:** v2 -- added combat feel spec, run structure, systemic satire, meta-progression, missing systems, architecture clarification

---

## 1. GAME IDENTITY

### Concept
Path of Taxation is a **satirical isometric action roguelite** that parodies Path of Exile 1 & 2's obsessive complexity, loot inflation, patch note culture, and community drama. The player is a "Taxpayer" -- an exile punished for questioning whether any of the systems make sense.

### Tone
- **Satirical but playable.** The comedy enhances gameplay, never undermines it.
- **PoE-specific, not generic ARPG.** Every joke targets PoE mechanics, GGG design philosophy, community culture, and Chris Wilson quotes.
- **Dark humor meets bureaucratic absurdity.** Tax law as eldritch horror.

### Core Loop
1. **Hub** (Clearfile Tax Office) -- manage loadout, NPC vendors, passive tree, stash, relationship dialogues
2. **Enter a Run** -- choose starting modifier from Debate Club rewards
3. **Room Selection** -- see 2-3 doors showing reward type BEFORE entering (combat/treasure/shop/event/challenge)
4. **Clear rooms** -- fight enemies, pick up loot/currency, choose build-defining upgrades
5. **Between rooms** -- choose next room, spend currency at mid-run shops, encounter narrative events
6. **Boss encounter** -- multi-phase satirical boss fight with arena mechanics
7. **Run Summary** -- see stats, progress toward unlocks, teaser for next story beat
8. **Return to Hub** -- spend meta-currency, upgrade gear, talk to NPCs (relationships progress per run)
9. **Debate Club** -- turn-based card minigame for next-run modifiers
10. **Meta-progression** -- unlock new characters, hub buildings, cosmetics, Tax Form pool expansion

### World: Taxeclast
The cursed land of Taxeclast, where the Seed of Regulation was released, spreading bureaucracy across everything. The ancient Tax Code (The Beast) stirs beneath.

---

## 2. TECH STACK

| Component | Technology | Purpose |
|---|---|---|
| Game Engine | **Rust + Bevy 0.18** | Client-side rendering, ECS, input, audio |
| Backend/DB | **SpacetimeDB 2** | Server-authoritative game logic, persistence, multiplayer-ready |
| Art Pipeline | **PixelLab + Retro Diffusion + Aseprite** | AI-generated pixel art with manual refinement |
| Audio | **Suno/Udio** (music) + **ElevenLabs/Bark** (SFX) | AI-generated soundtrack and sound effects |
| Level Editor | **Tiled** (via bevy_ecs_tiled) | Isometric room layout design |
| Content Format | **RON** files | Data-driven enemies, abilities, items, rooms |
| Version Control | **Git + Git LFS** | Code + binary assets |
| Build | **Cargo workspace** | Monorepo with client, server, shared, tools crates |

### Why NOT VoxAi
VoxAi generates 3D voxel models (Minecraft-style), not 2D pixel art. For AAA pixelated 2D game art, the correct tools are **PixelLab** (sprite sheets, animations, directional rotation) and **Retro Diffusion** (palette-locked pixel art, Aseprite integration).

---

## 3. ARCHITECTURE OVERVIEW

### Monorepo Structure
```
path-of-taxation/
|-- Cargo.toml                    # Workspace root
|-- client/                       # Bevy game client (binary)
|   +-- src/
|       |-- main.rs
|       |-- app_state.rs          # Boot > Menu > Hub > Run > Debate > Results
|       |-- plugins/              # camera, input, player, combat, enemies, vfx, ui, audio, network, debate, hub, run
|       |-- components/           # ECS components: player, combat, enemy, items, skills, world
|       |-- systems/              # movement, ability_pipeline, damage, pickup, spawning, interpolation, prediction, animation
|       +-- rendering/            # isometric, tilemap, sprites, lighting
|
|-- server/                       # SpacetimeDB module (wasm library)
|   +-- src/
|       |-- lib.rs
|       |-- tables/               # 17 tables: player, run, combat, items, skills, debate, events
|       |-- reducers/             # lifecycle, run_flow, combat, items, skills, debate, scheduled
|       +-- logic/                # damage_calc, enemy_ai, room_gen, loot, passive_tree, debate_engine
|
|-- shared/                       # Types shared between client + server
|   +-- src/                      # types, ids, constants, ability_defs, enemy_defs, item_defs
|
|-- content/                      # RON data files
|   |-- abilities/                # Per-character ability sets
|   |-- enemies/                  # Enemy archetypes + bosses
|   |-- items/                    # Currency, equipment, uniques
|   |-- rooms/                    # Room templates + encounter configs
|   |-- passive_tree/             # Full passive tree definition
|   +-- debate/                   # Cards + opponent configs
|
|-- assets/                       # Runtime assets
|   |-- sprites/                  # characters/, enemies/, vfx/, items/, tiles/
|   |-- audio/                    # sfx/, music/
|   |-- fonts/
|   +-- ui/                       # hud/, menus/, passive_tree/
|
|-- raw/                          # Source art (NOT shipped)
|   |-- aseprite/                 # .aseprite working files
|   |-- ai_generated/             # Raw AI outputs before cleanup
|   +-- references/               # Style bible, palettes, mood boards
|
|-- tools/                        # Dev tools
|   |-- content-validator/        # RON schema validation
|   |-- asset-pipeline/           # AI art integration, atlas packing
|   +-- balance-sim/              # Headless damage/loot simulation
|
+-- docs/                         # Documentation
```

### SpacetimeDB Design
- **17 tables** across 7 modules: Player/Character/MetaUnlock, Run/Room/RunHistory, ActiveEnemy/PlayerCombatState/Projectile/AreaEffect, Item/CurrencyStack/Equipment, SkillLoadout/PassiveAllocation/CooldownState, DebateSession/DebateReward, DamageEvent/VfxEvent/ChatMessage/ServerTick
- **Server tick** at 20Hz (50ms) via scheduled reducer -- processes enemy AI, projectiles, area effects, buff/debuff ticking
- **Subscriptions** scoped: global state always subscribed, combat entities only during runs, debate state only during debates
- **Event tables** for transient data (damage numbers, VFX triggers) -- pruned each tick

### Client Architecture
- **Server-authoritative with client prediction** for movement and cast animations
- **Network bridge**: `ServerStateCache` resource syncs SpacetimeDB subscriptions to Bevy ECS entities
- **State machine**: Boot > Menu > Hub > Run (with CombatPhase sub-state) > Debate > Results
- **11 plugins**: camera, input, player, combat, enemies, VFX, UI, audio, network, hub, debate

### Key Design Decisions
| Decision | Choice | Rationale |
|---|---|---|
| Single vs multiplayer | Single-player first, multiplayer-ready | SpacetimeDB runs locally for single-player; config change for multiplayer |
| Save system | SpacetimeDB commit log IS the save | No save files, no save corruption, no format versioning |
| Content format | RON files (dev) synced to SpacetimeDB tables (runtime) | RON for fast iteration, SpacetimeDB for authoritative runtime |
| Combat authority | Server-authoritative, client predicts movement + animations | Prevents cheating, enables future multiplayer |
| Debate Club | Pure state machine in SpacetimeDB, no scheduled reducers | Turn-based doesn't need tick processing |

---

## 4. DEMO: ACT 1 -- "THE AUDIT BEGINS" (Satirical PoE2 Act 1)

### Story Overview
The Taxpayer is sentenced to death by Commissioner Geonor (the Iron Auditor) for questioning the Tax Code. You wash up on a shore covered in overdue tax notices. Traverse the bureaucratic wilderness of Taxeclast, freeing the Hooded Advisor (God of Loopholes), and ultimately confront Commissioner Geonor in the Revenue Manor where he transforms into a Loan Shark.

### Demo Scope: 3 Zones + 1 Boss

For the demo, we implement a focused vertical slice of Act 1:

#### Zone 1: The Clearfile (Tutorial + First Combat)
- **Visual theme:** Cold, sparse forest of frozen filing cabinets and scattered paperwork
- **Quest: "Mercy for the Filer"** -- kill The Bloated Filer (tutorial boss, slow telegraphed attacks with oversized rubber stamp)
- **Introduces:** Movement (WASD), dodge roll (Tax Evasion Roll), basic attack, first Tax Form Gem
- **Enemies:** Undead Accountants (shambling, melee), Paper Shredders (small, fast)
- **NPC:** Renly the Pencil-Pusher at the Clearfile Tax Office (hub town)

#### Zone 2: The Mud Bureau (First Dungeon)
- **Visual theme:** Underground IRS tunnels, narrow winding corridors, dripping ink
- **Quest: "Treacherous Audit"** -- navigate tunnels, defeat The Auditor (boss)
- **Introduces:** Ability system (equip 2nd skill), item drops, currency (Audit Notices)
- **Enemies:** Tax Collectors (chase AI, windup strike), Ink Crawlers (poison/DoT)
- **Boss: The Auditor** -- burrows through your records, phases: paperwork barrage > desk slam > audit frenzy

#### Zone 3: The Red Ink Vale (Mid-Act Climax)
- **Visual theme:** Reddish blighted landscape, rivers of red ink, Pillars of Red Tape
- **Quest: "Secrets in the Fine Print"** -- find 3 Pillars of Red Tape, defeat the Regulation King
- **Introduces:** Elite enemies, loot rarity system, passive tree (first 20 nodes)
- **Enemies:** Bureaucratic Brambles (area denial), Enforcement Agents (ranged), Red Tape Weavers (debuffs)
- **Boss: The Regulation King** -- conjures penalty notices that fly at you, mandatory forms radiate outward, spins regulations around himself

#### Final Boss: Commissioner Geonor, the Iron Auditor (Revenue Manor)
- **Phase 1 (Human):** Frozen Asset walls + Penalty Slam, Asset Freeze cage, Debt Balloons
- **Intermission (33% HP):** Heals to full, transforms into a Loan Shark
- **Fog Phase:** Bureaucratic Obfuscation fills arena, Geonor lunges from the fog
- **Phase 2 (Loan Shark):** Chill of Insolvency breath, Interest Rate Spike slam, The Final Audit beam
- **Story beat:** Oriana (CFO, Chief Financial Offender) betrays him and escapes with the pension fund

### NPCs in Demo
| NPC | Role | Satire Source |
|---|---|---|
| Renly the Pencil-Pusher | Vendor, quest giver, sells stationery weapons | Renly the Blacksmith |
| Una the Tax Consultant | Skill vendor, senses regulatory corruption | Una the Caster Vendor |
| Finn the Offshore Accountant | Gambles with your returns, sells "legitimate" deductions | Finn the Gambler |
| The Hooded Advisor | Identifies deductions, respec passive points, secretly God of Loopholes | The Hooded One / Sin |

### Iconic Lines for Demo
| Original PoE | Path of Taxation |
|---|---|
| "Still sane, Exile?" | "Still solvent, Taxpayer?" |
| "You are captured, stupid beast!" | "You are audited, stupid citizen!" |
| "This world is an illusion, exile" | "This refund is an illusion, taxpayer" |
| "THE TOUCH OF GOD!" | "THE TOUCH OF THE IRS!" |
| "WHAT IN DAMNATION HAVE YOU DONE" | "WHAT IN TAXATION HAVE YOU FILED" |
| "Close your eyes and slam" | "Close your eyes and file" |
| "Krangled" (ruined) | "Audited" (ruined) |

---

## 5. PLAYER CHARACTER: THE REFUND WITCH

### Identity
Mid-range spellcaster rewarded for repositioning. Abilities themed around refunds, tax policy, and financial magic.

### Ability Kit (6 Abilities)
| Slot | Ability | Type | Description |
|---|---|---|---|
| 1 | Tax Bolt | Projectile | Fire a bolt of pure tax law. Low cost, spammable. |
| 2 | Audit Storm | AoE | Rain audit notices in a target area. DoT field. |
| 3 | Refund Shield | Defensive | Absorb damage, return % as healing when it breaks. |
| 4 | Depreciation Beam | Channel | Beam that reduces enemy armor over time. |
| 5 | Form 1040 Barrage | Multi-projectile | Fire 5 tax forms in a spread. High burst, long cooldown. |
| 6 | Capital Loss Teleport | Mobility | Teleport to target location, leave damaging field at origin. |

### Stats
- HP: 100 (base), Mana: 80 (base)
- Movement speed: 360 units/sec
- Dodge roll: no cooldown, no i-frames (PoE2 style)
- 2 Form Slots (satirical flasks): Income Potion + Expenditure Potion

---

## 6. ENEMY DESIGN

### Basic Enemies (Demo)
| Enemy | Behavior | Satire |
|---|---|---|
| Undead Accountant | Shamble toward player, melee swipe | Risen from dead returns |
| Paper Shredder | Fast, low HP, swarm behavior | Shreds your deductions |
| Tax Collector | Chase AI, windup telegraph, strike | PoE1 Tax Collector parody |
| Ink Crawler | Ranged poison DoT projectiles | Leaves trails of red ink |
| Bureaucratic Bramble | Stationary area denial, thorns | Living wall of red tape |
| Enforcement Agent | Ranged, repositions, fires summons | Government agent with badge |
| Red Tape Weaver | Debuff caster, slows and silences | Wraps you in regulations |

### Elite Variants
- Prefix system (like PoE magic/rare monsters): "Auditing" (DoT aura), "Taxing" (damage boost), "Regulatory" (debuff on hit), "Penalizing" (reflects damage)
- Visual: glowing outline + name plate

### Boss Design Principles
1. Multi-phase with rule changes per phase (parodies PoE patch notes rewriting the fight)
2. Clear telegraphs -- dodge roll is the answer, not gear check
3. Boss speaks in patch-note phrasing: "Your dodge roll effectiveness has been reduced by 15%"
4. Each boss satirizes a specific PoE community complaint

---

## 7. SYSTEMS DESIGN

### Passive Tree: "The Tax Code"
- Start with ~200 nodes for demo (full game: 1,500 -- matching PoE's tree)
- Nodes organized by archetype: Offense, Defense, Utility, "Loopholes" (unique powerful nodes)
- Satirical node names: "Itemized Deductions," "Offshore Accounting," "Creative Bookkeeping," "Plausible Deniability"
- Dual Specialization via "Loophole Points" (parody of PoE2's Weapon Set Passives)
- Requires external tool: "Path of Filing (PoF)" -- build planner website (stretch goal)

### Currency System
| Currency | PoE Equivalent | Effect |
|---|---|---|
| Audit Notice | Chaos Orb | Reroll all mods on rare item |
| Premium Filing Fee | Exalted Orb | Add random mod ("close your eyes and file") |
| Reappraisal Order | Divine Orb | Reroll mod values |
| Amended Return | Orb of Regret | Respec passive point |
| Compliance Certificate | Orb of Alchemy | Upgrade normal to rare |
| Form W-2 | Transmutation Orb | Upgrade normal to magic |
| 1099 Contractor | Orb of Augmentation | Add mod to magic item |

### Item System
- Equipment slots: Helmet, Chest, Gloves, Boots, Weapon, Offhand, 2 Rings, Amulet, Belt
- Rarity tiers: Normal (white) > Magic (blue) > Rare (yellow) > Unique (orange) > Excessively Taxed (red, corrupted)
- Affixes themed as tax concepts: "+15% Deduction Efficiency," "Adds 5-10 Penalty Damage," "23% increased Filing Speed"
- Loot filter system: "Too. Many. Forms." settings

### Debate Club (Turn-Based Minigame)
- **Slay the Spire**-inspired card game played between runs
- Deck of "Argument" cards: Attack (damage opponent credibility), Block (defend), Draw, Special
- Resource: "Rhetoric Points" (mana that increases each turn)
- Win = earn next-run modifiers (bonus damage, extra loot, harder enemies)
- Opponents: NPCs with unique card strategies
- Card names: "Ad Hominem," "Straw Man Fallacy," "Appeal to Authority," "Burden of Proof," "Moving the Goalposts"

---

## 8. COMBAT FEEL SPEC

Combat feel is what separates a spreadsheet from a game. Every hit must communicate impact through 5-6 simultaneous feedback channels. This section defines the "juice" layer on top of the mechanical combat system.

### 8.1 Hitstop (Freeze Frames)
Every hit freezes both attacker and target for a brief moment. This is the single most important feel technique.

| Hit Type | Freeze Duration | Notes |
|---|---|---|
| Normal hit | 3 frames (50ms) | Both attacker and target freeze |
| Critical hit | 5 frames (83ms) | Slightly longer, sells the crit |
| Heavy/charged attack | 6 frames (100ms) | Player committed, reward them |
| Kill blow | 4 frames + 8 frames slow-mo (200ms at 50% speed) | Dramatic finish |
| Last enemy in room | 4 frames + 16 frames slow-mo (350ms at 30% speed) | Cathartic moment |
| Boss phase transition | 12 frames + screen flash | Spectacle |
| Player takes damage | 2 frames (33ms) | Brief -- don't make getting hit feel good |

### 8.2 Screen Shake
Directional screen shake toward the impact direction. Never random.

| Event | Intensity | Duration | Falloff |
|---|---|---|---|
| Player attack hit | 1-2 px | 4 frames | Exponential decay |
| Critical hit | 3-4 px | 6 frames | Exponential decay |
| Player takes damage | 2-3 px | 5 frames | Exponential decay |
| Boss slam attack | 6-8 px | 10 frames | Bounce (2 bounces) |
| Explosion / AoE | 4-6 px | 8 frames | Radial, exponential |
| Room clear | 3 px | 12 frames | Slow fade (celebration) |

**Implementation:** Offset the camera, not the world. Use a shake queue that sums concurrent shakes with diminishing returns (cap at 12px total).

### 8.3 Camera Punch and Zoom
| Event | Effect | Duration |
|---|---|---|
| Special ability cast | Zoom in 5% toward target | 200ms ease-out |
| Boss entrance | Zoom to boss, hold 1s, zoom back | 2s total |
| Kill blow on elite/boss | Quick zoom 8% toward corpse | 300ms, snap back |
| Dodge roll (near miss) | Micro-zoom out 2% | 150ms |
| Debate Club card play | Camera shake 1px + vignette flash | 100ms |

### 8.4 Hit Flash and Damage Feedback
| Effect | Implementation | Duration |
|---|---|---|
| Enemy hit flash | Replace all sprite pixels with white | 2 frames (33ms) |
| Enemy damage tint | Tint sprite red (lerp back to normal) | 8 frames (133ms) |
| Player damage flash | Screen-edge red vignette pulse | 12 frames (200ms) |
| Player low HP | Persistent red vignette + heartbeat screen pulse | Continuous below 25% HP |
| Chromatic aberration | On crits and boss phase transitions | 4 frames, intensity 2-4px offset |
| Death dissolve (enemies) | Sprite breaks into 8-12 particles that scatter + fade | 20 frames (333ms) |

### 8.5 Particles on Impact
Every hit spawns directional particles. This is non-negotiable.

| Event | Particle Count | Behavior |
|---|---|---|
| Normal hit | 6-10 | Burst in hit direction, gravity-affected, fade over 12 frames |
| Critical hit | 12-18 | Same but faster velocity + 2-3 larger "spark" particles |
| Kill blow | 15-25 + screen flash | Radial burst + lingering sparkles |
| Dodge roll | 4-6 dust puffs | Behind player, low velocity, fast fade |
| Ability cast | 3-5 muzzle particles | At cast point, themed by damage type |
| Fire damage | Ember particles, orange-yellow | Float upward, flicker |
| Cold/audit damage | Paper scraps / snowflake particles | Flutter down slowly |
| Bureaucracy damage | Ink splatter particles | Stick to ground briefly |

### 8.6 Animation Windows and Canceling
Every ability has defined phases. Recovery can be canceled into dodge roll (like Hades dash-cancel).

```
[Anticipation] -> [Active/Hitbox] -> [Recovery] -> [Idle]
                                      ^
                                      Can cancel into: Dodge Roll, another ability (if off cooldown)
```

| Ability | Anticipation | Active | Recovery | Total | Cancel Point |
|---|---|---|---|---|---|
| Tax Bolt | 2f (33ms) | 3f (50ms) | 4f (67ms) | 9f (150ms) | Frame 6 (after active) |
| Audit Storm | 4f (67ms) | 6f (100ms) | 6f (100ms) | 16f (267ms) | Frame 11 |
| Refund Shield | 1f (17ms) | Instant | 2f (33ms) | 3f (50ms) | Frame 2 |
| Depreciation Beam | 3f (50ms) | Channeled | 4f (67ms) | Variable | Any frame (interrupt) |
| Form 1040 Barrage | 6f (100ms) | 8f (133ms) | 8f (133ms) | 22f (367ms) | Frame 15 |
| Capital Loss Teleport | 2f (33ms) | Instant | 6f (100ms) | 8f (133ms) | Frame 4 |
| Dodge Roll | 0f | 6f (100ms) i-frames | 3f (50ms) | 9f (150ms) | Frame 7 |

**Input Buffering:** Buffer the next input during the last 4 frames of any animation. Player presses attack during dodge recovery = attack fires immediately when dodge ends. This makes the game feel faster than it is.

**Coyote Time:** 3 frames (50ms) after leaving a platform edge or valid dodge window, the action still registers. Forgiveness mechanic.

### 8.7 Sound Design for Impact
Each hit plays 2-3 layered sounds simultaneously:

| Layer | Purpose | Example |
|---|---|---|
| **Impact** | The physical collision | Meaty thud, sharp slash, crunch |
| **Reaction** | The victim's response | Enemy grunt, armor clang, paper tear |
| **Sweetener** | Extra punch for big hits | Bass boom, glass shatter, reverb tail |

**Variation:** Every sound has 3-4 variants. Pitch-shift randomly by +/-5%. Never play the same exact sound twice in a row.

**Silence before impact:** Big boss attacks have a 200-300ms audio dip (reduce music + ambient by 60%) before the hit lands. Makes the impact feel enormous.

**Satirical SFX layer:** On top of standard impact sounds, add themed sounds: rubber stamp on blocked attacks, cash register on currency pickup, "DENIED" stamp on shielded hits, typewriter clatter on Debate Club.

### 8.8 Enemy Feel
| Mechanic | Spec |
|---|---|
| **Stagger/hitstun** | Enemies freeze on hit (hitstop), then play a 3-frame recoil animation. Heavy hits = longer stagger. |
| **Knockback** | Ease-out curve (fast initial push, slow deceleration). NOT linear. Enemies pushed into walls take bonus damage (10%) + wall-slam particle burst. |
| **Death animation** | Enemies dissolve into themed particles (paper scraps for accountants, ink splatter for crawlers). No ragdoll -- pixel art looks better with dissolution. |
| **Telegraph readability** | Wind-up: enemy sprite squashes (anticipation), red ground indicator appears. Minimum 400ms telegraph for melee, 600ms for ranged AoE. Players must be able to react. |
| **Crowd management** | Max 8 enemies actively attacking at once. Others "orbit" at medium range, doing idle/patrol animations. Prevents clustered chaos while maintaining visual threat density. |

---

## 9. RUN STRUCTURE AND PROGRESSION

### 9.1 Room Selection (Pre-Room Agency)
Before each room, the player sees **2-3 doors**, each showing the reward type and threat level. This is THE core strategic decision per Hades' design. The player builds their run through choices, not random chance.

**Door display format:**
```
[DOOR 1]                    [DOOR 2]                    [DOOR 3]
Icon: Tax Form              Icon: Deductions (gold)     Icon: Skull
"Combat Room"               "Treasure Room"             "IRS Audit Room"
Reward: Tax Form Gem        Reward: 150 Deductions      Reward: Legislative Amendment
Threat: Standard            Threat: None                Threat: Hard (+50% enemy HP)
```

### 9.2 Room Types (8 Distinct Types)
| Room Type | Content | Frequency |
|---|---|---|
| **Combat** | Standard enemy encounter, 5-8 enemies | 40% |
| **Elite Combat** | Fewer enemies but 1-2 elites with affixes | 15% |
| **Treasure** | No enemies, 1 chest with currency/items, maybe trapped | 10% |
| **Shop** | Mid-run vendor, spend Deductions on Tax Forms/items/healing | 10% |
| **Event** | Narrative choice with risk/reward (satirical scenario) | 10% |
| **Challenge** | Timed or no-hit room for bonus reward | 5% |
| **Rest** | Heal 30% HP, optionally upgrade one Tax Form | 5% |
| **IRS Audit** | Hard room with premium rewards -- optional risk/reward | 5% |

### 9.3 Build-Defining Drops: "Legislative Amendments"
These are the Daedalus Hammer equivalent -- rare drops (max 2 per run) that fundamentally change HOW a skill works, not just its numbers. They make each run feel radically different.

| Amendment | Affected Skill | Effect |
|---|---|---|
| Retroactive Refund | Tax Bolt | Bolts now return to you after hitting, hitting enemies on the way back |
| Itemized Barrage | Form 1040 Barrage | Instead of 5 forms in a spread, fires 1 massive form that pierces all enemies |
| Depreciating Assets | Depreciation Beam | Beam now leaves a persistent trail on the ground that damages enemies |
| Emergency Audit | Audit Storm | Storm now follows the player instead of being placed at a fixed location |
| Capital Gains | Capital Loss Teleport | Teleport leaves a clone that attacks for 3 seconds before vanishing |
| Full Refund | Refund Shield | When shield breaks, it explodes dealing all absorbed damage to nearby enemies |
| Compound Interest | Tax Bolt | Each successive bolt against the same target deals 15% more damage (stacks 5x) |
| Double Filing | Any skill | Skill fires twice but costs 50% more mana |
| Offshore Processing | Any skill | Skill has no mana cost but a 2x longer cooldown |
| Tax Bracket Escalation | Passive | Every 10 kills in a room, all damage increases by 10% (resets per room) |

### 9.4 Event Rooms (Narrative Choices)
Satirical scenarios where the player makes a choice with mechanical consequences.

**Example events:**
- **The Suspicious Deduction:** "You find a document offering +30% damage, but it's clearly fabricated. Take it? (50% chance an IRS Audit Room spawns later)" -- risk/reward
- **The Whistleblower:** "A fellow Taxpayer offers to reveal all enemies on the minimap for the rest of the run, but their 'help' costs 100 Deductions." -- resource tradeoff
- **The Loophole:** "You discover a shortcut that skips the next room entirely, but you forfeit its reward." -- speed vs rewards
- **Close Your Eyes and File:** "An Exalted Orb -- sorry, Premium Filing Fee -- sits on a pedestal. Use it on your equipped weapon? The result is random." -- pure PoE parody
- **The Patch Notes:** A government notice appears: "Due to rebalancing, [your highest-damage skill] damage reduced by 20% for the remainder of this run. Compensation: +30% to [your lowest-damage skill]." -- forces adaptation

### 9.5 In-Run Economy
| Currency | Earned From | Spent On | Persists Between Runs? |
|---|---|---|---|
| **Deductions** | Enemy drops, room rewards | Mid-run shops, event choices | No (run-only, like Charon's Obol) |
| **Compliance Credits** | Run completion, milestones | Hub upgrades, permanent unlocks | Yes (meta-currency) |
| **Audit Notices** (Chaos Orb) | Rare drops, boss kills | Reroll item mods | Yes (crafting) |
| **Tax Form Fragments** | Room rewards, challenges | Unlock new Tax Forms in the pool | Yes (build expansion) |

### 9.6 Run Difficulty Modifiers: "Pact of Penalties"
After the first boss kill, unlock voluntary difficulty modifiers (like Hades' Pact of Punishment). Each adds challenge for bonus Compliance Credits.

| Modifier | Effect | Bonus |
|---|---|---|
| Increased Filing Burden | Enemies have +25% HP per rank (5 ranks) | +5% Credits/rank |
| Expedited Processing | Room timer (clear within 90s or enemies enrage) | +10% Credits |
| Tax Bracket Inflation | Prices in shops doubled | +8% Credits |
| Regulatory Oversight | An IRS Agent spawns every 3 rooms as a mini-boss | +15% Credits |
| Retroactive Penalties | Enemies gain damage-reflect (5% per rank) | +5% Credits/rank |
| Performance Review | Boss gains extra phase | +20% Credits |

---

## 10. SYSTEMIC SATIRE DESIGN

The difference between "a game with funny names" and "a game where the mechanics ARE the joke." At least 5 systems must embody the satire mechanically, not just textually.

### 10.1 The Tax Code (Passive Tree as Comedy)
The passive tree is deliberately, hilariously overwhelming -- and that IS the joke.

- **500+ nodes for demo** (1,500 full game), far more than needed. Most are small stat bonuses (+1% fire damage). This is intentional.
- **Contradictory nodes exist:** "Increase Audit damage by 3%" sits next to "Decrease Audit damage by 2%." Both are allocatable. The net gain is 1%. This is how tax code works.
- **Absurd tooltip lengths:** Some nodes have 200-word descriptions in legalese. "Pursuant to Section 7(b)(iii) of the Taxeclast Revenue Code, as amended by the Regulatory Reform Act of Year 47, the bearer of this allocation shall receive an increase..."
- **"Hire an Accountant" button:** Auto-allocates points optimally. The joke: nobody reads the tree, just like nobody reads the tax code. This is also a genuine accessibility feature.
- **Hidden nodes:** Some powerful nodes are tiny, buried deep in the tree, unlabeled until you hover. Reward for the obsessive player who actually reads it all -- just like real tax loopholes.
- **The tree changes subtly between runs:** "Due to the Annual Tax Reform Act, 12 nodes have been relocated." Mirrors PoE passive tree reworks that invalidate builds.

### 10.2 Mid-Run Patch Notes
Randomly (10% chance per room transition), a "Balance Patch" event triggers:

```
============================================
  PATCH 3.47.2 -- BALANCE ADJUSTMENTS
============================================

> Tax Bolt damage has been reduced by 15%.
> This is a buff.
>
> Audit Storm radius increased from "large"
> to "slightly less large."
>
> Fixed a bug where players were having fun
> in the Red Ink Vale. This was unintended.
>
> The Regulation King no longer drops useful
> items. This is working as intended.
============================================
     [ACCEPT AND CONTINUE]
```

The changes are REAL. They actually modify your stats for the remainder of the run. The player must adapt. This parodies PoE's notorious mid-league nerfs. Sometimes the "nerfs" are actually buffs (just like GGG saying "this is a buff" about nerfs).

### 10.3 The Crafting Bench: "Form 1040-EZ... Not Really"
Crafting is a deliberately obtuse bureaucratic process that parodies PoE's crafting:

1. **Select item** to modify
2. **Choose a currency** to apply (Audit Notice, Premium Filing Fee, etc.)
3. **Fill out a form** -- a small UI asking "Reason for modification" with multiple-choice options (all equally meaningless)
4. **Submit and wait** -- a 2-second "processing" animation with a progress bar
5. **Result:** random outcome, displayed as an official government notice: "Your application has been APPROVED. Result: +12 Fire Damage. Processing fee: 1 Audit Notice."

The "Close Your Eyes and File" option: apply a Premium Filing Fee with the result hidden until you equip the item. Pure PoE "close your eyes and slam" parody.

Sometimes crafting fails: "Your application has been DENIED. Reason: Insufficient documentation. Your item is unchanged. The fee is non-refundable."

### 10.4 The Compliance Meter
A persistent HUD element that tracks how "legally" you're playing:

- **High Compliance (follow rules):** Government NPCs are friendly, shops have better prices, but enemies are stronger (the government protects its own)
- **Low Compliance (exploit loopholes):** Better loot drops, but random IRS Agent encounters, shops charge more, and an "Audit" can trigger mid-combat
- **The audit:** When compliance is very low, an IRS Agent mini-boss appears mid-room. Defeating it drops premium loot + a "Plea Bargain" item. Dying to it = instant run end.

This creates a genuine risk/reward system where "cheating the system" is mechanically viable but dangerous -- exactly like real tax evasion.

### 10.5 Item Tooltips as Tax Code
Every rare+ item has an absurdly long tooltip parodying PoE's item complexity:

```
================================
AMENDED LEDGER OF FISCAL RUIN
Sceptre (Rare)
================================
Level Requirement: 12
Filing Complexity: Advanced
Audit Risk Rating: MODERATE

+42 to Bureaucracy Damage
Adds 8-15 Penalty Damage
23% increased Filing Speed
+12% Deduction Efficiency
"Per Section 401(k)(B)(iii) of the
Taxeclast Revenue Code, the bearer
may deduct up to 15% of all fire
damage received, provided said damage
was incurred in the pursuit of a
qualifying trade or business."
14% chance to Audit on Hit
================================
```

### 10.6 Loading Screen "Tips"
Useless or contradictory tips displayed during room transitions:

- "Did you know? The Tax Code is over 1,500 pages. You've read 0 of them."
- "Tip: To avoid dying, don't get hit."
- "The Compliance Manifesto states: 'Easy combat reduces player suffering, and suffering is the point.'"
- "Patch 3.47: We've made the game harder. You're welcome."
- "Fun fact: The word 'fun' does not appear anywhere in the Tax Code."
- "If you're reading this, you've been on the loading screen too long. Working as intended."
- "Tip: The Passive Tree contains exactly one node that matters. Good luck."

### 10.7 Death Screen
When you die, the screen shows:

```
YOUR TAX RETURN HAS BEEN
         REJECTED

Cause of rejection: INSUFFICIENT VITALITY
Time filed: 14:23
Forms completed: 3 of 7
Deductions claimed: 847

     "Still solvent, Taxpayer?"

   [FILE AGAIN]    [RETURN TO OFFICE]
```

This reframes death as a bureaucratic rejection -- funny enough to soften frustration, encouraging another attempt (like Hades using death as story progression).

---

## 11. META-PROGRESSION AND RETENTION

### 11.1 The Filing Cabinet (Permanent Upgrades)
The hub equivalent of Hades' Mirror of Night. Spend Compliance Credits to unlock permanent upgrades that carry across all runs.

| Upgrade | Ranks | Effect | Cost |
|---|---|---|---|
| Extended Filing Period | 5 | +5% max HP per rank | 50/100/200/400/800 |
| Audit Resistance Training | 5 | +3% damage reduction per rank | 40/80/160/320/640 |
| Speed Filing | 3 | +5% movement speed per rank | 100/250/500 |
| Form Diversity Grant | 5 | +1 Tax Form offered per room choice | 75/150/300/600/1200 |
| Debate Prep Course | 3 | +1 starting card in Debate Club | 200/500/1000 |
| Loophole Discovery | 5 | +3% chance of Legislative Amendment drop | 100/200/400/800/1600 |
| Emergency Savings | 3 | Start each run with 50/100/200 Deductions | 150/400/800 |
| Compliance Flexibility | 3 | Compliance meter changes 20% slower per rank | 100/300/700 |
| Offshore Accounts | 1 | Keep 25% of Deductions between runs | 2000 |
| Second Character Unlock | 1 | Unlock the Audit Knight class | 5000 |

### 11.2 Hub NPC Relationships
5 hub NPCs with relationship tracks that progress through dialogue and gifting. Each relationship unlocks gameplay benefits (like Hades keepsakes).

| NPC | Personality | Relationship Reward (per rank) |
|---|---|---|
| **Renly the Pencil-Pusher** | Gruff, protective, secretly caring | Rank 1: Stationery Weapon discount. Rank 3: Free weapon repair per run. Rank 5: Unique weapon unlock. |
| **Una the Tax Consultant** | Mystical, senses regulatory disturbance | Rank 1: Free Tax Form identification. Rank 3: Reveals hidden nodes on passive tree. Rank 5: Unique "Spiritual Deduction" card for Debate Club. |
| **Finn the Offshore Accountant** | Charming, shady, pathological liar | Rank 1: Gamble odds improve from 30% to 40%. Rank 3: Black market shop appears in runs. Rank 5: "Offshore" Tax Forms added to pool (high risk/high reward). |
| **The Hooded Advisor** | Ancient, enigmatic, speaks in riddles | Rank 1: Free passive respec (1 point/run). Rank 3: Reveals next 2 rooms on map. Rank 5: "Loophole" passive tree branch unlocks. |
| **Leitis the Whistleblower** | Anxious, brave, idealistic | Rank 1: Intel on next boss attack patterns. Rank 3: Compliance meter visible. Rank 5: IRS Agents have 25% less HP. |

**Gift system:** Each NPC has a preferred gift type (currency type, item rarity, etc.). Giving gifts advances the relationship. NPCs comment on your runs, react to your deaths, and have ongoing subplots.

### 11.3 Post-Run Summary Screen
After every run, show progress and hooks for the next run:

```
=== RUN REPORT ===

Rooms cleared: 5/7
Enemies defeated: 34
Damage dealt: 12,847
Cause of failure: Commissioner Geonor, Phase 3

NEW UNLOCKS:
  [!] Tax Form Fragment: 3/5 toward "Penalty Interest" (new skill)
  [!] Compliance Credits earned: 245
  [!] Renly relationship: ████████░░ (2 more runs to Rank 3)

NEXT TIME:
  "The Hooded Advisor has something to tell you..."

[FILE AGAIN]    [RETURN TO OFFICE]
```

### 11.4 Seeded Daily Runs: "Today's Tax Return"
A daily challenge run with:
- Fixed seed (everyone plays the same rooms/enemies)
- Special modifier: "Today's Tax Return: All enemies explode on death. Your dodge roll leaves fire trails."
- Personal leaderboard (time, score, rooms cleared)
- Bonus Compliance Credits for completion

### 11.5 Build Archetypes (1 Character, 5 Playstyles)
The Refund Witch can be built 5 distinct ways through passive tree + Tax Form choices:

| Archetype | Focus | Key Tax Forms | Passive Path |
|---|---|---|---|
| **Direct Filing** | High single-target DPS | Compound Interest Tax Bolt, Depreciation Beam | "Offensive Filing" branch |
| **Mass Audit** | AoE/clear speed | Emergency Audit Storm, Form 1040 Barrage | "Area of Effect" branch |
| **Defensive Accounting** | Tank/sustain | Full Refund Shield, Refund on Hit | "Risk Mitigation" branch |
| **Speed Filing** | Fast clear, hit-and-run | Capital Loss chain-teleport, reduced cooldowns | "Expedited Processing" branch |
| **Creative Bookkeeping** | Debuffs/DoT | Depreciation stacking, compliance manipulation | "Regulatory Exploitation" branch |

---

## 12. MISSING SYSTEMS

### 12.1 Minimap
Corner minimap showing room layout, visited rooms, exit locations, and current position. Essential for isometric rooms.

### 12.2 Loot Filter: "Too. Many. Forms."
Configurable loot filter -- rarity threshold, item type toggle. Satirical UI: the filter settings screen looks like a government form with checkboxes and fine print.

### 12.3 Settings Menu
| Category | Options |
|---|---|
| Video | Resolution, fullscreen, V-Sync, screen shake intensity (0-100%), pixel scaling |
| Audio | Master, music, SFX, voice volume sliders |
| Controls | Key rebinding, controller support, mouse sensitivity |
| Accessibility | Colorblind palette mode, damage number scaling, game speed slider (50-100%), aim assist toggle, screen shake disable |
| Gameplay | Loot filter presets, tutorial tooltips toggle, minimap opacity |

### 12.4 Controller Support
Gamepad input mapping with twin-stick aiming for ranged abilities. Bevy has native gamepad support.

### 12.5 Tutorial/Onboarding
First-run tutorial introduces one system per room. Delivered in-character:
- Room 1: "Welcome to the tutorial. It's mandatory. Like taxes." (Movement + attack)
- Room 2: "You've learned to fight. Now learn to dodge. Also mandatory." (Dodge roll)
- Room 3: "Here's your first Tax Form. Don't worry, nobody understands these." (Abilities)
- Hub arrival: "Welcome to the Clearfile Tax Office. File your first return." (Hub systems)

### 12.6 Pause Menu
Pause with full state freeze (single-player). Shows run stats, can access settings. In multiplayer: vote-pause or individual inventory-only pause.

---

## 13. ARCHITECTURE CLARIFICATION

### 13.1 Client-Side vs Server-Side Responsibilities
**Critical decision:** Combat runs at **client-side 60fps**. SpacetimeDB is for **persistence and multiplayer sync**, NOT as the game loop.

| Responsibility | Where It Runs | Tick Rate |
|---|---|---|
| Input processing | Client | 60fps |
| Player movement | Client (predicted), Server (validated) | 60fps / 20Hz |
| Ability execution (animation, VFX, sound) | Client (immediate) | 60fps |
| Hit detection | Client (predicted), Server (authoritative) | 60fps / 20Hz |
| Damage calculation | Server (authoritative) | 20Hz |
| Enemy AI decisions | Server | 20Hz |
| Enemy movement interpolation | Client (interpolated from server) | 60fps (display) |
| Persistence (saves, inventory, meta) | Server (SpacetimeDB) | On-event |
| State sync (multiplayer) | Server -> Client push | 20Hz |

**Why:** A 20Hz server tick (50ms) is too slow for responsive melee ARPG combat. The client must run combat presentation at 60fps. SpacetimeDB handles state truth, persistence, and multiplayer arbitration. Single-player against localhost means <1ms latency -- prediction is invisible.

**Reconciliation:** Client predicts hit immediately (hitstop, particles, sound fire on client frame). Server confirms damage within 50ms. On disagreement, client rolls back visual-only (no gameplay stutter). This is rare in single-player, relevant only in multiplayer with latency.

### 13.2 Data-Driven Tuning
All feel parameters (hitstop durations, shake intensities, particle counts, damage values, cooldowns) live in **RON files loaded at runtime**, NOT compiled constants. This enables:
- Hot-reloading during development (change a value, see it instantly)
- Balance patches without recompilation
- Community modding potential
- A/B testing different feel profiles

```
// content/feel/combat_feel.ron
CombatFeel(
    hitstop_normal_frames: 3,
    hitstop_crit_frames: 5,
    hitstop_kill_frames: 4,
    screen_shake_normal_px: 2.0,
    screen_shake_crit_px: 4.0,
    particles_per_hit: 8,
    particles_per_crit: 15,
    input_buffer_frames: 4,
    coyote_time_frames: 3,
)
```

---

## 14. ART PIPELINE

### Style Bible
- **Resolution:** 480x270 internal, rendered at 4x (1920x1080)
- **Tile size:** 64x32 (2:1 dimetric isometric)
- **Character sprites:** 48-64px tall, 8 directions (5 unique + 3 mirrors)
- **Palette:** 32-48 colors total, 16-24 per biome. Locked Lospec palette.
- **Outline:** 1px black outlines on characters/enemies, no outlines on terrain
- **Shading:** 2-3 tone cel shading
- **Reference games:** Dead Cells (animation quality), Hyper Light Drifter (color/lighting), Children of Morta (layered pixel art + dynamic lighting)

### AI Art Pipeline
1. **Style Definition** -- Write style bible, lock palette, train custom LoRA on 10-20 hand-drawn reference sprites
2. **Generation** -- PixelLab for character sprites (directional rotation, animation), Retro Diffusion for tilesets and props (palette-locked)
3. **Post-Processing** -- Import to Aseprite, palette normalization, alpha cleanup, outline consistency, grid alignment
4. **Animation** -- AI keyframes + manual Aseprite animation (onion skinning), 4-8 frames per animation
5. **Atlas Packing** -- TexturePacker or Aseprite CLI batch export to sprite sheets
6. **Bevy Integration** -- `bevy_asset_loader` for loading, `ImagePlugin::default_nearest()` for pixel-perfect rendering

### Asset Scope Estimate (Demo)
| Category | Count | Frames/Variants |
|---|---|---|
| Player character (Refund Witch) | 1 | ~250 frames (idle, walk, run, 6 attacks, dash, hit, death x 5 dirs) |
| Basic enemies | 7 types | ~100 frames each = ~700 |
| Bosses | 3 (Bloated Filer, Auditor, Regulation King) + final (Commissioner) | ~300 frames each = ~1,200 |
| Tilesets | 3 biomes | 8-12 ground + 6-10 wall + 4-6 props each = ~80 tiles |
| Props/decorations | 40-60 | Individual sprites |
| VFX | 20-30 effects | 4-8 frames each = ~160 |
| UI icons | 50-80 | Skills, items, status effects |
| NPC sprites | 4 | ~30 frames each (idle + talk) = ~120 |
| **Total** | | **~2,600-3,000 frames** |

### Bevy Rendering Setup
```rust
app.add_plugins(
    DefaultPlugins
        .set(ImagePlugin::default_nearest())    // Pixel-perfect, no filtering
        .set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1920.0, 1080.0),
                ..default()
            }),
            ..default()
        }),
);
```

- **Tilemap:** bevy_ecs_tiled (Tiled editor integration) backed by bevy_ecs_tilemap
- **Animation:** bevy_spritesheet_animation for standard anims, custom system for combat (hitbox frame sync)
- **Lighting:** HD-resolution dynamic lighting overlaid on pixel sprites (Children of Morta technique)
- **Z-sorting:** Y-based depth sorting via Transform.translation.z

---

## 15. AUDIO DESIGN

### Music
- **AI-generated** via Suno or Udio with manual curation
- **Adaptive music system:** combat intensity layers (ambient > tension > combat > boss)
- **Hub theme:** chill bureaucratic jazz (think elevator music meets dark fantasy)
- **Combat themes:** aggressive with satirical undertones (typewriter percussion, stamp rhythms)
- **Boss themes:** each boss gets a unique track with PoE-style epic orchestral + absurd bureaucratic sounds

### Sound Effects
- **AI-generated** via ElevenLabs or sound libraries (Sonniss GDC bundles)
- **Core SFX:** attacks, impacts, dodge roll, item pickup, menu clicks, ability casts, enemy deaths
- **Satirical SFX:** rubber stamp impacts, paper rustling on damage, cash register on currency pickup, "DENIED" stamp on blocked attacks
- **Voice lines:** key NPC barks ("Still solvent, Taxpayer?"), boss phase transitions

---

## 16. DEVELOPMENT MILESTONES

### Phase 0: Foundation (Weeks 1-2)
- [ ] Set up Cargo workspace (client, server, shared, tools)
- [ ] Set up SpacetimeDB local development (`spacetime dev`)
- [ ] Implement SpacetimeDB tables: Player, Character, Run, Room, ServerTick
- [ ] Implement basic reducers: init, client_connected, start_run, enter_room
- [ ] Bevy app shell: state machine, camera, basic input
- [ ] SpacetimeDB client connection + subscription bridge (client-side 60fps combat, SpacetimeDB for persistence)
- [ ] Git + Git LFS setup, CI pipeline (cargo check + clippy + test)
- [ ] Create combat_feel.ron with all hitstop/shake/particle parameters (data-driven from day 1)
- [ ] Write style bible document: palette, outline rules, shading, proportions, animation timing
- [ ] Wireframe all screens: HUD, inventory, passive tree, Debate Club, shop, settings, run summary

### Phase 1: Core Combat + Feel (Weeks 3-5)
- [ ] Player movement at 60fps with server reconciliation at 20Hz
- [ ] Dodge roll (no cooldown, PoE2 style, 6 active frames + 3 recovery, cancelable at frame 7)
- [ ] Isometric rendering + tilemap (placeholder gray box art)
- [ ] Enemy spawning from SpacetimeDB tables
- [ ] Enemy AI: chase, windup telegraph (400ms minimum), strike (server tick + client interpolation)
- [ ] Hit detection (client-predicted, server-authoritative confirmation)
- [ ] Health, damage, knockback (ease-out curve, wall-slam bonus damage)
- [ ] **Combat feel layer:** hitstop, hit flash, screen shake, directional particles on hit (all from RON config)
- [ ] **Input buffering** (4 frames) + **coyote time** (3 frames)
- [ ] **Animation cancel system:** recovery frames cancelable into dodge or next ability
- [ ] Basic HUD: health bar, mana bar, ability cooldowns, minimap
- [ ] 2 abilities: Tax Bolt (projectile) + Capital Loss Teleport (blink) with full feel spec
- [ ] **PLAYTEST CHECKPOINT:** Does hitting an enemy feel satisfying? If no, iterate before proceeding.

### Phase 2: Combat Depth + Run Structure (Weeks 6-8)
- [ ] Full Refund Witch ability kit (6 abilities with animation windows per combat feel spec)
- [ ] Ability pipeline: input > buffer > anticipation > active/hitbox > recovery > cancel window
- [ ] Damage formula (PoE-style increased/more multipliers, satirized)
- [ ] All 7 basic enemy types with unique behaviors and distinct silhouettes
- [ ] Elite enemy prefix system ("Auditing," "Taxing," "Regulatory," "Penalizing")
- [ ] Enemy stagger/hitstun, death dissolution particles, crowd cap (max 8 actively attacking)
- [ ] **Room selection system:** 2-3 doors showing reward type before entering
- [ ] **8 room types:** combat, elite, treasure, shop, event, challenge, rest, IRS audit
- [ ] Room generation from RON templates
- [ ] **Legislative Amendments:** 10 build-defining rare drops (max 2 per run)
- [ ] **Event rooms:** 5 satirical narrative choice encounters
- [ ] Loot drops: items + currency with in-run economy (Deductions)
- [ ] Inventory system + loot filter ("Too. Many. Forms.")
- [ ] VFX system: damage numbers, hit sparks, ability effects, kill slow-mo
- [ ] **Compliance Meter:** HUD element, affects shop prices and IRS Agent spawns

### Phase 3: Art + Audio (Weeks 7-10, parallel with Phase 2)
- [ ] Train custom LoRA on 10-20 hand-drawn reference sprites matching style bible
- [ ] Generate + refine Refund Witch sprites (all animations, all 5 unique directions)
- [ ] Generate + refine 7 enemy types (distinct silhouettes at 50% zoom)
- [ ] Generate 3 biome tilesets (Clearfile, Mud Bureau, Red Ink Vale)
- [ ] Props, decorations, environmental storytelling assets (signs, debris, lore objects)
- [ ] VFX sprites (abilities, impacts, status effects, dodge dust, cast muzzle flash)
- [ ] UI design: HUD, menus, inventory, passive tree, room selection doors, Debate Club
- [ ] **Adaptive music system:** combat intensity layers (ambient > tension > combat > boss)
- [ ] Hub theme: bureaucratic jazz. 3 combat tracks. 4 boss themes.
- [ ] Core SFX: 3-4 variants per hit sound, pitch variation, layered impacts
- [ ] Satirical SFX: rubber stamps, cash registers, "DENIED" stamps, typewriter clatter
- [ ] **Silence-before-impact** on boss attacks (200-300ms audio dip)

### Phase 4: Bosses + Progression + Satire Systems (Weeks 9-12)
- [ ] Boss framework: multi-phase, pattern scripts, telegraphs, arena hazards
- [ ] Implement 3 bosses with 2+ phases each and arena mechanics
- [ ] Implement final boss: Commissioner Geonor (4 phases, fog mechanic, Loan Shark transform)
- [ ] **Passive tree ("The Tax Code"):** 500 nodes with deliberate absurdity, contradictions, legalese tooltips
- [ ] "Hire an Accountant" auto-allocate button
- [ ] Equipment system: slots, affixes, rarity, absurdly long satirical tooltips
- [ ] **Crafting bench:** bureaucratic UI with form-filling, processing timer, random denials
- [ ] Currency crafting: "Close Your Eyes and File" (hidden result until equip)
- [ ] **Mid-run Patch Notes:** 10% chance per room, real stat changes with satirical framing
- [ ] **Run flow with room selection:** hub > choose modifier > 3 zones with door choices > boss > summary > hub
- [ ] **Post-run summary screen:** stats, unlock progress, NPC relationship bars, next-story teaser
- [ ] **Filing Cabinet (meta-progression):** 15-20 permanent upgrades with Compliance Credits
- [ ] **Pact of Penalties:** 6 voluntary difficulty modifiers for bonus rewards
- [ ] **Loading screen tips:** 20+ useless/contradictory/satirical tips

### Phase 5: Hub + Debate Club + Systems (Weeks 11-14)
- [ ] **Hub NPCs:** 5 characters with relationship tracks, gifting, run-reactive dialogue
- [ ] NPC dialogue system with satirical writing (react to deaths, comment on builds)
- [ ] Debate Club card game: deck, draw, play, discard, with Rhetoric Points
- [ ] 25+ debate cards (Ad Hominem, Straw Man, Appeal to Authority, Burden of Proof, etc.)
- [ ] 3 debate opponents with unique AI strategies
- [ ] Debate rewards: next-run modifiers + Tax Form pool expansion
- [ ] **Death screen:** "YOUR TAX RETURN HAS BEEN REJECTED" with stats and humor
- [ ] **Tutorial/onboarding:** first-run system-per-room introduction, in-character
- [ ] **Settings menu:** video, audio, controls, accessibility (colorblind, speed slider, shake toggle)
- [ ] **Controller support:** gamepad mapping with twin-stick aiming
- [ ] **Daily challenge runs:** "Today's Tax Return" with fixed seed and modifiers
- [ ] Screen transitions with satirical loading tips
- [ ] Pause menu with run stats

### Phase 6: Polish + Playtest (Weeks 13-16)
- [ ] Full playthrough testing: hub > room selection > 3 zones > boss > debate > repeat
- [ ] **Combat feel tuning pass:** adjust all RON parameters based on playtest feedback
- [ ] **Balance pass:** enemy HP/damage, ability cooldowns, loot rates, economy
- [ ] **Consistency audit:** all sprites on same canvas, palette check, silhouette readability
- [ ] Performance optimization (target locked 60fps, sub-100ms input latency)
- [ ] Bug fixing, edge case handling, crash prevention
- [ ] **Satirical writing pass:** ensure every UI element, tooltip, and NPC line lands

### Phase 7: Demo Ship (Weeks 15-18)
- [ ] Final playtest with external testers (5-10 people)
- [ ] Steam page setup: screenshots, trailer, description
- [ ] Demo build packaging (Linux + Windows)
- [ ] Community feedback collection + itch.io page
- [ ] Marketing: GIF captures of combat, satirical screenshots, PoE community outreach

---

## 17. PARODY REFERENCE INDEX

### Core Terminology
| PoE Term | Path of Taxation Term |
|---|---|
| Path of Exile 2 | Path of Taxation 2 |
| Wraeclast | Taxeclast |
| Exile | Taxpayer |
| Grinding Gear Games | Grinding Government Games (GGG) |
| Chris Wilson | Chris Wilson, Commissioner of Fun |
| "The Vision" | "The Policy" |
| Leagues | Fiscal Quarters |
| Maps (endgame) | Tax Districts |
| Atlas | The Revenue Atlas |
| Passive Tree | The Tax Code |
| Path of Building (PoB) | Path of Filing (PoF) |
| Flask Piano | Form Piano |
| Krangled | Audited |
| Harvest Crafting | Standard Deduction Act |
| Trade Manifesto | Compliance Manifesto |
| WASD movement | Withhold, Assess, Submit, Deduct |
| Dodge Roll | Tax Evasion Roll |
| Raise Shield | Raise Briefcase |
| Skill Gems | Tax Form Gems |
| Support Gems | Supporting Documents |
| Uncut Gems | Uncut Forms |
| 6-Link | 6 Supporting Documents |
| Salvage Bench | The Shredder |
| Chaos Orb | Audit Notice |
| Exalted Orb | Premium Filing Fee |
| Divine Orb | Reappraisal Order |
| Orb of Regret | Amended Return |

### Act 1 Zone Mapping
| PoE2 Zone | Path of Taxation Zone |
|---|---|
| Clearfell Encampment | Clearfile Tax Office |
| Mud Burrow | Mud Bureau (Underground IRS) |
| The Grelwood | Dreadwood of Deductions |
| The Red Vale | Red Ink Vale |
| Cemetery of the Eternals | Cemetery of Expired Exemptions |
| Freythorn | Freethorn (Free Tax Zone -- it's a trap) |
| Ogham Farmlands | Income Farmlands |
| Ogham Manor | The Revenue Manor |

### NPC Mapping
| PoE2 NPC | Path of Taxation NPC |
|---|---|
| Renly (Blacksmith) | Renly the Pencil-Pusher |
| Una (Caster Vendor) | Una the Tax Consultant |
| Finn (Gambler) | Finn the Offshore Accountant |
| The Hooded One / Sin | The Hooded Advisor / God of Loopholes |
| Leitis | Leitis the Whistleblower |
| Count Geonor | Commissioner Geonor, Iron Auditor |
| Oriana | Oriana, CFO (Chief Financial Offender) |

### Boss Mapping
| PoE2 Boss | Path of Taxation Boss | Key Mechanic Parody |
|---|---|---|
| The Bloated Miller | The Bloated Filer | Rubber stamp slam |
| The Devourer | The Auditor | Burrows through your records |
| The Rust King | The Regulation King | Flying penalty notices |
| The King in the Mists | The Assessor in the Fog | Compound interest bombs |
| The Crowbell | The Toll Bell | Each phase charges more |
| The Executioner | The Collections Agent | Blocks progress until you settle |
| Count Geonor | Commissioner Geonor | Transforms into Loan Shark, fog = bureaucratic jargon |

---

## 18. RISK MITIGATION

| Risk | Mitigation |
|---|---|
| Bevy 0.18 breaking changes | Pin exact version, monitor bevy_ecs_tilemap/bevy_spritesheet_animation compat |
| SpacetimeDB instability | Local-first development, save commit logs, fallback to SQLite if needed |
| SpacetimeDB 20Hz too slow for combat | Combat runs client-side at 60fps; SpacetimeDB handles persistence + sync only (see Section 13.1) |
| AI art inconsistency | Train custom LoRA early, enforce palette in CI, manual Aseprite cleanup pass per 20-30 assets |
| Scope creep | Demo = 3 zones + 1 boss. No Act 2, no multiplayer, no endgame until demo ships |
| bevy_spacetimedb not updated for 0.18 | Write thin glue layer manually (~300 lines) or fork plugin |
| Legal (PoE parody) | Satire is protected speech; no direct asset copying; all names are original parodies |
| Art volume (~3,000 frames) | AI generation reduces to ~3-4 weeks with 2-person art effort; Dead Cells 3D-to-pixel pipeline as fallback |
| Combat feels flat | Playtest checkpoint at Phase 1 end (week 5). All feel params in RON for rapid iteration. Do NOT proceed to Phase 2 until hitting feels good. |
| Satire falls flat or becomes annoying | Patch Notes and Compliance Meter are toggleable. Playtester feedback on humor pacing. |
| Run variety insufficient | Room selection + 8 room types + Legislative Amendments + Event rooms = high variance. Monitor run uniqueness in playtests. |
| Texture memory with 3,000+ frames | Atlas packing per zone, streaming strategy, power-of-two textures. Budget: 256MB GPU max. |
| Procedural room gen hitches | Generate next room async during current room. "Processing your tax return..." transition screen. |
| Hot-reload iteration speed | All tuning values in RON files loaded at runtime, not compiled. Bevy asset watcher for instant feedback. |
| Multiplayer Debate Club undefined | Deferred post-demo. Single-player only for demo. Co-op spec needed before multiplayer launch. |

---

## 19. SUCCESS CRITERIA FOR DEMO

A successful demo means a player can:

1. Start the game, arrive at Clearfile Tax Office hub
2. Talk to 4 NPCs (Renly, Una, Finn, Hooded Advisor) with satirical dialogue that reacts to their runs
3. Enter a run, **choose between room doors**, play through 3 distinct zones with unique enemies
4. Use 6 abilities as the Refund Witch with **satisfying combat feel** (hitstop, screen shake, particles on every hit)
5. Find a **Legislative Amendment** that fundamentally changes their build mid-run
6. Encounter an **Event room** with a satirical choice that has real consequences
7. Collect loot and currency, equip items, allocate passive nodes in the **deliberately absurd Tax Code tree**
8. Experience a **mid-run Patch Notes** event that changes the rules and laugh
9. Use the **crafting bench** and get a result through bureaucratic form-filling
10. Fight and defeat Commissioner Geonor in a 4-phase boss battle with arena mechanics
11. See the **death screen** ("YOUR TAX RETURN HAS BEEN REJECTED") and immediately want to try again
12. Play one round of Debate Club and earn a run modifier
13. See their **post-run summary** with progress toward next unlock and NPC relationship advancement
14. Laugh at least 5 times at PoE-specific systemic jokes (not just names, but mechanics)
15. Immediately start another run with a different build approach

### Quality Bars

**Feel bar:** Every hit communicates impact through 3+ feedback channels simultaneously (hitstop + screen shake + particles minimum). Combat feels responsive and crunchy, not floaty.

**Visual bar:** Consistent pixel art style across all assets. Dynamic lighting over sprites. Smooth animations with anticipation-action-recovery timing. Readable at 1080p.

**Satire bar:** The humor comes from mechanics (Patch Notes, Compliance Meter, Tax Code tree, crafting bench), not just text. A PoE player recognizes specific references. A non-PoE player still finds it funny.

**Performance bar:** Locked 60fps with no hitches during combat. Sub-50ms input-to-visual response. Room transitions under 500ms (with loading tip).

**Retention bar:** After a 20-minute run, the post-run summary + Filing Cabinet unlocks + NPC teaser create a "one more run" pull. 5 distinct build archetypes ensure runs feel different.

---

*This document is the source of truth for Path of Taxation development. Update it as decisions change.*
