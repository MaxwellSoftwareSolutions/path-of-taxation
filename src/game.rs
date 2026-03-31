use bevy::prelude::*;
use crate::combat::CombatPlugin;
use crate::player::PlayerPlugin;
use crate::state::GameStatePlugin;
use crate::ui::UiPlugin;
use crate::world::WorldPlugin;

pub struct PathOfTaxationPlugin;

impl Plugin for PathOfTaxationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            GameStatePlugin,
            WorldPlugin,
            PlayerPlugin,
            CombatPlugin,
            UiPlugin,
        ));
    }
}
