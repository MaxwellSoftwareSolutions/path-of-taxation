use spacetimedb::{reducer, ReducerContext, Table, TimeDuration, ScheduleAt};

use crate::tables::player::{player, Player};
use crate::tables::events::{
    damage_event, server_tick_schedule, vfx_event, ServerTickSchedule,
};

/// Initialize the module: seed any default data and start the server tick loop.
#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    log::info!("Path of Taxation server module initialized.");

    // Start the 20Hz server tick loop (50ms interval).
    ctx.db.server_tick_schedule().insert(ServerTickSchedule {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::from(TimeDuration::from_micros(50_000)),
    });
}

/// Called when a client connects. Creates a player record if one does not exist.
#[reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    let sender = ctx.sender();
    log::info!("Client connected: {:?}", sender);

    if let Some(mut existing) = ctx.db.player().identity().find(sender) {
        // Returning player: update online status and last login.
        existing.is_online = true;
        existing.last_login = ctx.timestamp;
        ctx.db.player().identity().update(existing);
    } else {
        // New player: create their record.
        let abbrev = ctx.sender().to_abbreviated_hex();
        ctx.db.player().insert(Player {
            identity: sender,
            username: format!("Taxpayer_{}", abbrev),
            created_at: ctx.timestamp,
            last_login: ctx.timestamp,
            compliance_credits: 0,
            total_runs: 0,
            total_kills: 0,
            is_online: true,
        });
    }
}

/// Called when a client disconnects. Marks them offline.
#[reducer(client_disconnected)]
pub fn client_disconnected(ctx: &ReducerContext) {
    let sender = ctx.sender();
    log::info!("Client disconnected: {:?}", sender);

    if let Some(mut existing) = ctx.db.player().identity().find(sender) {
        existing.is_online = false;
        ctx.db.player().identity().update(existing);
    }
}

/// The 20Hz server tick. Processes enemy AI, projectiles, area effects, and prunes
/// transient event tables. Complex logic is left as todo!() stubs.
#[reducer]
pub fn process_server_tick(ctx: &ReducerContext, _tick: ServerTickSchedule) {
    // Prune old damage events (older than 2 ticks = 100ms).
    let cutoff = ctx.timestamp - TimeDuration::from_micros(100_000);
    let old_events: Vec<_> = ctx
        .db
        .damage_event()
        .iter()
        .filter(|e| e.created_at < cutoff)
        .collect();
    for event in old_events {
        ctx.db.damage_event().id().delete(event.id);
    }

    // Prune old VFX events.
    let old_vfx: Vec<_> = ctx
        .db
        .vfx_event()
        .iter()
        .filter(|e| e.created_at < cutoff)
        .collect();
    for event in old_vfx {
        ctx.db.vfx_event().id().delete(event.id);
    }

    // TODO: Process enemy AI decisions (chase, attack, etc.)
    // TODO: Move projectiles and check collisions
    // TODO: Tick area effects and apply damage
    // TODO: Process buff/debuff ticking
}
