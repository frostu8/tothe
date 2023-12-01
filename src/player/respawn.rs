//! Respawn things.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use bevy_ecs_ldtk::{app::LdtkEntityAppExt, LdtkEntity, LdtkLevel, LevelSelection};

use std::collections::HashMap;
use std::time::Duration;

pub struct RespawnPlugin;

impl Plugin for RespawnPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointMap>()
            .add_systems(Update, (tick_respawn_timer, respawn))
            .add_systems(Update, update_checkpoints);
    }

    fn finish(&self, app: &mut App) {
        app.register_ldtk_entity::<CheckpointBundle>("Checkpoint");
    }
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
}

impl Default for Respawn {
    fn default() -> Respawn {
        Respawn::new(Duration::from_millis(400))
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

fn tick_respawn_timer(mut timer_query: Query<&mut Respawn>, time: Res<Time>) {
    for mut respawn in timer_query.iter_mut() {
        respawn.timer.tick(time.delta());
    }
}

fn respawn(
    mut player_query: Query<(&mut Transform, &mut Visibility, &mut Respawn)>,
    current_checkpoint: CurrentCheckpoint,
) {
    let Some(respawn_pos) = current_checkpoint.position() else {
        return;
    };

    for (mut transform, mut visibility, mut respawn) in player_query.iter_mut() {
        if respawn.timer.just_finished() {
            // respawn player
            *visibility = Visibility::Visible;
            *transform =
                Transform::from_translation(respawn_pos.translation() + Vec3::new(8., 8., 0.));

            respawn.respawned = true;
        }
    }
}
