//! Pipe layer things.

use bevy::prelude::*;
use bevy::transform::TransformSystem;
use bevy_ecs_ldtk::prelude::*;
use bevy_ecs_tilemap::{
    map::{TilemapId, TilemapSize},
    tiles::{TileBundle, TilePos, TileStorage, TileTextureIndex},
};
use bevy_rapier2d::prelude::*;

use std::collections::HashSet;
use std::convert::identity;

use crate::interactions::{
    acceptor::{Acceptor, AcceptorBundle},
    generator::Generator,
    Buldge, Junction,
};
use crate::projectile::prefab::ProjectilePrefab;

/// Creates pipes from LDTK levels.
///
/// This plugin was agonizing to write.
pub struct LevelPipePlugin;

impl Plugin for LevelPipePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, mark_pipes_layer).add_systems(
            PostUpdate,
            (merge_pipes_down, build_pipe_network).before(TransformSystem::TransformPropagate),
        );
    }

    fn finish(&self, app: &mut App) {
        app.register_default_ldtk_int_cell_for_layer::<PipeSegmentBundle>("Pipes")
            .register_default_ldtk_entity_for_layer::<PipeEntityBundle>("PipeEntities");
    }
}

/// A bundle for pipe segments.
#[derive(Bundle)]
pub struct PipeSegmentBundle {
    segment: PipeSegment,
    junction: Junction,
}

impl LdtkIntCell for PipeSegmentBundle {
    fn bundle_int_cell(int_grid_cell: IntGridCell, _layer_instance: &LayerInstance) -> Self {
        PipeSegmentBundle {
            segment: match int_grid_cell.value {
                1 => PipeSegment::Blue,
                2 => PipeSegment::Red,
                _ => panic!("invalid pipe value"),
            },
            junction: Junction::default(),
        }
    }
}

/// A bundle for pipe entities.
#[derive(Bundle, LdtkEntity)]
pub struct PipeEntityBundle {
    #[grid_coords]
    grid_coords: GridCoords,
    #[with(PipeEntity::from_entity_instance)]
    pipe_entity: PipeEntity,
}

/// A pipe entity that will give the corresponding tile in the `Pipes` layer
/// special interactions.
#[derive(Clone, Component, Debug)]
pub enum PipeEntity {
    /// A pipe exit.
    Exit(Direction),
    /// A vertical chute.
    ///
    /// * `direction`: direction of exiting projectiles.
    ChuteVertical(f32),
}

impl PipeEntity {
    /// Creates a `PipeEntity` from an [`EntityInstance`].
    ///
    /// # Panics
    /// Panics if the [`EntityInstance`] is invalid or unexpected.
    pub fn from_entity_instance(inst: &EntityInstance) -> Self {
        match inst.identifier.as_ref() {
            "PipeExitLeft" => PipeEntity::Exit(Direction::Left),
            "PipeChuteVertical" => {
                let direction = inst.get_float_field("Direction").expect("valid direction");

                PipeEntity::ChuteVertical(*direction)
            }
            _ => panic!("invalid identifier"),
        }
    }

    /// Gets the texture index of the tileset (`pipes.png`).
    pub fn texture_index(&self) -> u32 {
        match self {
            PipeEntity::Exit(Direction::Left) => 0,
            PipeEntity::ChuteVertical(_) => 10,
            _ => todo!(),
        }
    }
}

/// A direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Direction {
    #[default]
    Right,
    Up,
    Left,
    Down,
}

impl Direction {
    /// Gets the vector of the direction.
    pub fn axis(self) -> Vec2 {
        match self {
            Direction::Right => Vec2::X,
            Direction::Up => Vec2::Y,
            Direction::Left => -Vec2::X,
            Direction::Down => -Vec2::Y,
        }
    }
}

/// A pipe segment.
#[derive(Clone, Copy, Component, Debug, Default, PartialEq, Eq, Hash)]
pub enum PipeSegment {
    /// Part of the blue pipes.
    #[default]
    Blue,
    /// Part of the red pipes.
    Red,
}

/// Marker trait for the pipes layer.
#[derive(Clone, Component, Debug, Default)]
pub struct PipesLayer;

fn mark_pipes_layer(
    mut commands: Commands,
    new_layers_query: Query<(Entity, &LayerMetadata), Added<LayerMetadata>>,
) {
    for (entity, layer) in new_layers_query.iter() {
        if layer.identifier == "Pipes" {
            commands.entity(entity).insert(PipesLayer);
        }
    }
}

