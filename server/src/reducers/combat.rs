use spacetimedb::{reducer, ReducerContext, Table, TimeDuration};

use crate::tables::player::character;
use crate::tables::run::run;
use crate::tables::combat::{player_combat_state, projectile, Projectile};
use crate::tables::skills::{cooldown_state, skill_loadout, CooldownState};
use crate::tables::events::{vfx_event, VfxEvent};

/// Move the player to a new position. Server validates the movement.
#[reducer]
pub fn move_player(ctx: &ReducerContext, pos_x: f32, pos_y: f32) -> Result<(), String> {
    let sender = ctx.sender();

    let mut state = ctx
        .db
        .player_combat_state()
        .identity()
        .find(sender)
        .ok_or("No active combat state. Are you in a run?")?;

    // Validate the run is still active.
    let current_run = ctx
        .db
        .run()
        .id()
        .find(state.run_id)
        .ok_or("Associated run not found.")?;
    if current_run.status != "active" {
        return Err("Run is not active.".into());
    }

    // Basic movement validation: check distance is within one tick's worth of movement.
    let dx = pos_x - state.pos_x;
    let dy = pos_y - state.pos_y;
    let distance = (dx * dx + dy * dy).sqrt();
    // Allow up to move_speed * tick_interval (50ms = 0.05s) with some tolerance.
    let max_distance = state.move_speed * 0.05 * 2.0;
    if distance > max_distance {
        // Clamp to max distance rather than rejecting outright (client prediction tolerance).
        let ratio = max_distance / distance;
        state.pos_x += dx * ratio;
        state.pos_y += dy * ratio;
    } else {
        state.pos_x = pos_x;
        state.pos_y = pos_y;
    }

    // Update facing angle based on movement direction.
    if distance > 0.01 {
        state.facing_angle = dy.atan2(dx);
    }

    ctx.db.player_combat_state().identity().update(state);
    Ok(())
}

/// Use an ability. Validates cooldowns, mana cost, and creates appropriate
/// projectiles/effects. Damage calculation is stubbed.
#[reducer]
pub fn use_ability(
    ctx: &ReducerContext,
    ability_key: String,
    target_x: f32,
    target_y: f32,
) -> Result<(), String> {
    let sender = ctx.sender();

    let mut state = ctx
        .db
        .player_combat_state()
        .identity()
        .find(sender)
        .ok_or("No active combat state. Are you in a run?")?;

    // Validate the run is still active.
    let current_run = ctx
        .db
        .run()
        .id()
        .find(state.run_id)
        .ok_or("Associated run not found.")?;
    if current_run.status != "active" {
        return Err("Run is not active.".into());
    }

    // Check the player has this ability equipped.
    let character_row = ctx
        .db
        .character()
        .id()
        .find(current_run.character_id)
        .ok_or("Character not found.")?;
    let has_ability = ctx
        .db
        .skill_loadout()
        .iter()
        .any(|s| s.character_id == character_row.id && s.ability_key == ability_key);
    if !has_ability {
        return Err(format!("Ability '{}' is not equipped.", ability_key));
    }

    // Check cooldown: ensure no active cooldown for this ability.
    let on_cooldown = ctx
        .db
        .cooldown_state()
        .iter()
        .any(|c| {
            c.identity == sender
                && c.run_id == state.run_id
                && c.ability_key == ability_key
                && c.cooldown_expires_at > ctx.timestamp
        });
    if on_cooldown {
        return Err(format!("Ability '{}' is on cooldown.", ability_key));
    }

    // Stub: apply mana cost (use a default of 10 mana for now).
    let mana_cost: i64 = 10;
    if state.current_mana < mana_cost {
        return Err("Not enough mana.".into());
    }
    state.current_mana -= mana_cost;

    let run_id = state.run_id;
    let origin_x = state.pos_x;
    let origin_y = state.pos_y;
    ctx.db.player_combat_state().identity().update(state);

    // Set cooldown (default 500ms).
    let cooldown_duration = TimeDuration::from_micros(500_000);
    ctx.db.cooldown_state().insert(CooldownState {
        id: 0,
        identity: sender,
        run_id,
        ability_key: ability_key.clone(),
        cooldown_expires_at: ctx.timestamp + cooldown_duration,
    });

    // Create a VFX event for the ability cast.
    ctx.db.vfx_event().insert(VfxEvent {
        id: 0,
        run_id,
        vfx_key: format!("cast_{}", ability_key),
        pos_x: origin_x,
        pos_y: origin_y,
        intensity: 1.0,
        created_at: ctx.timestamp,
    });

    // TODO: Look up ability definition from content system and create
    // appropriate projectiles/area effects/etc based on ability type.
    // For now, create a simple projectile for any ability.
    let dx = target_x - origin_x;
    let dy = target_y - origin_y;
    let dist = (dx * dx + dy * dy).sqrt().max(0.01);
    let speed = 500.0_f32;
    let vel_x = (dx / dist) * speed;
    let vel_y = (dy / dist) * speed;

    ctx.db.projectile().insert(Projectile {
        id: 0,
        run_id,
        owner_identity: Some(sender),
        owner_enemy_id: None,
        ability_key,
        pos_x: origin_x,
        pos_y: origin_y,
        vel_x,
        vel_y,
        damage: 25, // TODO: calculate from ability def + stats
        pierce_remaining: 0,
        created_at: ctx.timestamp,
        lifetime_ms: 2000,
    });

    Ok(())
}
