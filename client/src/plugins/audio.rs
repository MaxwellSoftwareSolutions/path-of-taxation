use bevy::prelude::*;

use crate::app_state::AppState;
use crate::plugins::combat::HitMsg;
use crate::plugins::enemies::EnemyDeathMsg;
use crate::components::player::Player;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SfxEvent>()
            .add_systems(Update, (
                combat_sfx_bridge_system,
                sfx_log_system,
            ).chain().run_if(in_state(AppState::Run)));
    }
}

/// Sound effect event variants. For now these are just logged; when actual
/// audio assets exist the playback system can match on the variant to pick
/// the right sound file.
#[derive(Message, Clone, Debug)]
pub enum SfxEvent {
    Hit,
    CritHit,
    PlayerHit,
    EnemyDeath,
    AbilityCast,
    Pickup,
    RoomClear,
    BossPhase,
}

/// Bridge system: reads combat messages and emits corresponding SfxEvents.
fn combat_sfx_bridge_system(
    mut hit_msgs: MessageReader<HitMsg>,
    mut death_msgs: MessageReader<EnemyDeathMsg>,
    player_query: Query<Entity, With<Player>>,
    mut sfx_msgs: MessageWriter<SfxEvent>,
) {
    let player_entity = player_query.single().ok();

    for hit in hit_msgs.read() {
        if hit.is_critical {
            sfx_msgs.write(SfxEvent::CritHit);
        } else if Some(hit.target) == player_entity {
            sfx_msgs.write(SfxEvent::PlayerHit);
        } else {
            sfx_msgs.write(SfxEvent::Hit);
        }
    }

    for _death in death_msgs.read() {
        sfx_msgs.write(SfxEvent::EnemyDeath);
    }
}

/// Placeholder playback system: logs SFX events until real audio assets are available.
fn sfx_log_system(
    mut sfx_msgs: MessageReader<SfxEvent>,
) {
    for event in sfx_msgs.read() {
        info!("SFX: {:?}", event);
    }
}
