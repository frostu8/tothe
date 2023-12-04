//! Generates projectiles.

use bevy::prelude::*;

use super::{InteractionSystem, Signal, SignalEvent};

use crate::projectile::prefab::{CreateProjectile, ProjectilePrefab};

/// Generator plugin.
pub struct GeneratorPlugin;

impl Plugin for GeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            generate_projectile.after(InteractionSystem::TravelSignal),
        );
    }
}

/// Generates projectiles upon receiving a [`SignalEvent`].
#[derive(Clone, Component, Debug)]
pub struct Generator {
    /// The location to spawn it relative to the generator.
    pub location: Vec3,
    /// The projectile prefab.
    pub prefab: ProjectilePrefab,
}

fn generate_projectile(
    mut commands: Commands,
    generator_query: Query<(&GlobalTransform, &Generator)>,
    mut signal_events: EventReader<SignalEvent>,
    signal_query: Query<&Signal>,
) {
    for ev in signal_events.iter() {
        // do not produce projectiles for accepting
        if ev.sender == ev.receiver {
            continue;
        }

        let Ok((transform, generator)) = generator_query.get(ev.receiver) else {
            continue;
        };

        let Ok(signal) = signal_query.get(ev.signal) else {
            continue;
        };

        let mut location = transform.translation() + generator.location;

        // set so that it appears above the tilemap
        // idk tihs number is really arbitrary
        location.z = 30.;

        // create a new projectile
        commands.add(
            CreateProjectile::new(generator.prefab.clone(), location)
                .hostility(signal.data.hostility.clone()),
        );
    }
}
