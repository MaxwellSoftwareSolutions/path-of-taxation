use bevy::prelude::*;
use pot_shared::item_defs::EquipSlot;

/// Marks an entity as a loot drop that can be picked up.
#[derive(Component, Clone, Debug)]
pub struct LootDrop {
    pub pickup_radius: f32,
}

impl Default for LootDrop {
    fn default() -> Self {
        Self { pickup_radius: 30.0 }
    }
}

/// Rarity tier for dropped items (client-side rendering).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemRarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

/// A rolled affix on an item instance.
#[derive(Clone, Debug)]
pub struct ItemAffix {
    pub name: String,
    pub stat: String,
    pub value: f32,
}

/// A concrete item with rolled affixes, ready for the inventory.
#[derive(Clone, Debug)]
pub struct ItemInstance {
    pub name: String,
    pub rarity: ItemRarity,
    pub slot: EquipSlot,
    pub affixes: Vec<ItemAffix>,
    pub base_damage: f32,
}

/// Marks a loot drop entity as currency (gold).
#[derive(Component, Clone, Debug)]
pub struct CurrencyDrop {
    pub amount: u32,
}

/// Marks a loot drop entity as an item (carries the generated item data).
#[derive(Component, Clone, Debug)]
pub struct ItemDrop {
    pub item: ItemInstance,
}

/// Timer for the bobbing animation on loot drops.
#[derive(Component, Clone, Debug)]
pub struct LootBob {
    pub phase: f32,
}

impl Default for LootBob {
    fn default() -> Self {
        Self { phase: 0.0 }
    }
}
