use bevy::prelude::*;

use crate::player::{AttackCooldown, Player, PlayerHealth};
use crate::state::ScreenState;

pub struct CombatPlugin;

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Deref, DerefMut)]
pub struct EnemySpeed(pub f32);

#[derive(Component)]
pub struct EnemyHealth {
    pub current: i32,
}

#[derive(Component)]
struct RunCombatEntity;

#[derive(Resource, Default)]
pub struct RunOutcome {
    pub room_cleared: bool,
}

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RunOutcome>()
            .add_systems(OnEnter(ScreenState::Run), setup_run_encounter)
            .add_systems(
                Update,
                (
                    chase_player,
                    player_attack,
                    enemy_contact_damage,
                    update_run_outcome,
                )
                    .run_if(in_state(ScreenState::Run)),
            )
            .add_systems(OnExit(ScreenState::Run), cleanup_run_combat);
    }
}

fn setup_run_encounter(mut commands: Commands, mut outcome: ResMut<RunOutcome>) {
    outcome.room_cleared = false;

    for (x, y, speed) in [
        (-360.0, 180.0, 110.0),
        (380.0, 120.0, 90.0),
        (280.0, -200.0, 130.0),
        (-240.0, -160.0, 100.0),
    ] {
        commands.spawn((
            Enemy,
            EnemySpeed(speed),
            EnemyHealth { current: 3 },
            Sprite::from_color(Color::srgb(0.9, 0.38, 0.31), Vec2::splat(28.0)),
            Transform::from_xyz(x, y, 1.0),
            RunCombatEntity,
        ));
    }
}

fn chase_player(
    time: Res<Time>,
    player: Query<&Transform, (With<Player>, Without<Enemy>)>,
    mut enemies: Query<(&EnemySpeed, &mut Transform), With<Enemy>>,
) {
    let Ok(player_transform) = player.single() else {
        return;
    };

    for (speed, mut transform) in &mut enemies {
        let direction =
            (player_transform.translation.truncate() - transform.translation.truncate()).normalize_or_zero();
        transform.translation += (direction * **speed * time.delta_secs()).extend(0.0);
    }
}

fn player_attack(
    mut commands: Commands,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut players: Query<(&Transform, &mut AttackCooldown), With<Player>>,
    mut enemies: Query<(Entity, &Transform, &mut EnemyHealth), With<Enemy>>,
) {
    let Ok((player_transform, mut cooldown)) = players.single_mut() else {
        return;
    };

    cooldown.timer.tick(time.delta());

    if !keyboard.just_pressed(KeyCode::Space) || !cooldown.timer.is_finished() {
        return;
    }

    cooldown.timer.reset();

    let player_position = player_transform.translation.truncate();

    for (entity, enemy_transform, mut enemy_health) in &mut enemies {
        let distance = player_position.distance(enemy_transform.translation.truncate());
        if distance <= 110.0 {
            enemy_health.current -= 1;
            if enemy_health.current <= 0 {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn enemy_contact_damage(
    time: Res<Time>,
    mut player: Query<(&Transform, &mut PlayerHealth), With<Player>>,
    enemies: Query<&Transform, With<Enemy>>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    let Ok((player_transform, mut health)) = player.single_mut() else {
        return;
    };

    let player_position = player_transform.translation.truncate();
    let mut touchers = 0;

    for enemy_transform in &enemies {
        let distance = player_position.distance(enemy_transform.translation.truncate());
        if distance <= 30.0 {
            touchers += 1;
        }
    }

    if touchers > 0 {
        let chip_window = (time.elapsed_secs_wrapped() * 2.0).fract();
        if chip_window < 0.03 {
            health.current -= touchers;
        }
    }

    if health.current <= 0 {
        next_state.set(ScreenState::Hub);
    }
}

fn update_run_outcome(
    enemies: Query<Entity, With<Enemy>>,
    mut outcome: ResMut<RunOutcome>,
    mut next_state: ResMut<NextState<ScreenState>>,
) {
    if enemies.is_empty() && !outcome.room_cleared {
        outcome.room_cleared = true;
        next_state.set(ScreenState::Debate);
    }
}

fn cleanup_run_combat(mut commands: Commands, entities: Query<Entity, With<RunCombatEntity>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
