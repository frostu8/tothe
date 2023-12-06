//! Merges level collision using a simple map.

use bevy::prelude::*;
use bevy_ecs_tilemap::{
    map::{TilemapSize, TilemapTileSize},
    tiles::TilePos,
};
use bevy_rapier2d::prelude::*;

use std::collections::HashMap;
use std::marker::PhantomData;

use crate::physics;

/// A plugin for a single map of collision.
pub struct LevelCollisionPlugin<T>
where
    T: Send + Sync + 'static,
{
    _marker: PhantomData<T>,
}

impl<T> Default for LevelCollisionPlugin<T>
where
    T: Send + Sync + 'static,
{
    fn default() -> LevelCollisionPlugin<T> {
        LevelCollisionPlugin {
            _marker: PhantomData,
        }
    }
}

impl<T> Plugin for LevelCollisionPlugin<T>
where
    T: Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            build_collision::<T>.in_set(LevelCollisionSystem::BuildCollision),
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum LevelCollisionSystem {
    /// Systems that actually build the collision
    BuildCollision,
}

/// A bitmap for collision.
///
/// Can be specialized using the marker type and adding
/// [`LevelCollisionPlugin`] for the marker.
#[derive(Clone, Component, Default, Debug)]
pub struct CollisionMap<T = ()>
where
    T: Send + Sync + 'static,
{
    map: Vec<bool>,
    _marker: PhantomData<T>,
}

impl<T> CollisionMap<T>
where
    T: Send + Sync + 'static,
{
    /// Creates a new collision map.
    pub fn new(map_size: &TilemapSize) -> CollisionMap<T> {
        CollisionMap::<T> {
            map: (0..map_size.count()).map(|_| false).collect(),
            _marker: PhantomData,
        }
    }

    /// Gets a bool from the map.
    pub fn get(&self, map_size: &TilemapSize, pos: impl Into<TilePos>) -> bool {
        let pos = pos.into();

        if pos.within_map_bounds(map_size) {
            self.map[pos.to_index(map_size)]
        } else {
            false
        }
    }

    /// Puts a bool in the map.
    pub fn put(&mut self, map_size: &TilemapSize, pos: impl Into<TilePos>, flag: bool) {
        self.map[pos.into().to_index(map_size)] = flag;
    }
}

/// A marker component for colliders created by
/// [`LevelCollisionSystem::BuildCollision`].
#[derive(Clone, Component, Debug)]
pub struct CreatedCollider<T>
where
    T: Send + Sync + 'static,
{
    _marker: PhantomData<T>,
}

impl<T> Default for CreatedCollider<T>
where
    T: Send + Sync + 'static,
{
    fn default() -> CreatedCollider<T> {
        CreatedCollider::<T> {
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Hash)]
struct Plate {
    left: u32,
    right: u32,
}

struct Rect {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

fn build_collision<T>(
    mut commands: Commands,
    layer_query: Query<
        (&Parent, &TilemapSize, &TilemapTileSize, &CollisionMap<T>),
        Changed<CollisionMap<T>>,
    >,
    created_colliders: Query<(Entity, &Parent), With<CreatedCollider<T>>>,
) where
    T: Send + Sync + 'static,
{
    layer_query.for_each(|(parent, map_size, tile_size, collision_map)| {
        // clear created colliders
        for (collider_entity, collider_parent) in created_colliders.iter() {
            if collider_parent.get() == parent.get() {
                commands.entity(collider_entity).despawn_recursive()
            }
        }

        let colliders = create_colliders_for(
            parent.get(),
            &mut commands,
            map_size,
            tile_size,
            collision_map,
        );

        for entity in colliders {
            commands
                .entity(entity)
                .insert(CreatedCollider::<T>::default());
        }
    })
}

fn create_colliders_for<T>(
    parent_entity: Entity,
    commands: &mut Commands,
    map_size: &TilemapSize,
    tile_size: &TilemapTileSize,
    map: &CollisionMap<T>,
) -> Vec<Entity>
where
    T: Send + Sync + 'static,
{
    let mut plates: Vec<Vec<Plate>> = Vec::new();

    // sort by y
    for y in 0..map_size.y {
        let mut current_layer = Vec::new();
        let mut plate_start: Option<u32> = None;

        // extra empty column so the algorithm "finishes" plates that touch the
        // right edge.
        for x in 0..map_size.x + 1 {
            let solid = map.get(map_size, UVec2::new(x, y));

            match (plate_start, solid) {
                (Some(s), false) => {
                    // build plate
                    current_layer.push(Plate {
                        left: s,
                        right: x - 1,
                    });
                    plate_start = None;
                }
                (None, true) => {
                    plate_start = Some(x);
                }
                _ => (),
            }
        }

        plates.push(current_layer);
    }

    build_rects(plates)
        .into_iter()
        .map(|rect| {
            commands
                .spawn((
                    Collider::cuboid(
                        (rect.right as f32 - rect.left as f32 + 1.) * tile_size.x / 2.,
                        (rect.top as f32 - rect.bottom as f32 + 1.) * tile_size.y / 2.,
                    ),
                    RigidBody::Fixed,
                    Friction::new(1.0),
                    Transform::from_xyz(
                        (rect.left + rect.right + 1) as f32 * tile_size.x / 2.,
                        (rect.bottom + rect.top + 1) as f32 * tile_size.y / 2.,
                        0.,
                    ),
                    GlobalTransform::default(),
                    CollisionGroups::new(physics::COLLISION_GROUP_SOLID, Group::all()),
                ))
                .set_parent(parent_entity)
                .id()
        })
        .collect()
}

fn build_rects(mut plates: Vec<Vec<Plate>>) -> Vec<Rect> {
    let mut rect_builder: HashMap<Plate, Rect> = HashMap::new();
    let mut prev_row = Vec::new();
    let mut finished_rects = Vec::new();

    // an extra empty row so the algorithm "finishes" the rects that touch the top edge
    plates.push(Vec::new());

    for (y, current_row) in plates.into_iter().enumerate() {
        for prev_plate in &prev_row {
            if !current_row.contains(prev_plate) {
                // remove the finished rect so that the same plate in the future starts a new rect
                if let Some(rect) = rect_builder.remove(prev_plate) {
                    finished_rects.push(rect);
                }
            }
        }
        for plate in &current_row {
            rect_builder
                .entry(plate.clone())
                .and_modify(|e| e.top += 1)
                .or_insert(Rect {
                    bottom: y as u32,
                    top: y as u32,
                    left: plate.left,
                    right: plate.right,
                });
        }
        prev_row = current_row;
    }

    finished_rects
}