fn merge_pipes_down(
    mut commands: Commands,
    new_pipes_query: Query<(Entity, &GridCoords, &PipeEntity, &Parent)>,
    levels_query: Query<&Children>,
    mut layers_query: Query<(Entity, &mut TileStorage), With<PipesLayer>>,
) {
    for (new_pipe_entity, grid_coords, pipe_entity, parent) in new_pipes_query.iter() {
        let Ok(level_children) = levels_query.get(parent.get()) else {
            continue;
        };

        // find pipes layer
        let mut layers = layers_query.iter_many_mut(level_children);

        // skip if pipes layer has not been marked yet
        if let Some((layer_entity, mut pipes_layer)) = layers.fetch_next() {
            // find tile in grid
            let pos = TilePos::new(grid_coords.x as u32, grid_coords.y as u32);
            let entity = match pipes_layer.get(&pos) {
                Some(entity) => entity,
                None => {
                    let entity = commands.spawn_empty().id();

                    pipes_layer.set(&pos, entity);

                    entity
                }
            };

            // update tile
            commands.entity(entity).insert(TileBundle {
                position: pos,
                tilemap_id: TilemapId(layer_entity),
                texture_index: TileTextureIndex(pipe_entity.texture_index()),
                ..Default::default()
            });

            // add exciting stuff
            match pipe_entity {
                PipeEntity::ChuteVertical(dir) => {
                    commands.entity(entity).insert((
                        AcceptorBundle {
                            collider: Collider::cuboid(6., 8.),
                            acceptor: Acceptor,
                        },
                        Generator {
                            prefab: ProjectilePrefab::QuarterNote {
                                // TODO: magic number
                                initial_velocity: Vec2::new(*dir, 0.) * 128.,
                            },
                            location: Vec3::new(9f32.copysign(*dir), 0., 0.),
                        },
                        Name::new("ChuteVertical"),
                        Junction::default(),
                        Buldge::no_cover(),
                    ));
                }
                PipeEntity::Exit(direction) => {
                    let location = match direction {
                        Direction::Left => Vec3::new(-8., -6., 0.),
                        _ => todo!(),
                    };

                    commands.entity(entity).insert((
                        Generator {
                            prefab: ProjectilePrefab::BeamNote {
                                // TODO: magic number
                                initial_direction: direction.axis().x * 32.,
                            },
                            location,
                        },
                        Name::new("Exit"),
                        Junction::default(),
                        Buldge::no_cover(),
                    ));
                }
            }

            // delete old pipeentity
            commands.entity(new_pipe_entity).remove::<PipeEntity>();
        }
    }
}

// lol idc anymore I just want this to work
fn build_pipe_network(
    mut param_set: ParamSet<(Query<&mut Junction>, Query<&Parent, Changed<Junction>>)>,
    //mut junctions_query: Query<&mut Junction>,
    colors_query: Query<&PipeSegment>,
    //added_junctions: Query<&Parent, Added<Junction>>,
    layers_query: Query<&TileStorage, With<PipesLayer>>,
) {
    // look for changes
    let mut changed_layers = HashSet::new();

    changed_layers.extend(
        param_set
            .p1()
            .iter()
            .map(|p| p.get())
            .filter(|&p| layers_query.contains(p)),
    );

    for tiles in layers_query.iter() {
        for y in 0..tiles.size.y {
            for x in 0..tiles.size.x {
                let pos = TilePos::new(x, y);

                build_junction(&mut param_set.p0(), &colors_query, tiles, pos);
            }
        }
    }
}

fn build_junction(
    junctions_query: &mut Query<&mut Junction>,
    colors_query: &Query<&PipeSegment>,
    tiles: &TileStorage,
    pos: TilePos,
) {
    let Some(tile_entity) = tiles.get(&pos) else {
        return;
    };

    let color = colors_query.get(tile_entity).ok();

    if let Ok(mut junction) = junctions_query.get_mut(tile_entity) {
        junction.clear();
    }

    for neighbor_pos in neighbor_positions(&tiles.size, &pos)
        .into_iter()
        .filter_map(identity)
    {
        let Some(neighbor_entity) = tiles.get(&neighbor_pos) else {
            continue;
        };

        let neighbor_color = colors_query.get(neighbor_entity).ok();

        let compatible = match (color, neighbor_color) {
            (Some(color), Some(neighbor_color)) if color == neighbor_color => true,
            (Some(_), Some(_)) => false,
            _ => true,
        };

        if compatible && junctions_query.contains(neighbor_entity) {
            let Ok(mut junction) = junctions_query.get_mut(tile_entity) else {
                continue;
            };

            junction.push_pipe(neighbor_entity);
        }
    }
}

fn neighbor_positions(size: &TilemapSize, pos: &TilePos) -> [Option<TilePos>; 4] {
    let pos = IVec2::new(pos.x as i32, pos.y as i32);

    [IVec2::X, IVec2::Y, -IVec2::X, -IVec2::Y].map(|n| {
        let pos = pos + n;

        let x_valid = pos.x >= 0 && (pos.x as u32) < size.x;
        let y_valid = pos.y >= 0 && (pos.y as u32) < size.y;

        if x_valid && y_valid {
            Some(TilePos::new(pos.x as u32, pos.y as u32))
        } else {
            None
        }
    })
}
