//! Projectile things.

pub mod residue;

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use crate::enemy::Hostility;
use crate::physics;

/// Projectile plugin.
pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HitEvent>()
            .add_systems(PreUpdate, create_hit_events)
            .add_systems(Update, despawn_absorbed_projectiles)
            .add_systems(PostUpdate, (update_collision_groups, update_sprite_color));
    }
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
            gravity_scale: GravityScale::default(),
            projectile: Projectile::default(),
            contact_behavior: ContactBehavior::Absorb,
            hostility: Hostility::Friendly,
        }
    }
}

/// A single projectile.
#[derive(Clone, Component, Debug, Default)]
pub struct Projectile;

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
    mut texture_atlas_query: Query<
        (&Hostility, &mut TextureAtlasSprite),
        (With<Projectile>, Changed<Hostility>),
    >,
    mut sprite_query: Query<(&Hostility, &mut Sprite), (With<Projectile>, Changed<Hostility>)>,
) {
    for (hostility, mut sprite) in texture_atlas_query.iter_mut() {
        sprite.color = hostility.color();
    }

    for (hostility, mut sprite) in sprite_query.iter_mut() {
        sprite.color = hostility.color();
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

fn despawn_absorbed_projectiles(
    mut commands: Commands,
    mut hit_events: EventReader<HitEvent>,
) {
    for ev in hit_events.iter() {
        if matches!(ev.result, ContactBehavior::Absorb) {
            commands.entity(ev.projectile).despawn_recursive();
        }
    }
}
