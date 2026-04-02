use serde::{Deserialize, Serialize};

use crate::types::Rarity;

/// Base item definition loaded from RON content files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub key: String,
    pub name: String,
    pub item_type: ItemType,
    pub equip_slot: Option<EquipSlot>,
    pub level_requirement: u32,
    pub description: String,
    /// For unique items: fixed affixes
    pub fixed_affixes: Vec<AffixDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemType {
    Weapon,
    Armor,
    Accessory,
    Currency,
    Consumable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    Helmet,
    Chest,
    Gloves,
    Boots,
    Weapon,
    Offhand,
    Ring1,
    Ring2,
    Amulet,
    Belt,
}

/// An affix that can roll on items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffixDef {
    pub key: String,
    /// Display name, e.g., "+15% Deduction Efficiency"
    pub display: String,
    pub stat: String,
    pub min_value: f64,
    pub max_value: f64,
    pub tier: u32,
    /// Minimum item level to roll this affix
    pub min_item_level: u32,
    /// Legalese flavor text (satirical)
    pub legalese: Option<String>,
}

/// Currency type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyDef {
    pub key: String,
    pub name: String,
    pub description: String,
    /// What PoE orb this parodies
    pub poe_equivalent: String,
    pub effect: String,
    pub rarity: Rarity,
    pub stack_max: u32,
}
