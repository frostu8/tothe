//! Environment stuff.

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::{
    map::{TilemapSize, TilemapTileSize},
    tiles::TilePos,
};
use bevy_rapier2d::prelude::*;

use std::collections::HashMap;

use crate::physics;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_collision_map, create_colliders).chain());
    }

    fn finish(&self, app: &mut App) {
        app.register_default_ldtk_int_cell_for_layer::<CollisionBundle>("CollisionOverride")
            .register_default_ldtk_int_cell_for_layer::<CollisionBundle>("Ground");
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
        _ => Collision::Vacant,
    }
}

/// An enum that denotes the solidity of grid regions.
#[derive(Copy, Clone, Component, Default, Debug)]
pub enum Collision {
    Solid,
    #[default]
    Vacant,
}

impl Collision {
    pub fn solid(self) -> bool {
        matches!(self, Collision::Solid)
    }
}

/// A bitmap for collision.
#[derive(Clone, Component, Default, Debug)]
pub struct CollisionMap {
    map: Vec<bool>,
}

impl CollisionMap {
    /// Creates a new collision map.
    pub fn new(map_size: &TilemapSize) -> CollisionMap {
        CollisionMap {
            map: (0..map_size.count()).map(|_| false).collect(),
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

#[derive(Clone, Component, Default, Debug)]
struct CreatedColliders(Vec<Entity>);

impl CreatedColliders {
    fn clear(&self, commands: &mut Commands) {
        self.0
            .iter()
            .copied()
            .for_each(|e| commands.entity(e).despawn());
    }
}

fn update_collision_map(
    mut commands: Commands,
    collision_query: Query<(&Collision, &TilePos, &Parent), Changed<Collision>>,
    mut layer_query: Query<(&TilemapSize, Option<&mut CollisionMap>)>,
) {
    let mut new_collision_maps: HashMap<Entity, CollisionMap> = HashMap::new();

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

        collision_map.put(map_size, *pos, collision.solid());
    }

    // add new collision maps
    for (entity, collision_map) in new_collision_maps {
        commands.entity(entity).insert(collision_map);
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

fn create_colliders(
    mut commands: Commands,
    layer_query: Query<
        (
            Entity,
            &Parent,
            &TilemapSize,
            &TilemapTileSize,
            &CollisionMap,
            Option<&CreatedColliders>,
        ),
        Changed<CollisionMap>,
    >,
) {
    layer_query.for_each(
        |(entity, parent, map_size, tile_size, collision_map, created_colliders)| {
            // clear created colliders
            if let Some(colliders) = created_colliders {
                colliders.clear(&mut commands);
            }

            let colliders = create_colliders_for(
                parent.get(),
                &mut commands,
                map_size,
                tile_size,
                collision_map,
            );

            commands.entity(entity).insert(CreatedColliders(colliders));
        },
    )
}

fn create_colliders_for(
    parent_entity: Entity,
    commands: &mut Commands,
    map_size: &TilemapSize,
    tile_size: &TilemapTileSize,
    map: &CollisionMap,
) -> Vec<Entity> {
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
