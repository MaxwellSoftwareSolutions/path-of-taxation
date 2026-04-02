use spacetimedb::{table, Identity, Timestamp};

/// An active Debate Club session (Slay the Spire-style card game between runs).
#[table(accessor = debate_session, public)]
pub struct DebateSession {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub owner: Identity,
    /// Opponent NPC key.
    pub opponent_key: String,
    /// "active", "won", "lost"
    pub status: String,
    pub current_turn: u32,
    pub player_credibility: i64,
    pub opponent_credibility: i64,
    /// Rhetoric Points available this turn.
    pub rhetoric_points: i64,
    pub max_rhetoric_points: i64,
    /// Cards in hand (serialized JSON array of card keys).
    pub hand_json: String,
    /// Cards in draw pile.
    pub draw_pile_json: String,
    /// Cards in discard pile.
    pub discard_pile_json: String,
    pub started_at: Timestamp,
}

/// Rewards earned from winning a Debate Club session.
#[table(accessor = debate_reward, public)]
pub struct DebateReward {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub owner: Identity,
    pub session_id: u64,
    /// e.g. "bonus_damage_10pct", "extra_loot", "harder_enemies"
    pub modifier_key: String,
    /// Is this reward available for the next run?
    pub is_available: bool,
    pub earned_at: Timestamp,
}
