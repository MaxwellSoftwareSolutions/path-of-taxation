use spacetimedb::{table, Identity};

/// An item instance owned by a player (in stash or on the ground during a run).
#[table(accessor = item, public)]
pub struct Item {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub owner: Identity,
    /// Reference to item definition key.
    pub item_key: String,
    /// "normal", "magic", "rare", "unique", "excessively_taxed"
    pub rarity: String,
    pub item_level: u32,
    /// Serialized affix list (JSON string for now, kept simple).
    pub affixes_json: String,
    /// Which run this item is in, if any (None = stash).
    pub run_id: Option<u64>,
    /// Is this item currently equipped?
    pub is_equipped: bool,
}

/// A stack of currency owned by a player.
#[table(accessor = currency_stack, public)]
pub struct CurrencyStack {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub owner: Identity,
    /// e.g. "audit_notice", "premium_filing_fee", "reappraisal_order"
    pub currency_key: String,
    pub quantity: u64,
}

/// An equipment slot mapping for a character.
#[table(accessor = equipment_slot, public)]
pub struct EquipmentSlot {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub character_id: u64,
    /// "helmet", "chest", "gloves", "boots", "weapon", "offhand", "ring1", "ring2", "amulet", "belt"
    pub slot: String,
    /// Item ID equipped in this slot, if any.
    pub item_id: Option<u64>,
}
