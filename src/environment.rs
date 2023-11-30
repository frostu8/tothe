//! Environment stuff.

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, _app: &mut App) {
    }

    fn finish(&self, app: &mut App) {
        app
            .register_ldtk_int_cell::<SolidBundle>(1);
    }
}

/// A bundle that indicates a solid 8x8px region.
#[derive(Bundle, LdtkIntCell)]
pub struct SolidBundle {
    #[with(initial_collider)]
    collider: Collider,
}

fn initial_collider(_: IntGridCell) -> Collider {
    Collider::cuboid(4., 4.)
}

