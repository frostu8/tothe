//! `tothe` general physics stuff.

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;
use bevy_rapier2d::rapier::geometry::ContactPair;

/// Collision for solids and environmental hazards.
pub const COLLISION_GROUP_SOLID: Group = Group::GROUP_1;
/// Collision for friendly entities (most of the time just the player).
pub const COLLISION_GROUP_FRIENDLY: Group = Group::GROUP_2;
/// Collision for hostile units.
pub const COLLISION_GROUP_HOSTILE: Group = Group::GROUP_3;
/// Collision for projectiles.
pub const COLLISION_GROUP_PROJECTILE: Group = Group::GROUP_4;

/// Physics plugin.
pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, check_grounded.in_set(PhysicsSet::CheckGrounded));
    }
}

#[derive(Clone, Debug, SystemSet, Hash, PartialEq, Eq)]
pub enum PhysicsSet {
    /// [`Grounded`] components are updated in this set.
    CheckGrounded,
}

/// A component that tracks whether the entity is grounded or not.
#[derive(Copy, Clone, Component, Debug, Default)]
pub struct Grounded {
    pub grounded: bool,
}

fn check_grounded(mut player_query: Query<(Entity, &mut Grounded)>, physics: Res<RapierContext>) {
    for (player, mut last_grounded) in player_query.iter_mut() {
        let mut grounded = false;

        for contact in physics.contacts_with(player) {
            // do normal check
            grounded |= check_ground_normal(&contact.raw);
        }

        // do not trip change detection
        // hmmm
        if grounded != last_grounded.grounded {
            last_grounded.grounded = grounded;
        }
    }
}

fn check_ground_normal(contact_pair: &ContactPair) -> bool {
    if !contact_pair.has_any_active_contact {
        return false;
    }

    // average normals
    let normal_sum = contact_pair
        .manifolds
        .iter()
        .map(|m| m.local_n2)
        .reduce(|acc, x| acc + x);

    if let Some(normal_sum) = normal_sum {
        let normal = normal_sum / contact_pair.manifolds.len() as f32;

        // find verticality
        let alignment = Vec2::Y.dot(normal.into());

        // since all the floors are perfectly perpendicular, we can get
        // pretty ridiculous with this value
        if alignment > 0.95 {
            return true;
        }
    }

    false
}
