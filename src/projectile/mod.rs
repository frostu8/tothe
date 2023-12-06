//! Projectile things.

pub mod prefab;
pub mod residue;
pub mod spawner; // TODO: move to playe mod

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use std::time::Duration;

use crate::enemy::Hostility;
use crate::physics;

/// Projectile plugin.
pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HitEvent>()
            .add_event::<DespawnEvent>()
            .add_systems(
                Update,
                (
                    (create_hit_events, set_absorb_flag).chain(),
                    synchronize_your_death_watches_lads,
                )
                    .in_set(ProjectileSystem::Event),
            )
            .add_systems(
                Update,
                despawn_projectiles
                    .in_set(ProjectileSystem::Despawn)
                    .after(ProjectileSystem::Event),
            )
            .add_systems(
                Update,
                (bounce_projectiles, animate_squish)
                    .after(ProjectileSystem::Event)
                    .before(ProjectileSystem::Despawn),
            )
            .add_systems(FixedUpdate, projectile_sine_wave)
            .add_systems(PostUpdate, (update_collision_groups, update_sprite_color));
    }
}

/// Projectile systems.
#[derive(Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum ProjectileSystem {
    /// Event systems
    Event,
    /// Despawns projectiles.
    ///
    /// Update [`Projectile::absorbed`] before this systme.
    Despawn,
}

/// Projectile bundle.
#[derive(Bundle)]
pub struct ProjectileBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub rigidbody: RigidBody,
    pub collider: Collider,
    pub active_events: ActiveEvents,
    pub collision_groups: CollisionGroups,
    pub gravity_scale: GravityScale,
    pub projectile: Projectile,
    pub contact_behavior: ContactBehavior,
    pub hostility: Hostility,
}

impl Default for ProjectileBundle {
    fn default() -> ProjectileBundle {
        ProjectileBundle {
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            rigidbody: RigidBody::Dynamic,
            collider: Collider::default(),
            active_events: ActiveEvents::COLLISION_EVENTS,
            collision_groups: CollisionGroups::new(Group::empty(), Group::empty()),
            gravity_scale: GravityScale(0.),
            projectile: Projectile::default(),
            contact_behavior: ContactBehavior::Absorb,
            hostility: Hostility::Friendly,
        }
    }
}

/// A single projectile.
#[derive(Clone, Component, Debug, Default)]
pub struct Projectile {
    //pub initial_speed: f32,
    /// Whether the projectile is being absorbed this frame.
    ///
    /// Set this to false to prevent the projectile from being absorbed. This
    /// cannot prevent projectiles being killed from [`TimeToLive`].
    pub absorbed: bool,
}

/// Determines the despawn behavior of projectiles.
///
/// In an interaction between a projectile and an entity, if both the
/// projectile and entity have [`ContactBehavior::Bounce`], the projectile will
/// go on its merry way. But if one entity has [`ContactBehavior::Absorb`], it
/// will be despawned.
#[derive(Clone, Copy, Component, Debug, Default, PartialEq, Eq)]
pub enum ContactBehavior {
    /// Bounces off the entity.
    ///
    /// This is the default behavior.
    #[default]
    Bounce,
    /// Absorb the projectile, i.e. despawns it.
    Absorb,
}

impl ContactBehavior {
    /// Merges two contact behaviors together.
    pub fn and(self, other: ContactBehavior) -> ContactBehavior {
        use ContactBehavior::*;

        match (self, other) {
            (Bounce, Bounce) => Bounce,
            _ => Absorb,
        }
    }
}

/// Makes a projectile sway on a sine wave.
#[derive(Clone, Component, Debug)]
pub struct SineWave {
    /// The axis of the sine wave.
    pub axis: Vec2,
    /// The period of the sine wave. A period of `1.` means the wave will cycle
    /// each `2pi` seconds.
    pub period: f32,
    /// The amplitude of the wave in world coordinates.
    pub amp: f32,

    ticks: u32,
}

