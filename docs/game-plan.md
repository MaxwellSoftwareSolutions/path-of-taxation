# Game Plan

## Premise

`Path of Taxation` is a satirical action roguelite about surviving a world where build complexity has become religion. It is inspired by ARPG culture, not by copying another game's exact systems, setting, or content.

The player is an exile punished for asking whether any of the systems actually make sense. Each run pushes deeper into a collapsing world ruled by loot dogma, patch-note theology, and economy cults.

## Core Design Goal

The game has to work as a fun action roguelite before the satire matters.

That means the main loop must be satisfying in this order:

1. Responsive combat
2. Interesting room-to-room choices
3. Build mutation during a run
4. Short but meaningful downtime
5. Strong replay motivation

## Core Loop

1. Start in the hub.
2. Choose a starting loadout.
3. Enter a short run with 5 to 7 rooms.
4. Clear combat encounters and pick one reward after each room.
5. Reach a mini-event or elite encounter.
6. Defeat the boss or die.
7. Return to the hub.
8. Play one turn-based minigame that grants the next run a strong modifier or unlock.
9. Spend currency on sidegrade unlocks.
10. Start another run with a changed strategy.

## First Character

### The Refund Witch

A mobile spellcaster built around changing outcomes after committing to them.

Combat identity:

- Mid-range caster
- Good area control
- Rewarded for quick repositioning
- Can pivot between safe and risky skill patterns mid-run

Reason to start here:

- Spell effects are easier to prototype than weapon-heavy animation chains.
- Satire lands well through exaggerated modifier language.
- It supports obvious build variation early.

## First Combat Kit

- Basic attack: `Paper Cut`
  - Fast short-range magical projectile
- Skill 1: `Audit Beam`
  - Piercing beam with strong line damage
- Skill 2: `Refund Burst`
  - Delayed explosion that rewards enemy grouping
- Dash: `Policy Exception`
  - Short invulnerable reposition
- Panic skill: `Legacy Interaction`
  - Temporary broken synergy mode

## First Biome

### Mud Flats of Discourse

The biome should parody early-game friction and forum arguments without becoming visually noisy.

Room themes:

- Broken shoreline with ritual debris
- Crates of unsorted loot forms
- Message boards nailed into dead trees
- Shrines of contradictory advice

Enemy goals:

- Force repositioning
- Reward line attacks and controlled area denial
- Stay readable in small groups

## First Boss

### Saint Patchius, Bringer of Adjustments

Boss fantasy:

- A smug apostle of balance changes
- Speaks in patch-note phrasing while fighting
- Rewrites one combat rule each phase

Fight structure:

- Phase 1: basic projectile and area control test
- Phase 2: disables or mutates one player habit
- Phase 3: high-pressure sequence that rewards movement discipline

## First Turn-Based Minigame

### Passive Tree Debate Club

The player faces a theorycrafter NPC in a short turn-based argument duel.

The purpose is not comedy alone. It must:

- break up action pacing
- reinforce the build theme
- award strategic modifications to the next run

Rules for the first version:

- 3 to 5 turns
- simple card-style actions
- one visible NPC intent per turn
- rewards one powerful run modifier

Example player cards:

- `Math Proof`
- `Edge Case`
- `Actually Tested`
- `Streamer Citation`
- `Forum Post at 2 AM`

## Meta Progression Rules

The project should avoid permanent stat inflation as the main progression model.

Prioritize:

- new starting kits
- new passive pools
- additional debate cards
- unlockable relics
- alternate event outcomes

Avoid as core progression:

- large permanent health bonuses
- permanent damage creep
- grind-only upgrades that flatten difficulty

## Tone Rules

The satire should be sharp but playable.

Allowed:

- absurd item names
- fake sacred jargon
- unreliable expert NPCs
- over-serious dialogue about trivial optimization

Not allowed:

- unreadable UI as a joke
- intentionally annoying inventory systems
- massive VFX clutter that hurts play
- direct copying of another game's proper nouns or lore

