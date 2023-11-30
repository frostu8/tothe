//! `tothe` library.

pub mod environment;
pub mod player;

use bevy::prelude::*;
use bevy::core_pipeline::clear_color::ClearColorConfig;
use bevy::render::camera::ScalingMode;

use bevy_ecs_ldtk::prelude::*;

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let clear_color = Color::hex("080809").expect("valid hex");

    commands.spawn(Camera2dBundle {
        camera_2d: Camera2d {
            clear_color: ClearColorConfig::Custom(clear_color),
        },
        projection: OrthographicProjection {
            far: 1000.,
            near: -1000.,
            scaling_mode: ScalingMode::FixedVertical(10. * 16.),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("world/world.ldtk"),
        //transform: Transform::from_scale(Vec3::splat(16.)),
        ..Default::default()
    });
}
