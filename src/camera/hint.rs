//! Camera hints.

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use bevy_ecs_ldtk::{
    app::{LdtkEntity, LdtkEntityAppExt},
    ldtk::ldtk_fields::LdtkFields as _,
    ldtk::{LayerInstance, TilesetDefinition},
    EntityInstance,
};

use super::{Follow, PlayerCamera};
use crate::{physics, player::LocalPlayer};

pub struct CameraHintPlugin;

impl Plugin for CameraHintPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (create_hint_entity, do_hint_sensor));
        //.add_systems(Update, debug_draw_hint_entity);
    }

    fn finish(&self, app: &mut App) {
        app.register_ldtk_entity::<CameraHintBundle>("CameraHint");
    }
}

/// A bundle for camera hints.
#[derive(Bundle, Debug)]
pub struct CameraHintBundle {
    camera_hint: CameraHint,
}

impl LdtkEntity for CameraHintBundle {
    fn bundle_entity(
        entity_instance: &EntityInstance,
        layer_instance: &LayerInstance,
        _tileset: Option<&Handle<Image>>,
        _tileset_definition: Option<&TilesetDefinition>,
        _asset_server: &AssetServer,
        _texture_atlases: &mut Assets<TextureAtlas>,
    ) -> Self {
        /*
        bevy::log::info!("{}", entity_instance.width);
        // thankfully pixels are world units
        let hint_size = Vec2::new(entity_instance.width as f32, entity_instance.height as f32)
            / layer_instance.grid_size as f32;*/

        let mut hint_grid_position = entity_instance
            .get_point_field("Hint")
            .expect("valid hint")
            .clone();

        // convert to bevy coordinates
        hint_grid_position.y = layer_instance.c_hei - hint_grid_position.y - 1;

        let hint_pixel_position = hint_grid_position * layer_instance.grid_size
            + IVec2::splat(layer_instance.grid_size / 2);
        let hint_position = Vec2::new(hint_pixel_position.x as f32, hint_pixel_position.y as f32);

        CameraHintBundle {
            camera_hint: CameraHint::new(hint_position),
        }
    }
}

/// A trigger in the world that adds an extra focus to the player camera.
#[derive(Clone, Component, Debug)]
pub struct CameraHint {
    /// Hint position in the level.
    hint_position: Vec2,
}

impl CameraHint {
    /// Creates a new `CameraHint`.
    pub fn new(hint_position: Vec2) -> CameraHint {
        CameraHint { hint_position }
    }
}

/// The actual trigger for the hint.
///
/// The associated entity is the hint.
#[derive(Clone, Component, Debug)]
pub struct CameraHintSensor(pub Entity);

fn create_hint_entity(
    mut commands: Commands,
    new_entity_query: Query<(Entity, Option<&Parent>, &CameraHint), Added<CameraHint>>,
) {
    for (entity, parent, camera_hint) in new_entity_query.iter() {
        bevy::log::info!("Wtscallop");
        let parent = parent.map(|p| p.get());

        // create the hint entity
        let hint_entity = if let Some(parent) = parent {
            commands
                .spawn(TransformBundle {
                    local: Transform::from_translation(camera_hint.hint_position.extend(0.)),
                    global: Default::default(),
                })
                .set_parent(parent)
                .id()
        } else {
            commands
                .spawn(TransformBundle {
                    local: Transform::from_translation(camera_hint.hint_position.extend(0.)),
                    global: Default::default(),
                })
                .id()
        };

        // create the sensor
        commands.entity(entity).insert((
            Collider::cuboid(4., 4.),
            CollisionGroups::new(
                physics::COLLISION_GROUP_TRIGGER,
                physics::COLLISION_GROUP_FRIENDLY,
            ),
            ActiveEvents::COLLISION_EVENTS,
            Sensor::default(),
            CameraHintSensor(hint_entity),
        ));
    }
}

fn do_hint_sensor(
    mut player_camera_query: Query<&mut Follow, With<PlayerCamera>>,
    player_query: Query<Entity, With<LocalPlayer>>,
    hint_sensor_query: Query<&CameraHintSensor>,
    mut collision_events: EventReader<CollisionEvent>,
) {
    let Ok(mut follow) = player_camera_query.get_single_mut() else {
        return;
    };

    for ev in collision_events.iter() {
        let (e1, e2, entered) = match *ev {
            CollisionEvent::Started(e1, e2, _) => (e1, e2, true),
            CollisionEvent::Stopped(e1, e2, _) => (e1, e2, false),
        };

        // find sensor and subject
        let (sensor, subject) = if let Ok(hint_sensor) = hint_sensor_query.get(e1) {
            (hint_sensor, e2)
        } else if let Ok(hint_sensor) = hint_sensor_query.get(e2) {
            (hint_sensor, e1)
        } else {
            continue;
        };

        if player_query.contains(subject) {
            if entered {
                // add hint to focus
                let mut new_subjects = follow.subjects().to_owned();
                new_subjects.push(sensor.0);

                follow.update(new_subjects);
            } else {
                // remove hint from focus
                let mut new_subjects = follow.subjects().to_owned();
                new_subjects.retain(|&e| e != sensor.0);

                follow.update(new_subjects);
            }
        }
    }
}

#[allow(dead_code)]
fn debug_draw_hint_entity(
    hints_query: Query<&CameraHintSensor>,
    transform_query: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    for hint in hints_query.iter() {
        let Ok(pos) = transform_query.get(hint.0) else {
            continue;
        };

        gizmos.circle(pos.translation(), Vec3::Z, 4., Color::BLUE);
    }
}
