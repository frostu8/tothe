//! Player things.

pub mod controller;

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use bevy_ecs_ldtk::{LdtkEntity, app::LdtkEntityAppExt};

use std::time::Duration;

use controller::{ControllerBundle, ControllerOptions};

/// A player plugin.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (tick_respawn_timer, respawn_player).chain())
            .add_systems(Startup, spawn_player);
    }

    fn finish(&self, app: &mut App) {
        app
            .register_ldtk_entity::<CheckpointBundle>("Checkpoint");
    }
}

/// A timer for player respawns.
#[derive(Clone, Component, Debug)]
pub struct RespawnTimer(Timer);

impl RespawnTimer {
    /// Creates a new respawn timer.
    pub fn new(duration: Duration) -> RespawnTimer {
        RespawnTimer(Timer::new(duration, TimerMode::Once))
    }

    /// Finishes the timer immediately.
    pub fn finish_now(&mut self) {
        self.0.set_elapsed(self.0.duration())
    }
}

impl Default for RespawnTimer {
    fn default() -> RespawnTimer {
        RespawnTimer::new(Duration::from_millis(400))
    }
}

#[derive(Clone, Component, Default, Debug)]
pub struct Alive(bool);

/// A marker component for a checkpoint, where a player will respawn when they
/// die.
#[derive(Clone, Component, Default, Debug)]
pub struct Checkpoint;

/// A checkpoint bundle.
#[derive(Bundle, Default, LdtkEntity)]
pub struct CheckpointBundle {
    pub checkpoint: Checkpoint,
}

fn tick_respawn_timer(
    mut timer_query: Query<&mut RespawnTimer>,
    time: Res<Time>,
) {
    for mut timer in timer_query.iter_mut() {
        timer.0.tick(time.delta());
    }
}

fn respawn_player(
    mut player_query: Query<(&mut Transform, &mut Visibility, &RespawnTimer, &mut Alive)>,
    checkpoint_query: Query<&GlobalTransform, With<Checkpoint>>,
) {
    let Ok(respawn_pos) = checkpoint_query.get_single() else {
        return;
    };

    for (mut transform, mut visibility, respawn_timer, mut alive) in player_query.iter_mut() {
        if !alive.0 && respawn_timer.0.finished() {
            // respawn player
            *visibility = Visibility::Visible;
            *transform = Transform::from_translation(respawn_pos.translation());
            alive.0 = true;
        }
    }
}

/// A startup system that spawns a default player in.
fn spawn_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load("player/player.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 2, 1, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    commands
        .spawn((
            SpatialBundle {
                visibility: Visibility::Hidden,
                ..Default::default()
            },
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED,
            Alive::default(),
            Collider::round_cuboid(2., 2., 0.25),
            //Collider::cuboid(4., 4.),
            Velocity::default(),
            Friction {
                coefficient: 0.,
                combine_rule: CoefficientCombineRule::Multiply,
            },
            ControllerBundle {
                options: ControllerOptions {
                    max_speed: 48.,
                    friction: 4.,
                },
                ..Default::default()
            },
            RespawnTimer::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                SpriteSheetBundle {
                    texture_atlas: texture_atlas_handle,
                    sprite: TextureAtlasSprite::new(0),
                    transform: Transform::from_xyz(0., 4., 0.),
                    ..Default::default()
                },
            ));
        });
}

