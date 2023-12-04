use bevy::input::gamepad::{AxisSettings, GamepadSettings};
use bevy::prelude::*;
use bevy::render::render_resource::{FilterMode, SamplerDescriptor};
use bevy::ecs::schedule::{ScheduleBuildSettings, LogLevel};

use bevy_ecs_ldtk::prelude::*;

use bevy_rapier2d::prelude::*;

use tothe::GamePlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        // fill the entire browser window
                        fit_canvas_to_parent: true,
                        // don't hijack keyboard shortcuts like F5, F6, F12, Ctrl+R etc.
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: SamplerDescriptor {
                        mag_filter: FilterMode::Nearest,
                        min_filter: FilterMode::Nearest,
                        mipmap_filter: FilterMode::Nearest,
                        ..Default::default()
                    },
                }),
        )
        .edit_schedule(
            Update,
            |schedule| {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Warn,
                    ..Default::default()
                });
            },
        )
        .add_plugins(LdtkPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(8.0))
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .add_plugins(GamePlugin)
        .insert_resource(LevelSelection::Identifier("Level_0".into()))
        .insert_resource(RapierConfiguration {
            // good arcade gravity
            gravity: Vec2::new(0., -9.81 * 72.),
            ..Default::default()
        })
        .insert_resource(LdtkSettings {
            level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation {
                load_level_neighbors: true,
            },
            ..Default::default()
        })
        .insert_resource(GamepadSettings {
            default_axis_settings: AxisSettings::new(-1., -0.3, 0.3, 1., 0.05)
                .expect("valid axis settings"),
            ..Default::default()
        })
        .run();
}
