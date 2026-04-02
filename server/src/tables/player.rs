use spacetimedb::{table, Identity, Timestamp};

/// A connected player account, keyed by their SpacetimeDB identity.
#[table(accessor = player, public)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub username: String,
    pub created_at: Timestamp,
    pub last_login: Timestamp,
    /// Persistent meta-currency earned across runs.
    pub compliance_credits: u64,
    pub total_runs: u64,
    pub total_kills: u64,
    pub is_online: bool,
}

/// A playable character belonging to a player.
#[table(accessor = character, public)]
pub struct Character {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub owner: Identity,
    pub name: String,
    /// e.g. "refund_witch", "audit_knight"
    pub class_key: String,
    pub level: u32,
    pub experience: u64,
    pub base_hp: i64,
    pub base_mana: i64,
    pub passive_points_available: u32,
    pub passive_points_spent: u32,
    pub created_at: Timestamp,
}

/// Persistent unlocks that carry across all runs (Filing Cabinet upgrades, etc).
#[table(accessor = meta_unlock, public)]
pub struct MetaUnlock {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub owner: Identity,
    /// e.g. "extended_filing_period", "speed_filing"
    pub unlock_key: String,
    /// Current rank of this upgrade.
    pub rank: u32,
}
