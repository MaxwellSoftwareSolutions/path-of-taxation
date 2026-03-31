use bevy::prelude::*;

use crate::state::ScreenState;

pub struct PlayerPlugin;

#[derive(Component)]
pub struct Player;

#[derive(Component, Deref, DerefMut)]
pub struct MoveSpeed(pub f32);

#[derive(Component)]
pub struct PlayerHealth {
    pub current: i32,
    pub max: i32,
}

#[derive(Component)]
pub struct AttackCooldown {
    pub timer: Timer,
}

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ScreenState::Run), spawn_player)
            .add_systems(Update, move_player.run_if(in_state(ScreenState::Run)))
            .add_systems(OnExit(ScreenState::Run), cleanup_player);
    }
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        MoveSpeed(360.0),
        PlayerHealth {
            current: 6,
            max: 6,
        },
        AttackCooldown {
            timer: Timer::from_seconds(0.35, TimerMode::Repeating),
        },
        Sprite::from_color(Color::srgb(0.42, 0.78, 1.0), Vec2::splat(36.0)),
        Transform::default(),
        Name::new("Refund Witch"),
    ));
}

fn move_player(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&MoveSpeed, &mut Transform), With<Player>>,
) {
    let mut direction = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }

    let delta = direction.normalize_or_zero() * time.delta_secs();

    for (speed, mut transform) in &mut players {
        transform.translation += (delta * **speed).extend(0.0);
        transform.translation.x = transform.translation.x.clamp(-620.0, 620.0);
        transform.translation.y = transform.translation.y.clamp(-300.0, 300.0);
    }
}

fn cleanup_player(mut commands: Commands, players: Query<Entity, With<Player>>) {
    for entity in &players {
        commands.entity(entity).despawn();
    }
}
