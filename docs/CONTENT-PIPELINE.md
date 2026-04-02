# Content Pipeline Architecture

This document defines the complete data-driven content pipeline for Path of Taxation. Every game entity -- abilities, enemies, items, rooms, encounters -- is defined in RON files, loaded into Rust structs, and made available as Bevy resources. The goal: adding a new enemy or ability should never require recompiling the game during iteration.

---

## 1. Data-Driven Content System

### 1.1 Directory Layout

All content lives under `content/` at the workspace root. The directory structure mirrors the game's domain model:

```
content/
  abilities/
    refund_witch.ron          # Per-character ability kits (AbilitySet)
    audit_knight.ron          # Future character kits
    legislative_amendments.ron # Build-defining rare drops (LegislativeAmendments)
  enemies/
    act1_basic.ron            # Basic enemy archetypes per act (Act1BasicEnemies)
    act1_elites.ron           # Elite prefixes per act (Act1ElitePrefixes)
    act2_basic.ron            # Future act expansions
    bosses/
      bloated_filer.ron       # One BossDef per file
      the_auditor.ron
      regulation_king.ron
      commissioner_geonor.ron
  items/
    currency.ron              # Currency types (CurrencyTypes)
    affixes.ron               # Affix pool definitions (future)
    uniques.ron               # Unique item definitions (future)
    bases.ron                 # Base item types (future)
    drop_tables.ron           # Loot table weights (future)
  rooms/
    templates.ron             # Zone/room layout templates (RoomTemplates)
    events.ron                # Event room scenarios (EventRooms)
  debate/
    cards.ron                 # Debate Club cards (DebateCards)
  feel/
    combat_feel.ron           # Hitstop, screenshake, particles (CombatFeel)
    loading_tips.ron          # Loading screen tips (LoadingTips)
  passive_tree/
    nodes.ron                 # Passive tree node definitions (future)
    clusters.ron              # Node cluster layouts (future)
```

### 1.2 Loading Pipeline: RON -> Rust Structs -> Bevy Resources

The pipeline has four stages, all occurring during `AppState::Boot`.

#### Stage 1: File Discovery

Scan `content/` recursively for `.ron` files. Each file's top-level RON type tag determines which loader processes it:

```rust
// shared/src/content_manifest.rs

/// Every RON file declares one of these as its root type.
/// The deserializer uses the type tag to select the target struct.
#[derive(Debug, Clone, Deserialize)]
pub enum ContentRoot {
    AbilitySet(AbilitySet),
    LegislativeAmendments(LegislativeAmendmentList),
    Act1BasicEnemies(EnemyList),
    Act1ElitePrefixes(ElitePrefixList),
    BossDef(BossDef),
    CurrencyTypes(CurrencyList),
    RoomTemplates(RoomTemplateCollection),
    EventRooms(EventRoomList),
    DebateCards(DebateCardList),
    CombatFeel(CombatFeel),
    LoadingTips(LoadingTipList),
}
```

#### Stage 2: Deserialization

Each `.ron` file is read and deserialized using the `ron` crate (already in workspace dependencies). Errors at this stage are fatal and halt the game with a diagnostic message.

```rust
// client/src/content/loader.rs

use std::fs;
use std::path::Path;
use ron::de::from_str;
use pot_shared::content_manifest::ContentRoot;

pub fn load_ron_file(path: &Path) -> Result<ContentRoot, ContentLoadError> {
    let text = fs::read_to_string(path)
        .map_err(|e| ContentLoadError::Io(path.to_path_buf(), e))?;
    let root: ContentRoot = from_str(&text)
        .map_err(|e| ContentLoadError::Parse(path.to_path_buf(), e))?;
    Ok(root)
}

#[derive(Debug, thiserror::Error)]
pub enum ContentLoadError {
    #[error("Failed to read {0}: {1}")]
    Io(std::path::PathBuf, std::io::Error),
    #[error("Failed to parse {0}: {1}")]
    Parse(std::path::PathBuf, ron::error::SpannedError),
    #[error("Validation failed for {0}: {1}")]
    Validation(std::path::PathBuf, String),
}
```

#### Stage 3: Validation

After deserialization, every content definition passes through validation. Validation rules are defined per type and catch broken references, out-of-range values, and missing cross-references before the game reaches a playable state.

```rust
// shared/src/content_validate.rs

pub trait Validate {
    fn validate(&self, registry: &ContentRegistry) -> Vec<ValidationError>;
}

#[derive(Debug)]
pub struct ValidationError {
    pub file: String,
    pub key: String,
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug)]
pub enum Severity {
    Error,   // Halts loading
    Warning, // Logged but continues
}
```

Validation rules per content type:

| Type | Validation Rules |
|------|-----------------|
| `AbilityDef` | `cancel_frame <= anticipation_frames + active_frames + recovery_frames`; `cooldown_ms > 0`; `mana_cost >= 0`; projectile fields required if `ability_type == Projectile`; AoE fields required if `ability_type == AoE` |
| `EnemyDef` | `base_hp > 0`; `windup_ms >= 400` for melee, `>= 600` for ranged AoE (per design rule in act1_basic.ron); `sprite_key` resolves to an existing asset; `attack_range > 0` |
| `BossDef` | At least 2 phases; `phases[0].hp_threshold == 1.0`; each phase has at least 1 attack; all attack keys unique within a boss; `sprite_size_px` within `[32,32]..=[128,128]` |
| `ItemDef` | Equip items must have `equip_slot`; affix `min_value <= max_value`; affix `tier >= 1` |
| `RoomTemplate` | `size_tiles` within `[8,8]..=[40,40]`; combat rooms must have `spawn_points >= 1`; `enemy_pool` references validate against loaded `EnemyDef` keys |
| `EventRoom` | Each choice has at least 1 outcome; outcome weights sum to 100; effect strings parse to known effect types |

#### Stage 4: Insertion into Bevy Resources

Validated content is inserted into typed `HashMap`-backed Bevy resources, keyed by `String`:

```rust
// client/src/content/registry.rs

use bevy::prelude::*;
use std::collections::HashMap;
use pot_shared::{ability_defs::*, enemy_defs::*, item_defs::*};

#[derive(Resource, Default)]
pub struct ContentRegistry {
    pub abilities: HashMap<String, AbilityDef>,
    pub ability_sets: HashMap<String, AbilitySet>,
    pub amendments: HashMap<String, LegislativeAmendment>,
    pub enemies: HashMap<String, EnemyDef>,
    pub elite_prefixes: HashMap<String, ElitePrefix>,
    pub bosses: HashMap<String, BossDef>,
    pub currencies: HashMap<String, CurrencyDef>,
    pub room_templates: HashMap<String, RoomTemplateCollection>,
    pub event_rooms: HashMap<String, EventRoom>,
    pub debate_cards: HashMap<String, DebateCard>,
    pub combat_feel: Option<CombatFeel>,
    pub loading_tips: Vec<String>,
}

impl ContentRegistry {
    /// Look up an ability by key. Panics with a diagnostic if missing.
    pub fn ability(&self, key: &str) -> &AbilityDef {
        self.abilities.get(key)
            .unwrap_or_else(|| panic!("Missing ability def: '{}'. Check content/abilities/", key))
    }

    /// Look up an enemy by key.
    pub fn enemy(&self, key: &str) -> &EnemyDef {
        self.enemies.get(key)
            .unwrap_or_else(|| panic!("Missing enemy def: '{}'. Check content/enemies/", key))
    }

    /// Look up a boss by key.
    pub fn boss(&self, key: &str) -> &BossDef {
        self.bosses.get(key)
            .unwrap_or_else(|| panic!("Missing boss def: '{}'. Check content/enemies/bosses/", key))
    }
}
```

#### The ContentPlugin

```rust
// client/src/content/mod.rs

pub mod loader;
pub mod registry;

use bevy::prelude::*;
use crate::app_state::AppState;

pub struct ContentPlugin;

impl Plugin for ContentPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<registry::ContentRegistry>()
            .add_systems(OnEnter(AppState::Boot), load_all_content);
    }
}

fn load_all_content(mut registry: ResMut<registry::ContentRegistry>) {
    let content_dir = std::path::Path::new("content");
    loader::load_directory(content_dir, &mut registry)
        .expect("Content loading failed -- check RON files");
    let errors = registry.validate_all();
    if errors.iter().any(|e| matches!(e.severity, Severity::Error)) {
        for err in &errors {
            eprintln!("[CONTENT] {:?}: {} -- {}.{}: {}",
                err.severity, err.file, err.key, err.field, err.message);
        }
        panic!("Content validation failed with {} error(s)", errors.len());
    }
}
```

Add `ContentPlugin` to the plugin list in `client/src/main.rs`, before all game plugins so the registry is populated before any system reads it.

### 1.3 Hot-Reload Strategy

For iteration speed, the client watches `content/` for changes using `notify` (file watcher crate) and reloads modified RON files without restarting:

```rust
// client/src/content/hot_reload.rs

use bevy::prelude::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event, EventKind};
use std::sync::mpsc;

#[derive(Resource)]
pub struct ContentWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::Receiver<notify::Result<Event>>,
}

pub fn setup_content_watcher(mut commands: Commands) {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res| { let _ = tx.send(res); },
        notify::Config::default(),
    ).expect("Failed to create file watcher");
    watcher.watch(
        std::path::Path::new("content"),
        RecursiveMode::Recursive,
    ).expect("Failed to watch content directory");
    commands.insert_resource(ContentWatcher {
        _watcher: watcher,
        receiver: rx,
    });
}

pub fn poll_content_changes(
    watcher: Res<ContentWatcher>,
    mut registry: ResMut<ContentRegistry>,
) {
    while let Ok(Ok(event)) = watcher.receiver.try_recv() {
        if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
            for path in &event.paths {
                if path.extension().is_some_and(|e| e == "ron") {
                    match loader::load_ron_file(path) {
                        Ok(root) => {
                            registry.merge_single(root, path);
                            info!("[HOT-RELOAD] Reloaded: {}", path.display());
                        }
                        Err(e) => {
                            warn!("[HOT-RELOAD] Failed to reload {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }
}
```

Hot-reload runs every frame in `Update` when compiled with `#[cfg(debug_assertions)]`. It is stripped from release builds. The `CombatFeel` resource is the primary hot-reload target -- change a hitstop value in `combat_feel.ron`, see the result on the next hit.

### 1.4 Content Validation at Load Time

Beyond per-type structural validation (Section 1.2 Stage 3), the loader performs cross-reference integrity checks:

