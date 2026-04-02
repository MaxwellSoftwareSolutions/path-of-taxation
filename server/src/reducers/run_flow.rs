use spacetimedb::{reducer, ReducerContext, Table};

use crate::tables::player::player;
use crate::tables::player::character;
use crate::tables::run::{run, room, run_history, Run, Room, RunHistory};
use crate::tables::combat::{
    active_enemy, area_effect, player_combat_state, projectile, PlayerCombatState,
};

/// Start a new run with the given character.
#[reducer]
pub fn start_run(ctx: &ReducerContext, character_id: u64) -> Result<(), String> {
    let sender = ctx.sender();

    // Validate the player exists.
    let existing_player = ctx
        .db
        .player()
        .identity()
        .find(sender)
        .ok_or("Player not found. Connect first.")?;

    // Validate the character belongs to this player.
    let character_row = ctx
        .db
        .character()
        .id()
        .find(character_id)
        .ok_or("Character not found.")?;
    if character_row.owner != sender {
        return Err("Character does not belong to you.".into());
    }

    // Check there is no active run for this player already.
    let has_active_run = ctx
        .db
        .run()
        .iter()
        .any(|r| r.owner == sender && r.status == "active");
    if has_active_run {
        return Err("You already have an active run. Abandon or complete it first.".into());
    }

    // Create the run. Use a simple seed from the timestamp.
    let seed = ctx.timestamp.to_micros_since_unix_epoch() as u64;
    let total_rooms = 7; // Standard run length.

    let inserted_run = ctx.db.run().insert(Run {
        id: 0,
        owner: sender,
        character_id,
        status: "active".into(),
        seed,
        current_room_index: 0,
        total_rooms,
        deductions: 0,
        kills: 0,
        damage_dealt: 0,
        damage_taken: 0,
        started_at: ctx.timestamp,
        ended_at: None,
    });

    // Initialize the player's combat state for this run.
    // Remove any stale combat state first.
    if ctx.db.player_combat_state().identity().find(sender).is_some() {
        ctx.db.player_combat_state().identity().delete(sender);
    }

    ctx.db.player_combat_state().insert(PlayerCombatState {
        identity: sender,
        run_id: inserted_run.id,
        pos_x: 0.0,
        pos_y: 0.0,
        current_hp: character_row.base_hp,
        max_hp: character_row.base_hp,
        current_mana: character_row.base_mana,
        max_mana: character_row.base_mana,
        move_speed: 360.0,
        is_dodging: false,
        facing_angle: 0.0,
        compliance: 50,
        last_damage_at: None,
    });

    // Update the player's total runs counter.
    let mut updated_player = existing_player;
    updated_player.total_runs += 1;
    ctx.db.player().identity().update(updated_player);

    log::info!("Run {} started for player {:?}", inserted_run.id, sender);
    Ok(())
}

/// Enter a room by index within the current run. Creates room if needed.
#[reducer]
pub fn enter_room(ctx: &ReducerContext, run_id: u64, room_index: u32) -> Result<(), String> {
    let sender = ctx.sender();

    let mut current_run = ctx
        .db
        .run()
        .id()
        .find(run_id)
        .ok_or("Run not found.")?;
    if current_run.owner != sender {
        return Err("This is not your run.".into());
    }
    if current_run.status != "active" {
        return Err("Run is not active.".into());
    }
    if room_index > current_run.current_room_index + 1 {
        return Err("Cannot skip rooms.".into());
    }

    // Check if this room already exists.
    let existing = ctx
        .db
        .room()
        .iter()
        .find(|r| r.run_id == run_id && r.room_index == room_index);
    if let Some(existing_room) = existing {
        if existing_room.status == "active" {
            return Err("Room is already active.".into());
        }
        if existing_room.status == "completed" {
            return Err("Room already completed.".into());
        }
    }

    // Determine room type based on seed and index.
    let room_type = match room_index % 4 {
        0 | 1 => "combat",
        2 => "treasure",
        3 => "shop",
        _ => "combat",
    };
    let enemies_total = if room_type == "combat" { 5 } else { 0 };

    ctx.db.room().insert(Room {
        id: 0,
        run_id,
        room_index,
        room_type: room_type.into(),
        status: "active".into(),
        enemies_remaining: enemies_total,
        enemies_total,
        reward_type: "tax_form".into(),
        created_at: ctx.timestamp,
    });

    // Update run's current room index.
    current_run.current_room_index = room_index;
    ctx.db.run().id().update(current_run);

    log::info!(
        "Player {:?} entered room {} (type: {}) in run {}",
        sender,
        room_index,
        room_type,
        run_id
    );
    Ok(())
}

