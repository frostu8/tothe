use bevy::prelude::*;
use bevy::render::render_resource::{FilterMode, SamplerDescriptor};

use bevy_ecs_ldtk::prelude::*;

use bevy_rapier2d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // fill the entire browser window
                fit_canvas_to_parent: true,
                // don't hijack keyboard shortcuts like F5, F6, F12, Ctrl+R etc.
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }).set(ImagePlugin {
            default_sampler: SamplerDescriptor {
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmap_filter: FilterMode::Nearest,
                ..Default::default()
            },
        }))
        .add_plugins(LdtkPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(8.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(tothe::player::PlayerPlugin)
        .add_plugins(tothe::player::controller::ControllerPlugin)
        .add_plugins(tothe::environment::EnvironmentPlugin)
        .add_systems(Startup, tothe::setup)
        .insert_resource(LevelSelection::Index(0))
        .run();
}