impl SineWave {
    /// The velocity of the current frame.
    pub fn velocity(&self, timestep: Duration) -> f32 {
        let SineWave {
            period, amp, ticks, ..
        } = *self;

        let time = (timestep * ticks).as_secs_f32();

        amp * period * (time * period).cos()
    }
}

impl Default for SineWave {
    fn default() -> SineWave {
        SineWave {
            axis: Vec2::Y,
            period: 1.,
            amp: 1.,
            ticks: 0,
        }
    }
}

/// A component for projectiles that will bounce off the ground.
#[derive(Clone, Component, Debug, Default)]
pub struct Bounce {
    height: Option<f32>,
}

/// A component coupled with [`Bounce`] to make projectiles squish visually.
#[derive(Clone, Component, Debug)]
pub struct Squish {
    /// How fast the squish will return to normal size, per second.
    pub retention: f32,
    /// The current squish value.
    pub squish: f32,
}

impl Default for Squish {
    fn default() -> Squish {
        Squish {
            retention: 1.,
            squish: 1.,
        }
    }
}

/// Despawns a projectile if it lives for too long.
///
/// Although this is a relatively generic and useful component, is included in
/// the projectile mod for simplicity, as it is most relevant when creating
/// empheremal projectiles.
#[derive(Clone, Component, Debug)]
pub struct TimeToLive(Timer);

impl TimeToLive {
    /// Creates a new `TimeToLive`.
    pub fn new(duration: Duration) -> TimeToLive {
        TimeToLive(Timer::new(duration, TimerMode::Once))
    }
}

impl Default for TimeToLive {
    /// Initializes a default `TimeToLive` for 60 seconds.
    fn default() -> TimeToLive {
        TimeToLive::new(Duration::from_secs(60))
    }
}

/// A contact between a projectile and an entity occured.
#[derive(Debug, Event)]
pub struct HitEvent {
    /// The projectile.
    ///
    /// Has a [`Projectile`] component that can be queried.
    pub projectile: Entity,
    /// The other entity.
    pub entity: Entity,
    /// The result of the interaction.
    pub result: ContactBehavior,
}

/// A projectile has despawned after living for too long.
#[derive(Debug, Event)]
pub struct DespawnEvent {
    /// The projectile.
    pub projectile: Entity,
}

fn synchronize_your_death_watches_lads(
    mut time_to_live_query: Query<(Entity, &mut TimeToLive)>,
    mut despawn_events: EventWriter<DespawnEvent>,
    time: Res<Time>,
) {
    for (entity, mut time_to_live) in time_to_live_query.iter_mut() {
        time_to_live.0.tick(time.delta());

        if time_to_live.0.finished() {
            despawn_events.send(DespawnEvent { projectile: entity });
        }
    }
}

fn projectile_sine_wave(
    mut sine_wave_query: Query<(&mut SineWave, &mut Velocity)>,
    time: Res<FixedTime>,
) {
    for (mut sine_wave, mut velocity) in sine_wave_query.iter_mut() {
        // preserve perpendicular velocity
        let perp = sine_wave.axis.perp();

        let perp_vel = velocity.linvel.dot(perp) * perp;

        let vel = sine_wave.axis * sine_wave.velocity(time.period);

        velocity.linvel = perp_vel + vel;
        sine_wave.ticks += 1;
    }
}

fn bounce_projectiles(
    mut bounce_query: Query<(
        &GlobalTransform,
        &Children,
        &mut Bounce,
        &mut Velocity,
        &mut Projectile,
        &GravityScale,
    )>,
    mut squish_query: Query<&mut Squish>,
    physics_config: Res<RapierConfiguration>,
) {
    for (transform, children, mut bounce, mut velocity, mut projectile, gravity_scale) in
        bounce_query.iter_mut()
    {
        if bounce.height.is_none() {
            bounce.height = Some(transform.translation().y);
        }

        let height_diff = bounce.height.unwrap() - transform.translation().y;

        if projectile.absorbed {
            projectile.absorbed = false;

            // find velocity it would take to reach the same height
            let gravity = physics_config.gravity * gravity_scale.0;
            let vel = (-2. * gravity.y * height_diff).sqrt();

            velocity.linvel.y = vel;

            // setup squish animation
            let mut children = squish_query.iter_many_mut(children);

            while let Some(mut squish) = children.fetch_next() {
                squish.squish = 0.7;
            }
        }
    }
}

