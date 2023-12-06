//! Camera cursor stuff.

use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::window::{PrimaryWindow, WindowRef};

/// Camera cursor plugin.
pub struct CameraCursorPlugin;

impl Plugin for CameraCursorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_cursor_in_world);
    }
}

/// The world position of the cursor.
#[derive(Clone, Component, Debug, Default)]
pub struct CursorWorldPosition(pub Vec2);

fn update_cursor_in_world(
    mut camera_query: Query<(&GlobalTransform, &Camera, &mut CursorWorldPosition)>,
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    windows_query: Query<&Window>,
) {
    for (transform, camera, mut pos) in camera_query.iter_mut() {
        // find window ref
        let RenderTarget::Window(window_ref) = camera.target else {
            continue;
        };

        // get window
        let window = match window_ref {
            WindowRef::Primary => primary_window_query.single(),
            WindowRef::Entity(ent) => match windows_query.get(ent) {
                Ok(window) => window,
                Err(_) => continue,
            },
        };

        // get cursor pos
        let Some(mut cursor_pos) = window.cursor_position() else {
            continue;
        };
        let target_size = Vec2::new(window.width(), window.height());

        cursor_pos.y = target_size.y - cursor_pos.y;
        let ndc_pos = cursor_pos * 2. / target_size - Vec2::ONE;

        let Some(world_pos) = camera.ndc_to_world(transform, ndc_pos.extend(1.)) else {
            continue;
        };

        pos.0 = world_pos.truncate();
    }
}
