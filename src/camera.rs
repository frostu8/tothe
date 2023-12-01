//! Camera follow and movement.

use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::ecs::query::QuerySingleError;
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::transform::{systems::propagate_transforms, TransformSystem};

use bevy_ecs_ldtk::{LdtkLevel, LevelSelection};

use crate::player::LocalPlayer;

pub const CLEAR_COLOR: Color = Color::rgb(0.03137, 0.03137, 0.03529);

/// Camera plugin.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_player_follow, update_current_level))
            .add_systems(
                PostUpdate,
                // This doesn't seem like good form, but it's the best idea I
                // have and the game jam is half over
                (camera_follow, bind_camera, propagate_transforms)
                    .chain()
                    .after(TransformSystem::TransformPropagate),
            )
            .add_systems(Startup, spawn_camera);
    }
}

/// A special component that makes the camera follow the player, and also
/// adjusts the LevelSelection resource.
#[derive(Clone, Component, Debug, Default)]
pub struct PlayerCamera;

/// A camera that's bound to the boundaries of a level.
#[derive(Clone, Component, Debug, Default)]
pub struct Constrained {
    /// The level id the camera is constrained in.
    pub level_id: Option<String>,
}

/// The camera will follow a subject.
///
/// # Note
/// An entity with this component cannot follow entities with this component.
#[derive(Clone, Component, Debug, Default)]
pub struct Follow(Option<Entity>);

/// A startup system that spawns the camera.
fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(CLEAR_COLOR),
            },
            projection: OrthographicProjection {
                far: 1000.,
                near: -1000.,
                scaling_mode: ScalingMode::FixedVertical(10. * 16.),
                ..Default::default()
            },
            ..Default::default()
        },
        Follow::default(),
        PlayerCamera::default(),
        Constrained::default(),
    ));
}

fn update_player_follow(
    mut camera_query: Query<&mut Follow, With<PlayerCamera>>,
    player_query: Query<Entity, With<LocalPlayer>>,
) {
    let player = match player_query.get_single() {
        Ok(player) => player,
        Err(QuerySingleError::NoEntities(_)) => return,
        Err(QuerySingleError::MultipleEntities(_)) => panic!("many players!"),
    };

    for mut follow in camera_query.iter_mut() {
        follow.0 = Some(player);
    }
}

fn update_current_level(
    camera_query: Query<&Constrained, (Changed<Constrained>, With<PlayerCamera>)>,
    mut level_selection: ResMut<LevelSelection>,
) {
    let Ok(constrained) = camera_query.get_single() else {
        return;
    };

    let Some(level_id) = constrained.level_id.to_owned() else {
        return;
    };

    *level_selection = LevelSelection::Identifier(level_id);
}

fn camera_follow(
    mut camera_query: Query<(&mut Transform, &Follow)>,
    transform_query: Query<&GlobalTransform, Without<Follow>>,
) {
    for (mut transform, follow) in camera_query.iter_mut() {
        // find subject
        let Some(subject) = follow.0 else {
            continue;
        };

        let Ok(subject) = transform_query.get(subject) else {
            continue;
        };

        // mimic transform
        *transform = Transform::from_translation(subject.translation());
    }
}

fn bind_camera(
    mut camera_query: Query<(&mut Transform, &mut Constrained, &OrthographicProjection)>,
    levels_query: Query<(&GlobalTransform, &Handle<LdtkLevel>)>,
    levels: Res<Assets<LdtkLevel>>,
    //mut gizmos: Gizmos,
) {
    for (mut transform, mut constrained, projection) in camera_query.iter_mut() {
        // get level rectangles
        let bound_space = levels_query
            .iter()
            .filter_map(|(t, level)| levels.get(level).map(|l| (t, l)))
            .map(|(t, level)| {
                // create rect from bounds
                let mut rect = Rect {
                    min: Vec2::new(0., 0.),
                    max: Vec2::new(level.level.px_wid as f32, level.level.px_hei as f32),
                };

                // transform
                rect.min = t.transform_point(rect.min.extend(1.)).truncate();
                rect.max = t.transform_point(rect.max.extend(1.)).truncate();

                (rect, level.level.identifier.clone())
            })
            .collect::<Vec<_>>();

        /*
        for rect in bound_space.iter() {
            gizmos.rect_2d((rect.min + rect.max) / 2., 0., rect.max - rect.min, Color::GREEN);
        }*/

        // get camera rectangle
        let mut camera_rect = projection.area;

        camera_rect.min = transform
            .transform_point(camera_rect.min.extend(1.))
            .truncate();
        camera_rect.max = transform
            .transform_point(camera_rect.max.extend(1.))
            .truncate();

        // TODO: move this to setting
        // constrict further so that camera cannot see nasty out of bounds
        camera_rect.min.y -= 8.;

        //gizmos.rect_2d((camera_rect.min + camera_rect.max) / 2., 0., camera_rect.max - camera_rect.min, Color::CYAN);

        // constrain
        let mtv = bound_space
            .into_iter()
            // find minimum translation vectors for each aabb
            .map(|(rect, lid)| {
                let x = if camera_rect.width() > rect.width() {
                    // there is no way to fit the camera in the rect, so use
                    // the difference of the centers
                    rect.center().x - camera_rect.center().x
                } else {
                    let left = rect.min.x - camera_rect.min.x;
                    let right = camera_rect.max.x - rect.max.x;

                    left.max(0.) - right.max(0.)
                };

                let y = if camera_rect.height() > rect.height() {
                    // there is no way to fit the camera in the rect, so use
                    // the difference of the centers
                    rect.center().y - camera_rect.center().y
                } else {
                    let bottom = rect.min.y - camera_rect.min.y;
                    let top = camera_rect.max.y - rect.max.y;

                    bottom.max(0.) - top.max(0.)
                };

                (Vec2::new(x, y), lid)
            })
            .reduce(|acc, v| {
                if v.0.length_squared() < acc.0.length_squared() {
                    v
                } else {
                    acc
                }
            });

        if let Some((mtv, level_id)) = mtv {
            constrained.level_id = Some(level_id);
            transform.translation += mtv.extend(0.);
        }
    }
}