1. **Enemy-to-Sprite**: Every `EnemyDef.sprite_key` and `BossDef.sprite_key` must resolve to an existing file under `assets/sprites/`.
2. **Room-to-Enemy**: Every `enemy_key` referenced in a `RoomTemplate.enemy_pool` must exist in the loaded `EnemyDef` set.
3. **Boss-Attack-to-Definition**: Every attack key in `BossPhase.attacks` must either reference a known ability key or be flagged as a boss-specific attack (prefixed with the boss's key).
4. **Amendment-to-Ability**: Every `LegislativeAmendment.target_ability` must match a loaded `AbilityDef.key` or be `"any"` or `"passive"`.
5. **Event-Effect-Parse**: Every effect string in `EventRoom` outcomes must parse to a known effect grammar (see Section 5).

---

## 2. Enemy Pipeline

### 2.1 Adding a New Enemy Type

Checklist for adding a new basic enemy:

**Step 1: Define the archetype in RON**

Edit the appropriate act file (e.g., `content/enemies/act1_basic.ron`) and add an `EnemyDef` entry to the `enemies` array. Required fields:

```ron
EnemyDef(
    key: "new_enemy_key",           // Unique string, snake_case
    name: "Display Name",
    description: "Flavor text...",
    base_hp: 80,                     // Before zone scaling
    base_damage: 12,                 // Before zone scaling
    damage_type: Penalty,            // One of: Penalty, Audit, Freeze, Bureaucracy, Expedited, Interest
    move_speed: 100.0,               // Units/second
    behavior: Shamble,               // One of: Shamble, Swarm, Chase, Ranged, Stationary, Kiter, Debuffer
    aggro_range: 200.0,
    attack_range: 48.0,
    attack_cooldown_ms: 1500,
    windup_ms: 500,                  // Must be >= 400 for melee, >= 600 for ranged AoE
    sprite_key: "enemies/new_enemy_key",
    sprite_size_px: [48, 64],
    is_silhouette_distinct: true,    // Design gate: must be true
    deduction_drop_min: 5,
    deduction_drop_max: 15,
    item_drop_chance: 0.05,
)
```

**Step 2: Create the sprite sheet**

Place the sprite sheet at `assets/sprites/enemies/new_enemy_key.png`. See Section 2.2 for format requirements.

**Step 3: Add to room templates**

Edit the relevant zone in `content/rooms/templates.ron` and add `(enemy_key: "new_enemy_key", count_min: N, count_max: M)` entries to the `enemy_pool` arrays of appropriate rooms.

**Step 4: Validate**

Run the content validator (see Section 1.2). The game will refuse to start if the sprite key doesn't resolve or the enemy key is referenced in a room but not defined.

**Files touched:**
- `content/enemies/act{N}_basic.ron` -- add `EnemyDef`
- `assets/sprites/enemies/{key}.png` -- sprite sheet
- `content/rooms/templates.ron` -- add to room pools

No Rust code changes required for a standard enemy using an existing `EnemyBehavior` variant.

### 2.2 Sprite Sheet Requirements

All enemy sprites use a fixed sheet layout:

| Property | Requirement |
|----------|-------------|
| Format | PNG, RGBA, no premultiplied alpha |
| Tile size | Defined per enemy in `sprite_size_px: [W, H]` |
| Directions | 8 directions: E, NE, N, NW, W, SW, S, SE (one row per direction, top to bottom in this order) |
| Animations | 4 animations per direction, laid out left-to-right in columns |
| Animation order | Idle (4 frames), Walk (6 frames), Attack (8 frames), Death (6 frames) |
| Total frames | 24 columns x 8 rows = 192 tiles per sheet |
| Palette | 16-color indexed palette per enemy for consistent pixel art |
| Silhouette gate | At 50% zoom, the enemy silhouette must be distinguishable from all other enemies in the same zone |

Sheet dimensions: `(tile_width * 24) x (tile_height * 8)` pixels.

Example for an enemy with `sprite_size_px: [48, 64]`: sheet is 1152x512 px.

Boss sprites follow the same layout but with larger tile sizes (up to `[96, 112]`) and an additional animation row for phase transitions (row 9).

### 2.3 AI Behavior Configuration via RON

The `behavior` field in `EnemyDef` selects a behavior tree from a fixed set of AI archetypes. Each archetype is implemented in `client/src/plugins/enemies.rs` as a Bevy system that reads the `EnemyDef` fields to parameterize its logic:

| Behavior | Movement Pattern | Attack Pattern | Key Parameters |
|----------|-----------------|----------------|----------------|
| `Shamble` | Direct path to player at `move_speed` | Melee swipe at `attack_range` | `windup_ms`, `attack_cooldown_ms` |
| `Swarm` | Direct path, clusters with nearby swarm allies | Rapid melee at `attack_range` | `attack_cooldown_ms` (short) |
| `Chase` | Direct path, faster, never gives up | Telegraphed heavy strike | `windup_ms` (long), `aggro_range` (long) |
| `Ranged` | Advances to `attack_range`, then stops | Fires projectile at player | `attack_range`, `windup_ms` |
| `Stationary` | Does not move | Passive aura damage in `attack_range` radius | `attack_range` = aura radius |
| `Kiter` | Maintains `attack_range` distance, retreats if player closes | Fires projectile, repositions | `move_speed`, `attack_range` |
| `Debuffer` | Stays at `attack_range`, slow movement | Casts debuff at player | `attack_cooldown_ms` (long), `windup_ms` (long) |

All AI respects the `MAX_ACTIVE_ATTACKERS` (8) cap from `shared/src/constants.rs`. Enemies beyond the cap orbit at medium range in an `Idle` or `Patrol` AI state until a slot opens.

Adding a new behavior type requires:
1. Adding a variant to `EnemyBehavior` in `shared/src/enemy_defs.rs`
2. Implementing the behavior system in `client/src/plugins/enemies.rs`
3. No RON format changes needed -- just use the new variant in enemy definitions

### 2.4 Enemy Scaling Formulas

Enemies scale with zone level. The server applies these formulas when spawning `ActiveEnemy` rows; the client reads the computed values from SpacetimeDB subscriptions.

```
effective_hp    = base_hp    * (1.0 + 0.15 * zone_level) * variant_hp_multiplier
effective_damage = base_damage * (1.0 + 0.12 * zone_level) * variant_damage_multiplier
effective_speed = move_speed * (1.0 + 0.03 * zone_level)  // capped at 1.5x base
```

Where `zone_level` is:
- The Clearfile: 1
- The Mud Bureau: 2
- The Red Ink Vale: 3
- The Revenue Manor: 4
- NG+ cycles: 4 + cycle_number

Variant multipliers stack with elite prefix multipliers:

| Variant | HP Multiplier | Damage Multiplier |
|---------|--------------|-------------------|
| Normal | 1.0 | 1.0 |
| Magic (1 prefix) | prefix.hp_multiplier | prefix.damage_multiplier |
| Rare (2 prefixes) | product of both hp_multipliers | product of both damage_multipliers |
| Unique | Defined per unique enemy (fixed stats) | Defined per unique enemy |

These formulas live in `shared/src/scaling.rs` (to be created) so both client (for prediction) and server (for authoritative values) use identical math.

---

## 3. Ability/Skill Pipeline

### 3.1 Adding a New Player Ability

Checklist for adding a new ability to an existing character:

**Step 1: Define in RON**

Edit the character's ability file (e.g., `content/abilities/refund_witch.ron`) and add an `AbilityDef` to the `abilities` array. Every field is required -- use `None` for inapplicable optional fields.

```ron
AbilityDef(
    key: "new_ability",
    name: "Display Name",
    description: "Tooltip text for the player.",
    damage_type: Audit,
    ability_type: Projectile,    // Projectile | AoE | Channel | Teleport | Shield | Melee

    base_damage: 15,
    mana_cost: 10,
    cooldown_ms: 500,

    // Animation windows at 60fps
    anticipation_frames: 2,      // Windup before active hitbox
    active_frames: 3,            // Hitbox/projectile is live
    recovery_frames: 4,          // Cooldown animation
    cancel_frame: 6,             // Frame at which recovery can be canceled

    // Projectile fields (required if ability_type == Projectile)
    projectile_speed: Some(800.0),
    projectile_count: Some(1),
    projectile_spread_deg: Some(0.0),
    pierce_count: Some(0),
    projectile_lifetime_ms: Some(1200),

    // AoE fields (None for non-AoE)
    aoe_radius: None,
    aoe_duration_ms: None,
    aoe_tick_interval_ms: None,

    // Movement fields (None for non-movement)
    teleport_range: None,
    dash_speed: None,
    dash_duration_ms: None,

    // Buff/Shield fields (None for non-defensive)
    shield_amount: None,
    shield_duration_ms: None,
    buff_key: None,
)
```

**Step 2: Create VFX definition**

Add a VFX entry for the ability in the client's VFX registry. The `vfx_key` follows the pattern `cast_{ability_key}`:

The VFX system in `client/src/plugins/vfx.rs` reads `VfxEvent` rows from SpacetimeDB. The event key `cast_new_ability` triggers the appropriate particle, flash, and sound layers. Map the key to a VFX preset:

```rust
// In client/src/plugins/vfx.rs, add to the match block:
"cast_new_ability" => spawn_projectile_cast_vfx(commands, &feel, pos, damage_type),
```

**Step 3: Create audio assets**

Place sound files at:
- `assets/audio/sfx/abilities/new_ability_cast_01.ogg` (+ variants 02, 03 -- minimum 3 per `sound.min_variants_per_sfx`)
- `assets/audio/sfx/abilities/new_ability_hit_01.ogg` (+ variants)

**Step 4: Wire into server**

The server's `use_ability` reducer in `server/src/reducers/combat.rs` currently uses stub values. When content loading is implemented server-side, it will look up `AbilityDef` from the registry and use `base_damage`, `cooldown_ms`, `mana_cost`, `projectile_speed`, etc., directly. Until then, update the TODO stubs.

**Files touched:**
- `content/abilities/{character}.ron` -- add `AbilityDef`
- `assets/audio/sfx/abilities/{key}_cast_*.ogg` -- cast sounds
- `assets/audio/sfx/abilities/{key}_hit_*.ogg` -- impact sounds
- `client/src/plugins/vfx.rs` -- map VFX key to preset (if ability needs custom VFX)
- `server/src/reducers/combat.rs` -- update `use_ability` to read from content (one-time task)

### 3.2 Animation Frame Data

Every ability defines four animation phases in frame counts at 60fps:

```
|--anticipation--|--active--|--recovery--|
                                ^
                            cancel_frame
```

| Phase | Purpose | Design Rule |
|-------|---------|-------------|
| Anticipation | Visual windup before damage. Player is committed. | Spam abilities: 1-3 frames. Burst abilities: 4-8 frames. |
| Active | Hitbox/projectile is live. Damage happens here. | Duration matches the visual effect. |
| Recovery | Post-attack animation. Vulnerable window. | Longer recovery = higher commitment cost. |
| Cancel Frame | Absolute frame number where recovery can be interrupted by dodge or another ability. | Must be >= `anticipation_frames + active_frames`. Earlier cancel = more fluid, later cancel = more commitment. |

The `input_buffer_frames` value from `combat_feel.ron` (currently 4 frames / 67ms) means the next input is queued during the last 4 frames of any phase. Combined with `coyote_time_frames` (3 frames / 50ms), the effective cancel window is `cancel_frame - 4` to `cancel_frame + 3`.

Frame data is consumed by:
- **Client**: `client/src/plugins/combat.rs` -- drives sprite animation state machine, input gating, and cancel logic
- **Server**: `server/src/reducers/combat.rs` -- validates that ability use requests respect cooldown timing

### 3.3 Hitbox/Projectile/AoE Configuration

The `ability_type` field determines which subsystem processes the ability:

**Projectile** (`ability_type: Projectile`):
- Creates `Projectile` rows in SpacetimeDB
- Fields: `projectile_speed`, `projectile_count`, `projectile_spread_deg`, `pierce_count`, `projectile_lifetime_ms`
- Spread is symmetric around the aim direction. 5 projectiles at 45 degrees = 9 degrees between each
- Pierce count of 0 = destroyed on first hit. Pierce 99 = passes through everything

**AoE** (`ability_type: AoE`):
- Creates `AreaEffect` rows in SpacetimeDB
- Fields: `aoe_radius`, `aoe_duration_ms`, `aoe_tick_interval_ms`
- Ticks apply `base_damage` to all enemies within `aoe_radius` every `aoe_tick_interval_ms`
- Total damage potential: `base_damage * (aoe_duration_ms / aoe_tick_interval_ms)`

**Channel** (`ability_type: Channel`):
- Sets `active_frames` to 999 (infinite). Runs until interrupted or mana depleted.
- `mana_cost` is per-tick (consumed every `aoe_tick_interval_ms`)
- `cancel_frame: 0` allows interruption at any time during channel

**Teleport** (`ability_type: Teleport`):
- Moves player to target position (clamped to `teleport_range`)
- Can combine with AoE fields to leave a damage zone at origin (see Capital Loss Teleport)

**Shield** (`ability_type: Shield`):
- Applies `shield_amount` as an absorb buffer
- Duration: `shield_duration_ms` or until broken
- `buff_key` is inserted into the player's active buff list

**Melee** (`ability_type: Melee`):
- Uses a hitbox positioned at player's facing direction at `attack_range` distance
- Width and arc defined by the animation (no explicit RON field -- driven by sprite hitbox data)

### 3.4 VFX and Audio Triggers per Phase

Each animation phase triggers specific feedback layers. These are configured through the `CombatFeel` resource loaded from `content/feel/combat_feel.ron`:

| Phase | VFX | Audio | Camera |
|-------|-----|-------|--------|
| Anticipation start | Cast particle burst (`cast_particle_count: 4`), muzzle glow | Charge-up sound | None |
| Active start (hit connects) | Hit flash (`enemy_flash_frames: 2`), directional particles (`normal_count: 8`), damage number | Impact sound (3-layer: impact + reaction + sweetener) | Screen shake (`normal_intensity_px: 2.0`) |
| Active (critical hit) | Extended flash, extra sparks (`crit_extra_sparks: 3`), chromatic aberration | Louder impact, pitch-shifted | Stronger shake (`crit_intensity_px: 4.0`) |
| Active (kill) | Death dissolve particles (`death_dissolve_particle_count: 10`), screen flash, lingering sparkles | Kill sound, brief silence dip before boss hits | Kill zoom (`kill_zoom_percent: 8.0`), time dilation (`kill_speed: 0.5` for 200ms) |
| Recovery | None | None | None |
| Cancel (into dodge) | Dodge dust particles (`dodge_dust_count: 5`) | Whoosh | None |

All VFX triggers emit `VfxEvent` rows server-side and are rendered client-side by `client/src/plugins/vfx.rs`. The hitstop system freezes both attacker and target sprites for `hitstop.normal_frames` (3 frames / 50ms) on every hit. This is the single most important feel element.

---

## 4. Item Pipeline

### 4.1 Item Definition Format

Items are defined in `content/items/` RON files. Three categories exist:

**Base Items** (future: `content/items/bases.ron`):

```ron
ItemDef(
    key: "iron_stamp",
    name: "Iron Stamp",
    item_type: Weapon,
    equip_slot: Some(Weapon),
    level_requirement: 1,
    description: "A basic rubber stamp reinforced with iron. For when paperwork gets physical.",
    fixed_affixes: [],   // Empty for non-uniques; affixes roll randomly
)
```

**Unique Items** (future: `content/items/uniques.ron`):

```ron
ItemDef(
    key: "the_commissioners_gavel",
    name: "The Commissioner's Gavel",
    item_type: Weapon,
    equip_slot: Some(Weapon),
    level_requirement: 12,
    description: "Once wielded by Geonor himself. Each strike echoes with the weight of a thousand penalties.",
    fixed_affixes: [
        AffixDef(
            key: "gavel_penalty_damage",
            display: "+35% Penalty Damage",
            stat: "penalty_damage_percent",
            min_value: 35.0,
            max_value: 35.0,     // Fixed value for uniques
            tier: 1,
            min_item_level: 1,
            legalese: Some("Per Section 6663(a), penalties shall be assessed at the Commissioner's discretion."),
        ),
        // ... more fixed affixes
    ],
)
```

**Currency** (`content/items/currency.ron`): Already implemented -- see the existing `CurrencyDef` struct and `currency.ron`.

### 4.2 Affix System: Prefix/Suffix, Rolled Values, Tiers

Affixes are the core of item randomization. The affix pool will be defined in `content/items/affixes.ron`:

```ron
AffixPool(
    prefixes: [
        AffixDef(
            key: "filing_speed_t1",
            display: "+{value}% Filing Speed",
            stat: "attack_speed_percent",
            min_value: 5.0,
            max_value: 10.0,
            tier: 1,
            min_item_level: 1,
            legalese: Some("Per Form 1040-EZ: expedited processing authorized."),
        ),
        AffixDef(
            key: "filing_speed_t2",
            display: "+{value}% Filing Speed",
            stat: "attack_speed_percent",
            min_value: 11.0,
            max_value: 18.0,
            tier: 2,
            min_item_level: 8,
            legalese: Some("Per Form 1040-EZ(Premium): further expedited processing authorized."),
        ),
        // ... more tiers
    ],
    suffixes: [
        AffixDef(
            key: "of_compliance_t1",
            display: "+{value} Maximum HP",
            stat: "max_hp_flat",
            min_value: 10.0,
            max_value: 20.0,
            tier: 1,
            min_item_level: 1,
            legalese: Some("Compliant taxpayers live longer. Statistically."),
        ),
        // ... more suffixes
    ],
)
```

**Affix rolling rules:**

| Rarity | Prefix Slots | Suffix Slots | Source |
|--------|-------------|-------------|--------|
| Normal | 0 | 0 | No affixes |
| Magic | 0-1 | 0-1 | At least 1 total |
| Rare | 1-3 | 1-3 | 4-6 total |
| Unique | Fixed | Fixed | Defined in `ItemDef.fixed_affixes` |
| ExcessivelyTaxed | 1-3 | 1-3 | Same as Rare, plus one corrupted mod |

**Tier selection**: When rolling an affix, the system filters the pool to affixes where `min_item_level <= item_level`, then weights higher tiers slightly lower:

```
weight(tier) = 1.0 / (tier ^ 0.5)
```

This means tier 1 affixes are common, tier 4 are rare. The `Reappraisal Order` currency re-rolls values within the same tier's `[min_value, max_value]` range.

### 4.3 Drop Table Configuration

Drop tables will be defined in `content/items/drop_tables.ron`:

```ron
DropTables(
    tables: [
        // Standard enemy kill drop
        (
            key: "basic_enemy_drop",
            rolls: 1,
            entries: [
                (item_pool: "currency_common", weight: 70, count_min: 1, count_max: 3),
                (item_pool: "currency_uncommon", weight: 20, count_min: 1, count_max: 1),
                (item_pool: "equipment_normal", weight: 8, count_min: 1, count_max: 1),
                (item_pool: "equipment_magic", weight: 2, count_min: 1, count_max: 1),
            ],
        ),
        // Elite enemy drop
        (
            key: "elite_enemy_drop",
            rolls: 2,
            entries: [
                (item_pool: "currency_uncommon", weight: 40, count_min: 1, count_max: 2),
                (item_pool: "currency_rare", weight: 15, count_min: 1, count_max: 1),
                (item_pool: "equipment_magic", weight: 25, count_min: 1, count_max: 1),
                (item_pool: "equipment_rare", weight: 15, count_min: 1, count_max: 1),
                (item_pool: "equipment_unique", weight: 5, count_min: 1, count_max: 1),
            ],
        ),
        // Boss drop -- guaranteed rare+
        (
            key: "boss_drop",
            rolls: 3,
            entries: [
                (item_pool: "currency_rare", weight: 30, count_min: 2, count_max: 4),
                (item_pool: "equipment_rare", weight: 40, count_min: 1, count_max: 2),
                (item_pool: "equipment_unique", weight: 20, count_min: 1, count_max: 1),
                (item_pool: "legislative_amendment", weight: 10, count_min: 1, count_max: 1),
            ],
        ),
    ],

    // Item pools referenced above
    pools: [
        (key: "currency_common", items: ["form_w2", "1099_contractor"]),
        (key: "currency_uncommon", items: ["compliance_certificate", "amended_return"]),
        (key: "currency_rare", items: ["audit_notice", "premium_filing_fee", "reappraisal_order"]),
        (key: "equipment_normal", items: ["__any_base_item__"]),  // Special: picks random base
        (key: "equipment_magic", items: ["__any_base_item__"]),
        (key: "equipment_rare", items: ["__any_base_item__"]),
        (key: "equipment_unique", items: ["the_commissioners_gavel"]),  // Specific uniques
        (key: "legislative_amendment", items: ["__any_amendment__"]),
    ],
)
```

The `__any_base_item__` and `__any_amendment__` sentinels tell the drop system to pick randomly from all loaded base items or amendments, respectively.

### 4.4 Item-to-Stat Application Flow

When a player equips an item, stats flow through this pipeline:

```
1. Base Character Stats (from AbilitySet.base_stats in RON)
      |
      v
2. Passive Tree Modifiers (from PassiveAllocation rows in SpacetimeDB)
      |
      v
3. Equipment Affixes (from Item.affixes_json, deserialized per equipped item)
      |  - Flat additions first: +20 HP, +5 mana
      |  - Then percentage multipliers: +15% damage, +10% speed
      |  - Then override values (rare, from Legislative Amendments)
      |
      v
4. Active Buffs (from shields, event room effects, compliance bonuses)
      |
      v
5. Final Computed Stats -> Written to PlayerCombatState in SpacetimeDB
```

The stat application happens server-side in a `recompute_stats` function called by the `equip_item` and `unequip_item` reducers. The computed stats (HP, mana, move speed, etc.) are written to the `PlayerCombatState` table, which the client subscribes to.

```rust
// server/src/logic/stats.rs (to be created)

pub fn compute_player_stats(
    base: &CharacterBaseStats,      // From content registry
    passives: &[PassiveNode],       // From passive_allocation table
    equipment: &[EquippedItem],     // From item + equipment_slot tables
    buffs: &[ActiveBuff],           // Transient combat buffs
) -> ComputedStats {
    let mut stats = ComputedStats::from_base(base);

    // Phase 1: Flat additions
    for affix in all_flat_affixes(equipment) {
        stats.apply_flat(affix.stat, affix.rolled_value);
    }
    for passive in passives {
        stats.apply_flat_passive(passive);
    }

    // Phase 2: Percentage multipliers (multiplicative stacking)
    for affix in all_percent_affixes(equipment) {
        stats.apply_percent(affix.stat, affix.rolled_value);
    }

    // Phase 3: Overrides (Legislative Amendments)
    for affix in all_override_affixes(equipment) {
        stats.apply_override(affix.stat, affix.rolled_value);
    }

    // Phase 4: Active buffs
    for buff in buffs {
        stats.apply_buff(buff);
    }

    stats
}
```

---

## 5. Encounter/Room Pipeline

### 5.1 Room Layout Configuration

Room layouts are referenced by string key in `RoomTemplate` entries. Each layout key maps to a procedural generation algorithm with parameters:

```ron
// In a room template entry:
(
    key: "mud_bureau_corridor",
    layout: "narrow_corridor",    // Layout algorithm key
    size_tiles: [30, 8],          // Width x Height in tiles
    spawn_points: 4,              // Number of enemy spawn positions
    exit_count: 2,                // Number of doors out
    // ...
)
```

Layout algorithms (implemented in `client/src/plugins/run.rs`):

| Layout Key | Description | Tile Ratio |
|------------|-------------|------------|
| `wide_open` | Flat arena, no obstacles | W >= H |
| `corridor_wide` | Wide corridor, gentle walls | W >= 2*H |
| `narrow_corridor` | Long thin passage | W >= 3*H |
| `t_junction` | T-shaped intersection, 3 paths | ~1:1 |
| `scattered_cover` | Open area with destructible objects at random positions | W ~= H |
| `small_room` | Compact, for shops/treasure/rest | W, H <= 14 |
| `medium_room` | Mid-size, for elite rooms | W, H 14-20 |
| `arena_circular` | Circular arena for boss/elite fights | W ~= H |
| `gauntlet` | Linear path with hazards along the sides | W >= 2.5*H |
| `wide_open_hazards` | Open with environmental hazards at fixed positions | W >= H |

Each layout algorithm:
1. Generates a tile grid of the specified `size_tiles`
2. Places walls/boundaries
3. Places hazards from the template's `hazards` list
4. Places spawn points at valid positions (away from the player entrance, minimum spacing between spawns)
5. Places exit doors at the specified `exit_count`

Room seed = `run.seed XOR room_index`, ensuring deterministic generation from the run seed.

### 5.2 Wave/Spawn Definitions

Enemy spawning within a combat room follows wave logic:

```
Wave 1: 60% of total enemies, spawned at room entry
Wave 2: 40% of remaining, spawned when Wave 1 is 50% cleared
(Elite/Boss rooms: all enemies spawn immediately)
```

Enemy selection from the `enemy_pool`:
1. For each enemy entry in the pool, roll a count between `count_min` and `count_max`
2. Sum all counts. If total exceeds `spawn_points`, reduce proportionally
3. Place enemies at spawn points. Assign to Wave 1 or Wave 2 based on the 60/40 split
4. For `EliteCombat` rooms, apply a random `ElitePrefix` to the first enemy in the pool

```rust
// Conceptual spawn logic in server/src/logic/spawning.rs (to be created)

pub fn generate_room_enemies(
    template: &RoomTemplate,
    zone_level: u32,
    rng: &mut impl Rng,
    registry: &ContentRegistry,
) -> Vec<SpawnedEnemy> {
    let mut enemies = Vec::new();

    for pool_entry in &template.enemy_pool {
        let count = rng.gen_range(pool_entry.count_min..=pool_entry.count_max);
        let def = registry.enemy(&pool_entry.enemy_key);

        for _ in 0..count {
            let variant = if template.room_type == "elite_combat" && enemies.is_empty() {
                // First enemy in elite room gets a random prefix
                let prefix = registry.random_elite_prefix(rng);
                EnemyVariant::Magic // or Rare for 2 prefixes
            } else {
                EnemyVariant::Normal
            };

            enemies.push(SpawnedEnemy {
                enemy_key: pool_entry.enemy_key.clone(),
                variant,
                scaled_hp: scale_hp(def.base_hp, zone_level, variant),
                scaled_damage: scale_damage(def.base_damage, zone_level, variant),
                // ... other scaled fields
            });
        }
    }

    enemies.truncate(template.spawn_points as usize);
    enemies
}
```

### 5.3 Boss Encounter Scripting

Boss encounters use the `BossDef` struct from `shared/src/enemy_defs.rs`. The encounter flow:

```
1. Boss room entered
2. CombatPhase transitions to BossIntro
3. Camera zooms to boss (boss_entrance_zoom_duration_ms: 2000)
4. Aggro bark plays
5. CombatPhase transitions to BossFight
6. Phase 1 begins (hp_threshold: 1.0)

LOOP:
  - Boss selects attack from current phase's attack list (weighted random, no repeat within 2)
  - Attack windup (telegraph VFX + bark if mapped)
  - Attack execution
  - If boss HP crosses next phase threshold:
    - Hitstop (boss_transition_frames: 12)
    - Time dilation (boss_phase_speed: 0.2 for 500ms)
    - transition_text displayed if present
    - Phase transition bark
    - New phase begins, attack list changes

7. Boss HP reaches 0
  - Death bark plays
  - Time dilation (last_kill_speed: 0.3 for 350ms)
  - Boss death animation
  - Loot drops from boss_drop table
  - Room marked completed
```

Boss attacks are keyed by string and implemented as functions in `client/src/plugins/combat.rs`. Each boss has a dedicated attack module. Adding a new boss attack:

1. Add the attack key string to the boss's `BossPhase.attacks` array in RON
2. Implement the attack function in `client/src/plugins/combat.rs` (or a boss-specific submodule)
3. Map the attack key to the function in the boss attack dispatcher

Boss-specific attacks that aren't reusable across bosses are prefixed with the boss key: `paperwork_fan` belongs to `the_auditor`, `frozen_asset_wall` belongs to `commissioner_geonor`.

### 5.4 Reward/Loot Tables per Room Type

Each room type has a fixed reward structure:

| Room Type | Reward Source | Details |
|-----------|--------------|---------|
| `Combat` | Per-enemy drops + room completion bonus | Each killed enemy rolls its own `EnemyDef.item_drop_chance`. Room clear grants Deductions based on `deduction_drop_min/max` sum. |
| `EliteCombat` | `elite_enemy_drop` table + room bonus | Elite uses the elite drop table. Normal adds use `basic_enemy_drop`. Room bonus is 1.5x standard. |
| `Treasure` | Chest loot | Opens a chest with 2-4 items from the zone's treasure table. May be trapped (`hazards` list). |
| `Shop` | No free loot | Player spends Deductions to buy items at NPC-set prices. |
| `Event` | Defined per event in RON | Effects from chosen outcome (see `content/rooms/events.ron`). |
| `Challenge` | Timed clear bonus | If cleared within the timer, loot is `high` tier. If failed, `standard` tier. |
| `Rest` | HP/Mana restoration | Restore 30% HP and 50% Mana. No loot. |
| `IrsAudit` | Premium loot | All enemies are elite. Drop table is `boss_drop` tier. Extremely dangerous. |

The `reward_tier` field in room templates maps to drop table modifiers:

| Tier | Item Level Bonus | Quantity Multiplier | Rarity Weight Shift |
|------|-----------------|--------------------|--------------------|
| `none` | 0 | 0x | N/A |
| `low` | 0 | 0.5x | -10% rare chance |
| `standard` | 0 | 1.0x | baseline |
| `high` | +2 | 1.5x | +15% rare chance |
| `premium` | +4 | 2.0x | +30% rare chance |

---

## 6. Persistence Boundaries

### 6.1 What Lives on Client (Combat, Rendering, Input)

The client is authoritative for nothing. It is a renderer and input collector. It runs prediction for responsiveness but always defers to the server.

**Client-only state** (not persisted, not synced):

| System | Data | Lifetime |
|--------|------|----------|
| Rendering | Sprite animations, particle systems, camera position | Per-frame |
| Combat Feel | Hitstop timers, screen shake offsets, time dilation scale | Per-hit event |
| Input | Key states, input buffer queue, coyote time counters | Per-frame |
| Prediction | Predicted player position (before server confirms) | Until server reconciliation |
| VFX | Active particle emitters, death dissolve animations | Duration from `CombatFeel` |
| UI | Tooltip state, menu navigation, damage number floaters | Per-interaction |
| Audio | Sound playback state, music crossfade | Per-scene |

The client reads the `ContentRegistry` for ability frame data, enemy definitions, and feel parameters. It never writes to SpacetimeDB tables during combat -- it only calls reducers.

### 6.2 What Lives on Server/SpacetimeDB (Inventory, Progression, Drops, Unlocks)

**Server-authoritative persistent state:**

| Table | Persistence | Description |
|-------|-------------|-------------|
| `Player` | Permanent | Account identity, username, lifetime stats, compliance credits |
| `Character` | Permanent | Class, level, XP, passive points |
| `Item` | Permanent | All items in stash (`run_id: None`) or active in a run |
| `CurrencyStack` | Permanent | Currency quantities per player |
| `EquipmentSlot` | Permanent | What's equipped on each character |
| `SkillLoadout` | Permanent | Which abilities are slotted |
| `PassiveAllocation` | Permanent | Passive tree node allocations |
| `MetaUnlock` | Permanent | Filing Cabinet upgrades, permanent unlocks |
| `RunHistory` | Permanent | Completed/failed/abandoned run records |
| `DebateReward` | Permanent | Earned debate modifiers for next run |

**Server-authoritative transient state (active during a run):**

| Table | Persistence | Description |
|-------|-------------|-------------|
| `Run` | Per-run | Run metadata: seed, room index, status, stats |
| `Room` | Per-run | Room instances within a run |
| `ActiveEnemy` | Per-room | Living enemy instances with position, HP, AI state |
| `PlayerCombatState` | Per-run | Player position, HP, mana, compliance |
| `Projectile` | Per-room | Active projectiles in flight |
| `AreaEffect` | Per-room | Active AoE zones |
| `CooldownState` | Per-run | Ability cooldown timers |
| `DamageEvent` | Ephemeral | Pruned each server tick after client consumption |
| `VfxEvent` | Ephemeral | Pruned each server tick after client consumption |

**Server-authoritative transient state (Debate Club):**

| Table | Persistence | Description |
|-------|-------------|-------------|
| `DebateSession` | Per-session | Active debate state, hands, RP |

### 6.3 The Sync Model: When Does Client Talk to Server?

Communication follows a **reducer-call + subscription** pattern. The client never polls. SpacetimeDB subscriptions push table changes to the client automatically.

#### Client-to-Server (Reducer Calls)

The client calls reducers in response to player input. These are fire-and-forget RPC calls:

| Trigger | Reducer | Frequency |
|---------|---------|-----------|
| Player moves | `move_player(pos_x, pos_y)` | Every client frame during movement (~60Hz, batched to 20Hz server tick) |
| Player uses ability | `use_ability(ability_key, target_x, target_y)` | On input (ability use rate limited by cooldowns) |
| Player dodges | `dodge_roll(direction_x, direction_y)` | On input |
| Room door selected | `enter_room(run_id, room_index)` | Per room transition |
| Room cleared | `complete_room(run_id, room_id)` | Per room clear (server validates enemies == 0) |
| Item equipped | `equip_item(item_id, slot)` | On player action in inventory UI |
| Item picked up | `pickup_item(item_id)` | On loot interaction |
| Currency used | `use_currency(currency_key, target_item_id)` | On crafting bench interaction |
| Run started | `start_run(character_id)` | Per run start |
| Run abandoned | `abandon_run(run_id)` | On quit/disconnect |
| Debate card played | `play_debate_card(session_id, card_key)` | Per debate turn |
| Event choice made | `choose_event_option(run_id, event_key, choice_index)` | Per event room |

#### Server-to-Client (Subscriptions)

The client subscribes to rows relevant to its identity. SpacetimeDB pushes updates automatically:

| Subscription | Purpose | Update Frequency |
|--------------|---------|-----------------|
| `PlayerCombatState WHERE identity = self` | Own HP, mana, position (reconciliation) | Every server tick (20Hz) |
| `ActiveEnemy WHERE run_id = current_run` | Enemy positions, HP, AI state for rendering | Every server tick (20Hz) |
| `Projectile WHERE run_id = current_run` | Projectile positions for rendering | Every server tick (20Hz) |
| `AreaEffect WHERE run_id = current_run` | AoE zone positions for rendering | On creation/expiry |
| `DamageEvent WHERE run_id = current_run` | Damage numbers, hit feedback triggers | On damage dealt (pruned each tick) |
| `VfxEvent WHERE run_id = current_run` | VFX triggers (cast, death, phase transition) | On event (pruned each tick) |
| `Room WHERE run_id = current_run` | Room status, enemy counts, door options | On room state change |
| `Run WHERE owner = self` | Run status, deductions, kill count | On stat change |
| `Item WHERE owner = self` | Inventory changes, new drops | On item creation/modification |
| `CurrencyStack WHERE owner = self` | Currency balance updates | On currency change |

#### Timing Diagram: Ability Use

```
Client                          Server (SpacetimeDB)
  |                                  |
  |-- use_ability("tax_bolt",x,y) -->|
  |                                  |-- validate cooldown
  |   [client plays anticipation     |-- validate mana
  |    animation immediately via     |-- create Projectile row
  |    prediction]                   |-- create VfxEvent("cast_tax_bolt")
  |                                  |-- create CooldownState row
  |                                  |-- update PlayerCombatState.mana
  |<-- subscription: VfxEvent -------|
  |<-- subscription: Projectile -----|
  |<-- subscription: PlayerCombatState|
  |                                  |
  |   [client reconciles predicted   |
  |    state with server state]      |
  |                                  |
  |   ... projectile moves ...       |
  |                                  |-- server tick: move projectile
  |                                  |-- collision detected
  |                                  |-- create DamageEvent
  |                                  |-- update ActiveEnemy.current_hp
  |<-- subscription: DamageEvent ----|
  |<-- subscription: ActiveEnemy ----|
  |                                  |
  |   [client plays hit VFX,         |
  |    hitstop, screen shake,        |
  |    damage number from            |
  |    combat_feel.ron params]       |
```

#### Prediction and Reconciliation

The client predicts player movement and ability animations locally for responsiveness. When the server's `PlayerCombatState` update arrives (20Hz), the client reconciles:

1. **Position**: If server position differs from predicted by > 2 pixels, lerp toward server position over 3 frames. If > 32 pixels, snap immediately (likely a teleport or server correction).
2. **Mana/HP**: Snap to server values immediately. Show the difference as a delayed correction in the UI (e.g., mana bar adjusts smoothly).
3. **Cooldowns**: Client tracks cooldown timers locally for UI responsiveness. Server is authoritative -- if the server rejects a `use_ability` call due to cooldown, the client cancels the predicted animation.

#### Offline/Disconnect Handling

- If the client loses connection for > 5 seconds, the run is auto-paused (server sets `Run.status = "paused"`)
- On reconnect, the client re-subscribes and receives the full current state
- If disconnected for > 5 minutes, the run is auto-abandoned (server calls internal `abandon_run`)
- All persistent data (items, characters, meta-unlocks) is safe in SpacetimeDB regardless of client state

---

## Appendix A: Content File Quick Reference

Adding content without touching Rust code:

| I want to add... | Edit this file | Struct to add |
|-------------------|---------------|---------------|
| A new basic enemy | `content/enemies/act{N}_basic.ron` | `EnemyDef` |
| A new elite prefix | `content/enemies/act{N}_elites.ron` | `ElitePrefix` |
| A new boss | `content/enemies/bosses/{boss_key}.ron` (new file) | `BossDef` |
| A new ability | `content/abilities/{character}.ron` | `AbilityDef` in the `abilities` array |
| A new Legislative Amendment | `content/abilities/legislative_amendments.ron` | `LegislativeAmendment` |
| A new currency type | `content/items/currency.ron` | `CurrencyDef` |
| A new room template | `content/rooms/templates.ron` | Entry in the zone's `templates` array |
| A new event room | `content/rooms/events.ron` | Entry in the `events` array |
| A new debate card | `content/debate/cards.ron` | Entry in the `cards` array |
| A new loading tip | `content/feel/loading_tips.ron` | String in the `tips` array |
| Tune combat feel | `content/feel/combat_feel.ron` | Modify values (hot-reloadable in debug) |

## Appendix B: Naming Conventions

| Entity | Key Format | Example |
|--------|-----------|---------|
| Abilities | `snake_case`, verb-noun | `tax_bolt`, `audit_storm`, `capital_loss_teleport` |
| Enemies | `snake_case`, descriptive | `undead_accountant`, `paper_shredder` |
| Bosses | `snake_case`, title-ish | `bloated_filer`, `the_auditor`, `commissioner_geonor` |
| Items | `snake_case`, thematic | `iron_stamp`, `the_commissioners_gavel` |
| Affixes | `snake_case_tN` (N = tier) | `filing_speed_t1`, `of_compliance_t2` |
| Elite Prefixes | `snake_case`, adjective | `auditing`, `taxing`, `regulatory`, `penalizing` |
| Room Templates | `zone_type_variant` | `clearfile_tutorial_move`, `mud_bureau_corridor` |
| Sprites | `category/key` path | `enemies/undead_accountant`, `enemies/bosses/the_auditor` |
| Audio | `category/key_variant` path | `sfx/abilities/tax_bolt_cast_01` |

## Appendix C: Files to Create

The following files are referenced in this document but do not yet exist. They should be created as the content pipeline is implemented:

| File | Purpose | Priority |
|------|---------|----------|
| `shared/src/content_manifest.rs` | `ContentRoot` enum for RON type dispatch | P0 -- required for any content loading |
| `shared/src/scaling.rs` | Enemy scaling formulas shared between client and server | P0 -- required for enemy spawning |
| `shared/src/content_validate.rs` | `Validate` trait and per-type validation rules | P1 -- required before content grows |
| `client/src/content/mod.rs` | `ContentPlugin` definition | P0 |
| `client/src/content/loader.rs` | RON file discovery and deserialization | P0 |
| `client/src/content/registry.rs` | `ContentRegistry` Bevy resource | P0 |
| `client/src/content/hot_reload.rs` | File watcher for debug hot-reload | P2 -- nice to have for iteration |
| `server/src/logic/stats.rs` | `compute_player_stats` from base + equipment + passives + buffs | P1 -- required for stat application |
| `server/src/logic/spawning.rs` | Room enemy generation from templates | P1 -- required for room flow |
| `content/items/affixes.ron` | Affix pool definitions (prefix/suffix tiers) | P1 |
| `content/items/bases.ron` | Base item type definitions | P1 |
| `content/items/uniques.ron` | Unique item definitions with fixed affixes | P2 |
| `content/items/drop_tables.ron` | Loot table weights and pool references | P1 |
