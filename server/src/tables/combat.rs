use spacetimedb::{table, Identity, Timestamp};

/// An enemy instance currently alive in a run.
#[table(accessor = active_enemy, public)]
pub struct ActiveEnemy {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub run_id: u64,
    pub room_id: u64,
    /// Reference to enemy archetype definition key.
    pub enemy_key: String,
    /// "normal", "magic", "rare", "unique"
    pub variant: String,
    pub pos_x: f32,
    pub pos_y: f32,
    pub current_hp: i64,
    pub max_hp: i64,
    /// "idle", "patrol", "chase", "windup", "attack", "recover", "flee", "staggered"
    pub ai_state: String,
    pub target_x: f32,
    pub target_y: f32,
    pub last_attack_at: Timestamp,
}

/// Combat state for the player during a run (position, HP, mana, buffs).
#[table(accessor = player_combat_state, public)]
pub struct PlayerCombatState {
    #[primary_key]
    pub identity: Identity,
    #[index(btree)]
    pub run_id: u64,
    pub pos_x: f32,
    pub pos_y: f32,
    pub current_hp: i64,
    pub max_hp: i64,
    pub current_mana: i64,
    pub max_mana: i64,
    pub move_speed: f32,
    pub is_dodging: bool,
    pub facing_angle: f32,
    /// Compliance meter value (0-100).
    pub compliance: u32,
    pub last_damage_at: Option<Timestamp>,
}

/// An active projectile in flight.
#[table(accessor = projectile, public)]
pub struct Projectile {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub run_id: u64,
    pub owner_identity: Option<Identity>,
    pub owner_enemy_id: Option<u64>,
    /// Ability key that fired this projectile.
    pub ability_key: String,
    pub pos_x: f32,
    pub pos_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub damage: i64,
    pub pierce_remaining: u32,
    pub created_at: Timestamp,
    pub lifetime_ms: u64,
}

/// An active area-of-effect zone.
#[table(accessor = area_effect, public)]
pub struct AreaEffect {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub run_id: u64,
    pub owner_identity: Option<Identity>,
    pub ability_key: String,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub damage_per_tick: i64,
    pub tick_interval_ms: u64,
    pub duration_ms: u64,
    pub created_at: Timestamp,
}
