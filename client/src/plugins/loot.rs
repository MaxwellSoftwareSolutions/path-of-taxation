use std::collections::HashMap;

use bevy::prelude::*;
use rand::Rng;

use pot_shared::item_defs::EquipSlot;

use crate::app_state::AppState;
use crate::components::items::*;
use crate::components::player::Player;
use crate::plugins::enemies::EnemyDeathMsg;
use crate::rendering::isometric::WorldPosition;

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inventory>()
            .init_resource::<InventoryOpen>()
            .add_systems(Update, (
                loot_drop_system,
                loot_bob_system,
                loot_pickup_system,
                inventory_toggle_system,
                inventory_ui_system,
                inventory_click_system,
            ).run_if(in_state(AppState::Run)))
            .add_systems(OnExit(AppState::Run), cleanup_inventory_ui);
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Player inventory: equipped items, backpack, and currency.
#[derive(Resource, Debug)]
pub struct Inventory {
    /// Equipped items keyed by slot name (e.g. "Weapon", "Helmet").
    pub equipped: HashMap<String, ItemInstance>,
    /// Unequipped items in the backpack.
    pub backpack: Vec<ItemInstance>,
    /// Currency (tax deductions) collected.
    pub deductions: u32,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            equipped: HashMap::new(),
            backpack: Vec::new(),
            deductions: 0,
        }
    }
}

/// Whether the inventory overlay is currently visible.
#[derive(Resource, Default)]
pub struct InventoryOpen(pub bool);

// ---------------------------------------------------------------------------
// Marker components for inventory UI
// ---------------------------------------------------------------------------

/// Root node for the inventory overlay (despawned on close).
#[derive(Component)]
pub struct InventoryUI;

/// Marker for an equipment slot button in the UI.
#[derive(Component)]
pub struct EquipSlotButton {
    pub slot_name: String,
}

/// Marker for a backpack item button in the UI.
#[derive(Component)]
pub struct BackpackItemButton {
    pub index: usize,
}

// ---------------------------------------------------------------------------
// Item generation
// ---------------------------------------------------------------------------

/// All possible affix templates.
const AFFIX_TEMPLATES: &[(&str, &str)] = &[
    ("+X% damage", "damage_pct"),
    ("+X HP", "hp_flat"),
    ("+X% speed", "speed_pct"),
    ("+X mana", "mana_flat"),
    ("+X% crit", "crit_pct"),
    ("+X armor", "armor_flat"),
    ("+X% dodge", "dodge_pct"),
    ("+X life regen", "life_regen"),
];

/// All possible equipment slot names (matching EquipSlot variants we use).
const SLOT_POOL: &[EquipSlot] = &[
    EquipSlot::Weapon,
    EquipSlot::Helmet,
    EquipSlot::Chest,
    EquipSlot::Boots,
    EquipSlot::Ring1,
    EquipSlot::Amulet,
];

/// Slot name for display and HashMap key.
fn slot_display_name(slot: EquipSlot) -> &'static str {
    match slot {
        EquipSlot::Weapon => "Weapon",
        EquipSlot::Helmet => "Helmet",
        EquipSlot::Chest => "Body",
        EquipSlot::Boots => "Boots",
        EquipSlot::Ring1 | EquipSlot::Ring2 => "Ring",
        EquipSlot::Amulet => "Amulet",
        EquipSlot::Gloves => "Gloves",
        EquipSlot::Offhand => "Offhand",
        EquipSlot::Belt => "Belt",
    }
}

/// Generate a random item.
fn generate_item() -> ItemInstance {
    let mut rng = rand::rng();

    // Pick rarity.
    let rarity_roll: f32 = rng.random();
    let rarity = if rarity_roll < 0.05 {
        ItemRarity::Unique
    } else if rarity_roll < 0.20 {
        ItemRarity::Rare
    } else if rarity_roll < 0.50 {
        ItemRarity::Magic
    } else {
        ItemRarity::Normal
    };

    // Pick slot.
    let slot_idx = rng.random_range(0..SLOT_POOL.len());
    let slot = SLOT_POOL[slot_idx];

    // Roll affixes based on rarity.
    let affix_count = match rarity {
        ItemRarity::Normal => 0,
        ItemRarity::Magic => rng.random_range(1..=2_u32),
        ItemRarity::Rare => rng.random_range(2..=3),
        ItemRarity::Unique => rng.random_range(2..=3),
    };

    let mut affixes = Vec::new();
    let mut used_indices: Vec<usize> = Vec::new();
    for _ in 0..affix_count {
        // Pick a unique affix template.
        let mut idx = rng.random_range(0..AFFIX_TEMPLATES.len());
        let mut attempts = 0;
        while used_indices.contains(&idx) && attempts < 20 {
            idx = rng.random_range(0..AFFIX_TEMPLATES.len());
            attempts += 1;
        }
        used_indices.push(idx);

        let (name_template, stat) = AFFIX_TEMPLATES[idx];
        let value = rng.random_range(1.0..=20.0_f32);
        let name = name_template.replace("X", &format!("{:.0}", value));

        affixes.push(ItemAffix {
            name,
            stat: stat.to_string(),
            value,
        });
    }

    // Generate item name.
    let base_name = slot_display_name(slot);
    let name = match rarity {
        ItemRarity::Normal => format!("{}", base_name),
        ItemRarity::Magic => format!("Enchanted {}", base_name),
        ItemRarity::Rare => format!("Superior {}", base_name),
        ItemRarity::Unique => format!("Legendary {}", base_name),
    };

    let base_damage = if slot == EquipSlot::Weapon {
        rng.random_range(5.0..=25.0)
    } else {
        0.0
    };

    ItemInstance {
        name,
        rarity,
        slot,
        affixes,
        base_damage,
    }
}