fn animate_squish(mut squish_query: Query<(&mut Transform, &mut Squish)>, time: Res<Time>) {
    for (mut transform, mut squish) in squish_query.iter_mut() {
        transform.scale.y = squish.squish;

        squish.squish = (squish.squish + squish.retention * time.delta_seconds()).min(1.);
    }
}

fn update_collision_groups(
    mut projectile_query: Query<
        (&Hostility, &mut CollisionGroups),
        (With<Projectile>, Changed<Hostility>),
    >,
) {
    for (hostility, mut collision_groups) in projectile_query.iter_mut() {
        match *hostility {
            Hostility::Friendly => {
                *collision_groups = CollisionGroups::new(
                    physics::COLLISION_GROUP_PROJECTILE,
                    physics::COLLISION_GROUP_SOLID | physics::COLLISION_GROUP_HOSTILE,
                );
            }
            Hostility::Hostile => {
                *collision_groups = CollisionGroups::new(
                    physics::COLLISION_GROUP_PROJECTILE,
                    physics::COLLISION_GROUP_SOLID | physics::COLLISION_GROUP_FRIENDLY,
                );
            }
        }
    }
}

fn update_sprite_color(
    projectile_query: Query<(Entity, &Hostility), (With<Projectile>, Changed<Hostility>)>,
    children_query: Query<&Children>,
    mut texture_atlas_query: Query<&mut TextureAtlasSprite>,
) {
    for (proj_entity, hostility) in projectile_query.iter() {
        for entity in children_query
            .iter_descendants(proj_entity)
            .chain(std::iter::once(proj_entity))
        {
            if let Ok(mut sprite) = texture_atlas_query.get_mut(entity) {
                sprite.color = hostility.color();
            }
        }
    }
}

fn create_hit_events(
    mut collision_events: EventReader<CollisionEvent>,
    mut hit_events: EventWriter<HitEvent>,
    projectile_query: Query<Entity, With<Projectile>>,
    behavior_query: Query<&ContactBehavior>,
) {
    // technically this actually does nothing but copy data but it's nice to
    // have access to all of this easily
    for ev in collision_events.iter() {
        // only listen to started collisions
        let CollisionEvent::Started(c1, c2, _) = *ev else {
            continue;
        };

        // find projectile
        let (projectile, entity) = if projectile_query.contains(c1) {
            (c1, c2)
        } else if projectile_query.contains(c2) {
            (c2, c1)
        } else {
            continue;
        };

        let projectile_behavior = behavior_query
            .get(projectile)
            .ok()
            .copied()
            .unwrap_or_default();
        let entity_behavior = behavior_query.get(entity).ok().copied().unwrap_or_default();

        hit_events.send(HitEvent {
            projectile,
            entity,
            result: projectile_behavior.and(entity_behavior),
        });
    }
}

fn set_absorb_flag(
    mut hit_events: EventReader<HitEvent>,
    mut projectile_query: Query<&mut Projectile>,
) {
    for ev in hit_events.iter() {
        let Ok(mut projectile) = projectile_query.get_mut(ev.projectile) else {
            continue;
        };

        projectile.absorbed = true;
    }
}

fn despawn_projectiles(
    mut commands: Commands,
    projectile_query: Query<(Entity, &Projectile)>,
    mut despawn_events: EventReader<DespawnEvent>,
) {
    for (entity, proj) in projectile_query.iter() {
        if proj.absorbed {
            commands.entity(entity).despawn_recursive();
        }
    }

    for ev in despawn_events.iter() {
        if let Some(entity) = commands.get_entity(ev.projectile) {
            entity.despawn_recursive();
        }
    }
}
