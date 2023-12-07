//! Respawn things for the player.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use bevy_ecs_ldtk::{app::LdtkEntityAppExt, LdtkEntity, LdtkLevel, LevelSelection};

use std::collections::HashMap;
use std::time::Duration;

use super::{LocalPlayer, controller::ControllerOptions};

use crate::{GameState, GameAssets, spawn_world};

pub struct RespawnPlugin;

impl Plugin for RespawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointMap>()
            .init_resource::<WorldRespawn>()
            .add_systems(
                Update,
                world_respawn
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                respawn
                    .run_if(in_state(GameState::InGame))
                    .in_set(RespawnSystem::Respawn),
            )
            .add_systems(Update, update_checkpoints);
    }

    fn finish(&self, app: &mut App) {
        app.register_ldtk_entity::<CheckpointBundle>("Checkpoint");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum RespawnSystem {
    Respawn,
}

/// A timer for player respawns.
#[derive(Clone, Component, Debug)]
pub struct Respawn {
    timer: Timer,
    respawned: bool,
}

impl Respawn {
    /// Creates a new respawn timer.
    pub fn new(duration: Duration) -> Respawn {
        Respawn {
            timer: Timer::new(duration, TimerMode::Once),
            respawned: false,
        }
    }

    /// Resets the respawn timer.
    pub fn start_respawn(&mut self) {
        self.timer.reset();
        self.respawned = false;
    }
}

impl Default for Respawn {
    fn default() -> Respawn {
        Respawn::new(Duration::from_millis(200))
    }
}

/// A timer to respawn the whole world.
#[derive(Resource)]
pub struct WorldRespawn {
    /// How long it takes until the world is respawned starting from when this
    /// resource is first notified.
    pub duration: Duration,
    timer: Timer,
    post_timer: Timer,
    finished: bool,
}

impl WorldRespawn {
    /// Creates a new `WorldRespawn` with a respawn duration.
    pub fn new(duration: Duration) -> WorldRespawn {
        WorldRespawn {
            duration: duration.clone(),
            timer: Timer::new(duration, TimerMode::Once),
            post_timer: Timer::new(duration, TimerMode::Once),
            finished: true,
        }
    }

    /// Sets the respawn timer.
    pub fn start_respawn(&mut self) {
        self.timer.reset();
        self.post_timer.reset();
        self.finished = false;
    }
}

impl Default for WorldRespawn {
    fn default() -> WorldRespawn {
        WorldRespawn::new(Duration::from_millis(200))
    }
}

/// A marker component for a checkpoint, where a player will respawn when they
/// die.
#[derive(Clone, Component, Default, Debug)]
pub struct Checkpoint;

/// A resource that keeps track of all checkpoints.
#[derive(Clone, Default, Debug, Resource)]
pub struct CheckpointMap {
    map: HashMap<String, Entity>,
}

/// A query for the current checkpoint.
#[derive(SystemParam)]
pub struct CurrentCheckpoint<'w, 's> {
    checkpoints: Res<'w, CheckpointMap>,
    level_selection: Res<'w, LevelSelection>,
    checkpoint_query: Query<'w, 's, &'static GlobalTransform, With<Checkpoint>>,
}

impl<'w, 's> CurrentCheckpoint<'w, 's> {
    /// Gets the current checkpoint's transform.
    pub fn position(&self) -> Option<&GlobalTransform> {
        let level = match &*self.level_selection {
            LevelSelection::Identifier(level) => level,
            _ => todo!("no support for other level selections"),
        };

        self.checkpoints
            .map
            .get(level)
            .and_then(|c| self.checkpoint_query.get(*c).ok())
    }
}

/// A checkpoint bundle.
#[derive(Bundle, Default, LdtkEntity)]
pub struct CheckpointBundle {
    pub checkpoint: Checkpoint,
}

fn update_checkpoints(
    mut checkpoint_map: ResMut<CheckpointMap>,
    added_checkpoints_query: Query<(Entity, &Parent), Added<Checkpoint>>,
    levels_query: Query<&Handle<LdtkLevel>>,
    levels: Res<Assets<LdtkLevel>>,
) {
    for (entity, parent) in added_checkpoints_query.iter() {
        let Ok(level) = levels_query.get(parent.get()) else {
            continue;
        };

        let Some(level) = levels.get(level) else {
            continue;
        };

        checkpoint_map
            .map
            .insert(level.level.identifier.clone(), entity);
    }
}

fn world_respawn(
    mut commands: Commands,
    mut world_respawn: ResMut<WorldRespawn>,
    game_world_query: Query<Entity, With<crate::GameWorld>>,
    mut curtain_query: Query<&mut crate::ui::Curtain>,
    mut respawn_timer_query: Query<&mut Respawn, With<LocalPlayer>>,
    assets: Res<GameAssets>,
    time: Res<Time>,
) {
    if world_respawn.finished {
        if !world_respawn.post_timer.finished() {
            world_respawn.post_timer.tick(time.delta());

            if let Ok(mut curtain) = curtain_query.get_single_mut() {
                curtain.stage = -world_respawn.post_timer.percent();
            }
        }

        return;
    }

    if world_respawn.timer.finished() {
        // try to respawn world
        for entity in game_world_query.iter() {
            commands.entity(entity).despawn_recursive();
        }

        spawn_world(commands, assets);

        world_respawn.finished = true;
    } else {
        // TODO: weird player spawn hack
        if world_respawn.timer.percent() < f32::EPSILON {
            for mut respawn in respawn_timer_query.iter_mut() {
                respawn.start_respawn();
            }
        }

        world_respawn.timer.tick(time.delta());

        if let Ok(mut curtain) = curtain_query.get_single_mut() {
            curtain.stage = world_respawn.timer.percent_left();
        }
    }
}

fn respawn(
    mut player_query: Query<(&mut Transform, &mut Visibility, &mut ControllerOptions, &mut Respawn)>,
    current_checkpoint: CurrentCheckpoint,
    time: Res<Time>,
) {
    let respawn_pos = current_checkpoint.position();

    for (mut transform, mut visibility, mut controller, mut respawn) in player_query.iter_mut() {
        respawn.timer.tick(time.delta());

        if let Some(respawn_pos) = &respawn_pos {
            if respawn.timer.finished() && !respawn.respawned {
                // respawn player
                *visibility = Visibility::Visible;
                controller.enabled = true;
                *transform =
                    Transform::from_translation(respawn_pos.translation());

                respawn.respawned = true;
            }
        }
    }
}