/// Get the color associated with an item rarity.
fn rarity_color(rarity: ItemRarity) -> Color {
    match rarity {
        ItemRarity::Normal => Color::WHITE,
        ItemRarity::Magic => Color::srgb(0.3, 0.5, 1.0),
        ItemRarity::Rare => Color::srgb(1.0, 1.0, 0.2),
        ItemRarity::Unique => Color::srgb(1.0, 0.6, 0.1),
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Listen for enemy deaths and spawn loot drops.
fn loot_drop_system(
    mut death_msgs: MessageReader<EnemyDeathMsg>,
    mut commands: Commands,
) {
    let mut rng = rand::rng();

    for msg in death_msgs.read() {
        // Always spawn currency.
        let gold_amount = rng.random_range(1..=5_u32);
        commands.spawn((
            LootDrop { pickup_radius: 35.0 },
            CurrencyDrop { amount: gold_amount },
            LootBob { phase: rng.random_range(0.0..std::f32::consts::TAU) },
            WorldPosition::new(
                msg.position.x + rng.random_range(-15.0..15.0_f32),
                msg.position.y + rng.random_range(-15.0..15.0_f32),
            ),
            Sprite {
                color: Color::srgb(2.0, 1.8, 0.3), // HDR gold glow
                custom_size: Some(Vec2::new(10.0, 10.0)),
                ..default()
            },
            Transform::default(),
        ));

        // 25% chance to drop an item.
        if rng.random_range(0.0..1.0_f32) < 0.25 {
            let item = generate_item();
            let sprite_color = match item.rarity {
                ItemRarity::Normal => Color::srgb(1.5, 1.5, 1.5),
                ItemRarity::Magic => Color::srgb(0.5, 0.8, 2.5),
                ItemRarity::Rare => Color::srgb(2.5, 2.5, 0.4),
                ItemRarity::Unique => Color::srgb(2.5, 1.2, 0.2),
            };

            commands.spawn((
                LootDrop { pickup_radius: 30.0 },
                ItemDrop { item },
                LootBob { phase: rng.random_range(0.0..std::f32::consts::TAU) },
                WorldPosition::new(
                    msg.position.x + rng.random_range(-20.0..20.0_f32),
                    msg.position.y + rng.random_range(-20.0..20.0_f32),
                ),
                Sprite {
                    color: sprite_color,
                    custom_size: Some(Vec2::new(14.0, 14.0)),
                    ..default()
                },
                Transform::default(),
            ));
        }
    }
}

/// Bobbing animation for loot drops.
fn loot_bob_system(
    mut query: Query<(&mut LootBob, &mut Transform), With<LootDrop>>,
    time: Res<Time>,
) {
    for (mut bob, mut transform) in &mut query {
        bob.phase += time.delta_secs() * 3.0;
        // Nudge y for a floating effect. depth_sort_system (PostUpdate) sets the base
        // position from WorldPosition; this additive offset runs in Update and produces
        // a subtle 1-frame-lagged bob that looks fine in practice.
        let bob_offset = bob.phase.sin() * 3.0;
        transform.translation.y += bob_offset;
    }
}

/// Check player proximity to loot drops and pick them up.
fn loot_pickup_system(
    player_query: Query<&WorldPosition, With<Player>>,
    loot_query: Query<(Entity, &WorldPosition, &LootDrop, Option<&CurrencyDrop>, Option<&ItemDrop>)>,
    mut inventory: ResMut<Inventory>,
    mut run_state: Option<ResMut<crate::plugins::run::RunStateRes>>,
    mut commands: Commands,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };

    for (entity, loot_pos, loot_drop, currency, item) in &loot_query {
        let dist = player_pos.distance_to(loot_pos);
        if dist < loot_drop.pickup_radius {
            // Pick up currency.
            if let Some(currency) = currency {
                inventory.deductions += currency.amount;
                // Also track in run stats for the results screen.
                if let Some(ref mut run) = run_state {
                    run.deductions_earned += currency.amount;
                }
            }

            // Pick up item.
            if let Some(item_drop) = item {
                inventory.backpack.push(item_drop.item.clone());
            }

            commands.entity(entity).despawn();
        }
    }
}

/// Toggle inventory open/closed with Tab key.
fn inventory_toggle_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut inv_open: ResMut<InventoryOpen>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        inv_open.0 = !inv_open.0;
    }
}