/// Mark the current room as completed.
#[reducer]
pub fn complete_room(ctx: &ReducerContext, run_id: u64, room_id: u64) -> Result<(), String> {
    let sender = ctx.sender();

    let current_run = ctx
        .db
        .run()
        .id()
        .find(run_id)
        .ok_or("Run not found.")?;
    if current_run.owner != sender {
        return Err("This is not your run.".into());
    }
    if current_run.status != "active" {
        return Err("Run is not active.".into());
    }

    let mut current_room = ctx
        .db
        .room()
        .id()
        .find(room_id)
        .ok_or("Room not found.")?;
    if current_room.run_id != run_id {
        return Err("Room does not belong to this run.".into());
    }
    if current_room.status != "active" {
        return Err("Room is not active.".into());
    }
    if current_room.enemies_remaining > 0 {
        return Err("Enemies still remaining in this room.".into());
    }

    current_room.status = "completed".into();
    ctx.db.room().id().update(current_room);

    log::info!("Room {} completed in run {}", room_id, run_id);
    Ok(())
}

/// Abandon the current run. Records it in history as abandoned.
#[reducer]
pub fn abandon_run(ctx: &ReducerContext, run_id: u64) -> Result<(), String> {
    let sender = ctx.sender();

    let mut current_run = ctx
        .db
        .run()
        .id()
        .find(run_id)
        .ok_or("Run not found.")?;
    if current_run.owner != sender {
        return Err("This is not your run.".into());
    }
    if current_run.status != "active" {
        return Err("Run is not active.".into());
    }

    current_run.status = "abandoned".into();
    current_run.ended_at = Some(ctx.timestamp);
    let run_id_val = current_run.id;
    let character_id_val = current_run.character_id;
    let total_rooms_val = current_run.total_rooms;
    let kills_val = current_run.kills;
    let damage_dealt_val = current_run.damage_dealt;
    let started_micros = current_run.started_at.to_micros_since_unix_epoch();
    ctx.db.run().id().update(current_run);

    // Count how many rooms were completed.
    let rooms_cleared = ctx
        .db
        .room()
        .iter()
        .filter(|r| r.run_id == run_id && r.status == "completed")
        .count() as u32;

    // Calculate duration.
    let now_micros = ctx.timestamp.to_micros_since_unix_epoch();
    let duration_secs = ((now_micros - started_micros).max(0) / 1_000_000) as u64;

    // Record in history.
    ctx.db.run_history().insert(RunHistory {
        id: 0,
        owner: sender,
        character_id: character_id_val,
        run_id: run_id_val,
        rooms_cleared,
        total_rooms: total_rooms_val,
        kills: kills_val,
        damage_dealt: damage_dealt_val,
        compliance_credits_earned: 0,
        outcome: "abandoned".into(),
        duration_secs,
        completed_at: ctx.timestamp,
    });

    // Clean up combat state.
    if ctx.db.player_combat_state().identity().find(sender).is_some() {
        ctx.db.player_combat_state().identity().delete(sender);
    }

    // Clean up active enemies for this run.
    let enemies: Vec<_> = ctx
        .db
        .active_enemy()
        .iter()
        .filter(|e| e.run_id == run_id)
        .collect();
    for enemy in enemies {
        ctx.db.active_enemy().id().delete(enemy.id);
    }

    // Clean up projectiles for this run.
    let projectiles: Vec<_> = ctx
        .db
        .projectile()
        .iter()
        .filter(|p| p.run_id == run_id)
        .collect();
    for proj in projectiles {
        ctx.db.projectile().id().delete(proj.id);
    }

    // Clean up area effects for this run.
    let effects: Vec<_> = ctx
        .db
        .area_effect()
        .iter()
        .filter(|a| a.run_id == run_id)
        .collect();
    for effect in effects {
        ctx.db.area_effect().id().delete(effect.id);
    }

    log::info!("Run {} abandoned by {:?}", run_id, sender);
    Ok(())
}
