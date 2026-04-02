use spacetimedb::{table, Identity, Timestamp};

/// An active or completed run instance.
#[table(accessor = run, public)]
pub struct Run {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub owner: Identity,
    pub character_id: u64,
    /// "active", "completed", "failed", "abandoned"
    pub status: String,
    pub seed: u64,
    pub current_room_index: u32,
    pub total_rooms: u32,
    /// Current in-run currency (Deductions).
    pub deductions: u64,
    pub kills: u64,
    pub damage_dealt: u64,
    pub damage_taken: u64,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
}

/// A room within a run, representing one encounter/event/shop.
#[table(accessor = room, public)]
pub struct Room {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub run_id: u64,
    pub room_index: u32,
    /// "combat", "elite_combat", "treasure", "shop", "event", "challenge", "rest", "irs_audit"
    pub room_type: String,
    /// "pending", "active", "completed", "failed"
    pub status: String,
    /// Number of enemies remaining (for combat rooms).
    pub enemies_remaining: u32,
    pub enemies_total: u32,
    /// Reward type shown on the door before entering.
    pub reward_type: String,
    pub created_at: Timestamp,
}

/// Historical record of completed runs for stats and meta-progression.
#[table(accessor = run_history, public)]
pub struct RunHistory {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub owner: Identity,
    pub character_id: u64,
    pub run_id: u64,
    pub rooms_cleared: u32,
    pub total_rooms: u32,
    pub kills: u64,
    pub damage_dealt: u64,
    pub compliance_credits_earned: u64,
    /// "completed", "failed", "abandoned"
    pub outcome: String,
    pub duration_secs: u64,
    pub completed_at: Timestamp,
}
