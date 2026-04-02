use spacetimedb::{table, Identity, Timestamp};

/// The skill loadout for a character (which abilities are equipped in which slots).
#[table(accessor = skill_loadout, public)]
pub struct SkillLoadout {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub character_id: u64,
    /// Slot number (0-5 for 6 ability slots).
    pub slot_index: u32,
    /// Ability key from content definitions, e.g. "tax_bolt", "audit_storm".
    pub ability_key: String,
}

/// A passive tree node allocation for a character.
#[table(accessor = passive_allocation, public)]
pub struct PassiveAllocation {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub character_id: u64,
    /// Node ID in the passive tree.
    pub node_key: String,
    pub allocated_at: Timestamp,
}

/// Active cooldown state for abilities during a run.
#[table(accessor = cooldown_state, public)]
pub struct CooldownState {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    pub identity: Identity,
    #[index(btree)]
    pub run_id: u64,
    pub ability_key: String,
    pub cooldown_expires_at: Timestamp,
}
