//! Level stuff.

pub mod collision;
pub mod pipe;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::{map::TilemapSize, tiles::TilePos};
use bevy_rapier2d::prelude::*;

use std::collections::HashMap;

use collision::{CollisionMap, CreatedCollider, LevelCollisionPlugin, LevelCollisionSystem};

use crate::enemy::{Enemy, Hostility};
use crate::physics;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LevelCollisionPlugin::<Ground>::default())
            .add_plugins(LevelCollisionPlugin::<Spikes>::default())
            .add_systems(
                Update,
                update_collision_map::<Ground>.before(LevelCollisionSystem::BuildCollision),
            )
            .add_systems(
                Update,
                update_collision_map::<Spikes>.before(LevelCollisionSystem::BuildCollision),
            )
            .add_systems(Update, make_spikes_deadly);
    }

    fn finish(&self, app: &mut App) {
        app.register_default_ldtk_int_cell_for_layer::<CollisionBundle>("CollisionOverride")
            .register_default_ldtk_int_cell_for_layer::<CollisionBundle>("Ground");
    }
}

/// A component that identifies an entity by its instance identifier.
#[derive(Clone, Component, Debug, Default)]
pub struct Iid(pub String);

impl From<&EntityInstance> for Iid {
    fn from(e: &EntityInstance) -> Iid {
        Iid(e.iid.clone())
    }
}

/// A bundle that indicates a region of collision..
#[derive(Bundle, LdtkIntCell)]
pub struct CollisionBundle {
    #[with(initial_collision)]
    collider: Collision,
}

fn initial_collision(i: IntGridCell) -> Collision {
    match i.value {
        1 => Collision::Solid,
        2 => Collision::Solid,
        4 => Collision::Spikes,
        _ => Collision::Vacant,
    }
}

/// An enum that denotes the solidity of grid regions.
#[derive(Copy, Clone, Component, Default, Debug)]
pub enum Collision {
    Solid,
    Spikes,
    #[default]
    Vacant,
}

/// A marker type for the simple ground collision.
pub struct Ground;

/// A marker type for the spikes collision.
pub struct Spikes;

trait CheckCollision {
    fn solid(s: &Collision) -> bool;
}

impl CheckCollision for Ground {
    fn solid(s: &Collision) -> bool {
        matches!(s, Collision::Solid)
    }
}

impl CheckCollision for Spikes {
    fn solid(s: &Collision) -> bool {
        matches!(s, Collision::Spikes)
    }
}

fn update_collision_map<T>(
    mut commands: Commands,
    collision_query: Query<(&Collision, &TilePos, &Parent), Changed<Collision>>,
    mut layer_query: Query<(&TilemapSize, Option<&mut CollisionMap<T>>)>,
) where
    T: CheckCollision + Send + Sync + 'static,
{
    let mut new_collision_maps: HashMap<Entity, CollisionMap<T>> = HashMap::new();

    // do updates
    for (collision, pos, parent) in collision_query.iter() {
        // get collision map
        let Ok((map_size, mut collision_map)) = layer_query.get_mut(parent.get()) else {
            continue;
        };

        let collision_map = if let Some(c) = collision_map.as_mut() {
            &mut *c
        } else {
            // add to cache
            new_collision_maps
                .entry(parent.get())
                .or_insert_with(|| CollisionMap::new(&map_size))
        };

        collision_map.put(map_size, *pos, T::solid(collision));
    }

    // add new collision maps
    for (entity, collision_map) in new_collision_maps {
        commands.entity(entity).insert(collision_map);
    }
}

fn make_spikes_deadly(
    mut commands: Commands,
    added_spikes_query: Query<Entity, Added<CreatedCollider<Spikes>>>,
) {
    for entity in added_spikes_query.iter() {
        commands
            .entity(entity)
            .insert(CollisionGroups::new(
                physics::COLLISION_GROUP_SOLID | physics::COLLISION_GROUP_HOSTILE,
                Group::all(),
            ))
            .insert(Hostility::Hostile)
            .insert(Enemy::invincible());
    }
}
