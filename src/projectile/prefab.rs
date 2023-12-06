//! Provides simple projectile prefabs.

use bevy::ecs::system::Command;
use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use super::{Bounce, Projectile, ProjectileBundle, SineWave, Squish, TimeToLive};

use crate::enemy::Hostility;
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
    /// A quarter note that sways up and down on a sine wave.
    QuarterNote { initial_velocity: Vec2 },
    /// A beam note that bouncess. If the direction is `0`, it will choose a
    /// random direction to bounce into.
    BeamNote { initial_direction: f32 },
}

impl ProjectilePrefab {
    /// Creates a new projectile in a world.
    pub fn create(&self, world: &mut World, location: Vec3, hostility: Hostility) {
        world.resource_scope::<GameAssets, _>(|world, assets| {
            self.create_inner(world, &*assets, location, hostility)
        });
    }

    fn create_inner(
        &self,
        world: &mut World,
        assets: &GameAssets,
        location: Vec3,
        hostility: Hostility,
    ) {
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
                        hostility,
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
            ProjectilePrefab::QuarterNote { initial_velocity } => {
                let velocity_normal = initial_velocity.normalize();

                //  |\/\/\/|
                //  |      |
                //  |      |
                //  | (o)(o)
                //  C      _)
                //   | ,___|
                //   |   /
                //  /____\
                // /      \
                world.spawn((
                    ProjectileBundle {
                        transform: Transform::from_translation(location),
                        gravity_scale: GravityScale(0.),
                        projectile: Projectile {
                            //initial_speed: initial_velocity.length(),
                            ..Default::default()
                        },
                        collider: Collider::cuboid(2., 2.),
                        hostility,
                        ..Default::default()
                    },
                    Velocity {
                        linvel: *initial_velocity,
                        angvel: 0.,
                    },
                    SineWave {
                        axis: Vec2::new(velocity_normal.y, -velocity_normal.x),
                        period: 16.,
                        amp: 2.,
                        ..Default::default()
                    },
                    assets.projectile_sheet.clone(),
                    TextureAtlasSprite::new(2),
                    VisibilityBundle::default(),
                    TimeToLive::default(),
                ));
            }
            ProjectilePrefab::BeamNote { initial_direction } => {
                world
                    .spawn((
                        ProjectileBundle {
                            transform: Transform::from_translation(location),
                            gravity_scale: GravityScale(0.5),
                            projectile: Projectile {
                                //initial_speed: initial_velocity.length(),
                                ..Default::default()
                            },
                            collider: Collider::cuboid(2., 2.),
                            hostility,
                            ..Default::default()
                        },
                        Velocity {
                            linvel: Vec2::new(*initial_direction, 0.),
                            angvel: 0.,
                        },
                        Bounce::default(),
                        LockedAxes::ROTATION_LOCKED,
                        VisibilityBundle::default(),
                        TimeToLive::default(),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            SpriteSheetBundle {
                                texture_atlas: assets.projectile_sheet.clone(),
                                sprite: TextureAtlasSprite::new(1),
                                ..Default::default()
                            },
                            hostility,
                            Squish::default(),
                        ));
                    });
            }
        }
    }
}

/// A command that creates a projectile.
pub struct CreateProjectile {
    prefab: ProjectilePrefab,
    location: Vec3,
    hostility: Hostility,
}

impl CreateProjectile {
    /// Creates a new `CreateProjectile`.
    pub fn new(prefab: ProjectilePrefab, location: Vec3) -> CreateProjectile {
        CreateProjectile {
            prefab,
            location,
            hostility: Hostility::default(),
        }
    }

    /// Sets the hostility.
    pub fn hostility(self, hostility: Hostility) -> CreateProjectile {
        CreateProjectile { hostility, ..self }
    }
}

impl Command for CreateProjectile {
    fn apply(self, world: &mut World) {
        let CreateProjectile {
            prefab,
            location,
            hostility,
        } = self;

        prefab.create(world, location, hostility);
    }
}
