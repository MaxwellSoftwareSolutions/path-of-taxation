use spacetimedb::{table, Identity, Timestamp, ScheduleAt};

// The scheduled table needs the reducer in scope.
use crate::reducers::lifecycle::process_server_tick;

/// Transient damage event for client-side display (damage numbers, combat log).
/// Pruned each server tick.
#[table(accessor = damage_event, public)]
pub struct DamageEvent {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub run_id: u64,
    /// Who dealt the damage (player identity or None for enemy).
    pub source_identity: Option<Identity>,
    pub source_enemy_id: Option<u64>,
    /// Who received the damage.
    pub target_identity: Option<Identity>,
    pub target_enemy_id: Option<u64>,
    pub damage: i64,
    /// "penalty", "audit", "freeze", "bureaucracy", "expedited", "interest"
    pub damage_type: String,
    pub is_critical: bool,
    pub is_kill: bool,
    pub pos_x: f32,
    pub pos_y: f32,
    pub created_at: Timestamp,
}

/// Transient VFX trigger event for client-side rendering.
/// Pruned each server tick.
#[table(accessor = vfx_event, public)]
pub struct VfxEvent {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub run_id: u64,
    /// VFX key, e.g. "hit_flash", "death_dissolve", "ability_cast"
    pub vfx_key: String,
    pub pos_x: f32,
    pub pos_y: f32,
    /// Optional intensity/scale.
    pub intensity: f32,
    pub created_at: Timestamp,
}

/// In-game chat message (for future multiplayer).
#[table(accessor = chat_message, public)]
pub struct ChatMessage {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub sender: Identity,
    pub sender_name: String,
    pub content: String,
    pub sent_at: Timestamp,
}

/// Server tick schedule table for the 20Hz game loop.
#[table(accessor = server_tick_schedule, public, scheduled(process_server_tick))]
pub struct ServerTickSchedule {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}
