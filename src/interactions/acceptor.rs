//! Entrances to things.

use bevy::ecs::query::WorldQuery;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use std::time::Duration;

use crate::enemy::Hostility;
use crate::projectile::{HitEvent, Projectile, ProjectileSystem};

use super::{Signal, SignalData, SignalEvent};

/// Acceptor plugin.
pub struct AcceptorPlugin;

impl Plugin for AcceptorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            accept_projectiles
                .after(ProjectileSystem::Event)
                .before(ProjectileSystem::Despawn),
        )
        .add_systems(Update, update_ghost_projectiles);
    }
}

/// A bundle for an [`Acceptor`].
#[derive(Bundle, Clone, Debug, Default)]
pub struct AcceptorBundle {
    pub acceptor: Acceptor,
    pub collider: Collider,
}

/// An acceptor is great.
///
/// It "consumes projectiles" and turns them into signals. When the projectiles
/// hit the collider on this object, instead of being absorbed, they will be
/// disabled and an associated [`Signal`] is created.
#[derive(Clone, Component, Debug, Default)]
pub struct Acceptor;

/// A spooky ghost.
///
/// This is created when an acceptor accepts a [`Projectile`], but it wants the
/// projectile to visually go into the acceptor.
#[derive(Clone, Component, Debug, Default)]
pub struct GhostProjectile {
    initial: Vec2,
    target: Vec2,
    time_to_live: Timer,
}

impl GhostProjectile {
    /// Creates a new `GhostProjectile`.
    pub fn new(initial: Vec2, target: Vec2, duration: Duration) -> GhostProjectile {
        GhostProjectile {
            initial,
            target,
            time_to_live: Timer::new(duration, TimerMode::Once),
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct ProjectileQuery {
    entity: Entity,
    name: DebugName,
    projectile: &'static mut Projectile,
    hostility: &'static Hostility,
    //rigidbody: &'static mut RigidBody,
    //collision_groups: &'static mut CollisionGroups,
    //visibility: &'static mut Visibility,
}

#[derive(WorldQuery)]
struct CreateGhostQuery {
    sprite: &'static TextureAtlasSprite,
    texture_atlas: &'static Handle<TextureAtlas>,
    transform: &'static GlobalTransform,
    velocity: &'static Velocity,
}

fn accept_projectiles(
    mut commands: Commands,
    mut hit_events: EventReader<HitEvent>,
    acceptor_query: Query<(Entity, &GlobalTransform, &Acceptor)>,
    mut projectile_query: Query<(ProjectileQuery, CreateGhostQuery)>,
    mut signal_events: EventWriter<SignalEvent>,
) {
    for ev in hit_events.iter() {
        match (
            projectile_query.get_mut(ev.projectile),
            acceptor_query.get(ev.entity),
        ) {
            (Ok((mut proj, create_ghost)), Ok((me, acceptor_transform, _acceptor))) => {
                // accept projectile
                //*proj.visibility = Visibility::Hidden;
                //*proj.rigidbody = RigidBody::Fixed;

                // cancel absorb
                proj.projectile.absorbed = false;

                commands.entity(proj.entity).despawn_recursive();

                bevy::log::info!("accepted projectile {:?}", proj.name);

                // create new ghost
                commands.spawn((
                    SpriteSheetBundle {
                        sprite: create_ghost.sprite.clone(),
                        texture_atlas: create_ghost.texture_atlas.clone(),
                        transform: create_ghost.transform.clone().into(),
                        ..Default::default()
                    },
                    GhostProjectile::new(
                        create_ghost.transform.translation().truncate(),
                        acceptor_transform.translation().truncate(),
                        std::cmp::min(
                            Duration::from_secs_f32(16. / create_ghost.velocity.linvel.length()),
                            Duration::from_millis(500),
                        ),
                    ),
                ));

                // create new signal
                let signal = commands.spawn((
                    TransformBundle::default(),
                    Signal::at(
                        SignalData {
                            hostility: proj.hostility.clone(),
                        },
                        me,
                    ),
                )).id();
                signal_events.send(SignalEvent {
                    receiver: me,
                    sender: me,
                    signal,
                    overfill: 0.,
                });
            }
            // skip other events
            _ => (),
        }
    }
}

fn update_ghost_projectiles(
    mut commands: Commands,
    mut ghost_query: Query<(Entity, &mut Transform, &mut GhostProjectile)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut ghost) in ghost_query.iter_mut() {
        ghost.time_to_live.tick(time.delta());

        if ghost.time_to_live.finished() {
            commands.entity(entity).despawn_recursive();
        } else {
            // lerp
            transform.translation = ghost
                .initial
                .lerp(ghost.target, ghost.time_to_live.percent())
                .extend(transform.translation.z);

            transform.scale = Vec3::splat(1. - ghost.time_to_live.percent());
        }
    }
}
