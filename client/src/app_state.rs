use bevy::prelude::*;

/// Top-level application state machine.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Initial loading and setup.
    #[default]
    Boot,
    /// Main menu.
    Menu,
    /// Hub area (Clearfile Tax Office) -- loadout, NPCs, stash.
    Hub,
    /// Active combat run (rooms, enemies, loot).
    Run,
    /// Debate Club turn-based card minigame.
    Debate,
    /// Post-run results summary.
    Results,
}

/// Sub-state for combat phases within a Run.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::Run)]
pub enum CombatPhase {
    /// Player chooses which room to enter next.
    RoomSelect,
    /// Active combat in a room.
    #[default]
    Combat,
    /// Room cleared -- pick up loot, prepare for next.
    RoomClear,
    /// Boss intro cutscene / dialog.
    BossIntro,
    /// Boss fight.
    BossFight,
}
