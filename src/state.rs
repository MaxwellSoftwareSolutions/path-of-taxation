use bevy::prelude::*;

#[derive(States, Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ScreenState {
    #[default]
    Boot,
    Hub,
    Run,
    Debate,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ScreenState>()
            .add_systems(Startup, boot_into_hub);
    }
}

fn boot_into_hub(mut next_state: ResMut<NextState<ScreenState>>) {
    next_state.set(ScreenState::Hub);
}