/// Build or tear down the inventory UI based on InventoryOpen state.
fn inventory_ui_system(
    inv_open: Res<InventoryOpen>,
    inventory: Res<Inventory>,
    existing_ui: Query<Entity, With<InventoryUI>>,
    mut commands: Commands,
) {
    if !inv_open.is_changed() && !inventory.is_changed() {
        return;
    }

    // Always clean up existing UI first.
    for entity in &existing_ui {
        commands.entity(entity).despawn();
    }

    if !inv_open.0 {
        return;
    }

    // Build the inventory overlay.
    commands
        .spawn((
            InventoryUI,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(40.0),
                ..default()
            },
        ))
        .with_children(|root| {
            // Left panel: Equipment slots.
            root.spawn(Node {
                width: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|panel| {
                // Title.
                panel.spawn((
                    Text::new("EQUIPMENT"),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.8, 0.6)),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));

                // Deductions counter.
                panel.spawn((
                    Text::new(format!("Deductions: {}", inventory.deductions)),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::srgb(2.0, 1.8, 0.3)),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));

                // Equipment slot buttons.
                let slots = ["Weapon", "Helmet", "Body", "Boots", "Ring", "Amulet"];
                for slot_name in &slots {
                    let equipped_text = if let Some(item) = inventory.equipped.get(*slot_name) {
                        format!("{}: {}", slot_name, item.name)
                    } else {
                        format!("{}: Empty", slot_name)
                    };

                    let text_color = inventory.equipped.get(*slot_name)
                        .map(|item| rarity_color(item.rarity))
                        .unwrap_or(Color::srgb(0.5, 0.5, 0.5));

                    panel.spawn((
                        EquipSlotButton { slot_name: slot_name.to_string() },
                        Button,
                        BackgroundColor(Color::srgb(0.15, 0.15, 0.18)),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(10.0)),
                            ..default()
                        },
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(equipped_text),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(text_color),
                        ));
                    });
                }
            });

            // Right panel: Backpack.
            root.spawn(Node {
                width: Val::Px(350.0),
                max_height: Val::Px(500.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|panel| {
                // Title.
                panel.spawn((
                    Text::new("BACKPACK"),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.8, 0.6)),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));

                if inventory.backpack.is_empty() {
                    panel.spawn((
                        Text::new("No items"),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    ));
                } else {
                    for (i, item) in inventory.backpack.iter().enumerate() {
                        let display = if item.affixes.is_empty() {
                            format!("{} [{}]", item.name, slot_display_name(item.slot))
                        } else {
                            let affix_str: Vec<&str> = item.affixes.iter().map(|a| a.name.as_str()).collect();
                            format!("{} [{}] ({})", item.name, slot_display_name(item.slot), affix_str.join(", "))
                        };

                        panel.spawn((
                            BackpackItemButton { index: i },
                            Button,
                            BackgroundColor(Color::srgb(0.12, 0.12, 0.15)),
                            Node {
                                width: Val::Percent(100.0),
                                min_height: Val::Px(30.0),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(8.0)),
                                ..default()
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new(display),
                                TextFont { font_size: 14.0, ..default() },
                                TextColor(rarity_color(item.rarity)),
                            ));
                        });
                    }
                }
            });
        });
}

/// Handle clicks on inventory UI buttons.
fn inventory_click_system(
    equip_query: Query<(&Interaction, &EquipSlotButton), Changed<Interaction>>,
    backpack_query: Query<(&Interaction, &BackpackItemButton), Changed<Interaction>>,
    mut inventory: ResMut<Inventory>,
) {
    // Click on equipped slot -> unequip to backpack.
    for (interaction, slot_btn) in &equip_query {
        if *interaction == Interaction::Pressed {
            if let Some(item) = inventory.equipped.remove(&slot_btn.slot_name) {
                inventory.backpack.push(item);
            }
        }
    }

    // Click on backpack item -> equip to matching slot.
    for (interaction, bp_btn) in &backpack_query {
        if *interaction == Interaction::Pressed {
            if bp_btn.index < inventory.backpack.len() {
                let item = inventory.backpack.remove(bp_btn.index);
                let slot_key = slot_display_name(item.slot).to_string();

                // If something is already equipped in that slot, move it to backpack.
                if let Some(old_item) = inventory.equipped.remove(&slot_key) {
                    inventory.backpack.push(old_item);
                }

                inventory.equipped.insert(slot_key, item);
            }
        }
    }
}

/// Clean up inventory UI when leaving Run state.
fn cleanup_inventory_ui(
    mut commands: Commands,
    query: Query<Entity, With<InventoryUI>>,
    loot_query: Query<Entity, With<LootDrop>>,
    mut inv_open: ResMut<InventoryOpen>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    for entity in &loot_query {
        commands.entity(entity).despawn();
    }
    inv_open.0 = false;
}
