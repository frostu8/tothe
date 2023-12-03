//! Provides simple projectile prefabs.

use bevy::ecs::system::Command;
use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use super::{Projectile, ProjectileBundle, TimeToLive};

use crate::GameAssets;

/// A projectile prefab.
///
/// Contains initial values for a projectile. When a projectile is created with
/// [`CreateProjectile`], this will be added as a component.
#[derive(Clone, Component, Debug)]
pub enum ProjectilePrefab {
    /// The player projectile; a wimpy, but fast moving projectile that cannot
    /// damage enemies but can be transformed.
    QuarterRest { initial_velocity: Vec2 },
}

impl ProjectilePrefab {
    /// Creates a new projectile in a world.
    pub fn create(&self, world: &mut World, location: Vec3) {
        world.resource_scope::<GameAssets, _>(|world, assets| {
            self.create_inner(world, &*assets, location)
        });
    }

    fn create_inner(&self, world: &mut World, assets: &GameAssets, location: Vec3) {
        match self {
            ProjectilePrefab::QuarterRest { initial_velocity } => {
                let rot = initial_velocity.y.atan2(initial_velocity.x);

                world.spawn((
                    ProjectileBundle {
                        transform: Transform::from_translation(location)
                            * Transform::from_rotation(Quat::from_axis_angle(Vec3::Z, rot)),
                        gravity_scale: GravityScale(0.),
                        projectile: Projectile {
                            //initial_speed: initial_velocity.length(),
                            ..Default::default()
                        },
                        collider: Collider::cuboid(2., 2.),
                        ..Default::default()
                    },
                    Velocity {
                        linvel: *initial_velocity,
                        angvel: 0.,
                    },
                    assets.projectile_sheet.clone(),
                    TextureAtlasSprite::new(0),
                    VisibilityBundle::default(),
                    TimeToLive::default(),
                ));
            }
        }
    }
}

/// A command that creates a projectile.
pub struct CreateProjectile {
    prefab: ProjectilePrefab,
    location: Vec3,
}

impl CreateProjectile {
    /// Creates a new `CreateProjectile`.
    pub fn new(prefab: ProjectilePrefab, location: Vec3) -> CreateProjectile {
        CreateProjectile { prefab, location }
    }
}

impl Command for CreateProjectile {
    fn apply(self, world: &mut World) {
        let CreateProjectile { prefab, location } = self;

        prefab.create(world, location);
    }
}
