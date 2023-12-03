//! Projectile spawners.

use bevy::prelude::*;

use std::time::Duration;

use super::prefab::{CreateProjectile, ProjectilePrefab};
use crate::GameState;

pub struct ProjectileSpawnerPlugin;

impl Plugin for ProjectileSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnProjectile>()
            .add_systems(Update, update_charge.in_set(SpawnerSystem::TickTimer))
            .add_systems(
                Update,
                spawn_projectile
                    .run_if(in_state(GameState::InGame))
                    .in_set(SpawnerSystem::Spawn)
                    .after(SpawnerSystem::TickTimer),
            );
    }
}

/// Spawner systems.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum SpawnerSystem {
    TickTimer,
    /// Spawner spawns projectiles.
    ///
    /// [`SpawnProjectile`] must be sent before this system.
    Spawn,
}

/// Spawns a projectile.
#[derive(Debug, Event)]
pub struct SpawnProjectile {
    subject: Entity,
}

impl SpawnProjectile {
    /// Creates a new `SpawnProjectile` event.
    pub fn new(subject: Entity) -> SpawnProjectile {
        SpawnProjectile { subject }
    }
}

/// A spawner for projectiles.
#[derive(Clone, Component, Debug)]
pub struct Spawner {
    /// The initial velocity of the projectile.
    pub initial_velocity: Vec2,
}

impl Default for Spawner {
    fn default() -> Spawner {
        Spawner {
            initial_velocity: Vec2::new(0., 0.),
        }
    }
}

/// A charge for a spawner.
#[derive(Clone, Component, Debug)]
pub struct Charge {
    timer: Timer,
    charges: u32,
    max_charges: u32,
}

impl Charge {
    /// Creates a new `Charge`.
    pub fn new(duration: Duration, max_charges: u32) -> Charge {
        Charge {
            timer: Timer::new(duration, TimerMode::Repeating),
            charges: 0,
            max_charges,
        }
    }

    /// Fills the `Charge`.
    pub fn as_full(mut self) -> Charge {
        self.charges = self.max_charges;
        self.timer.pause();
        self
    }

    /// Takes a charge.
    pub fn use_charge(&mut self) {
        self.charges -= 1;
        self.timer.unpause();
    }

    /// Checks if the spawner has a charge.
    pub fn has_charge(&self) -> bool {
        self.charges > 0
    }

    /// Ticks the `Charge`.
    pub fn tick(&mut self, delta: Duration) {
        self.timer.tick(delta);
        self.charges += self.timer.times_finished_this_tick();

        if self.charges >= self.max_charges {
            self.charges = self.max_charges;
            self.timer.pause();
        }
    }
}

impl Default for Charge {
    fn default() -> Charge {
        Charge::new(Duration::from_secs(1), 1)
    }
}

fn update_charge(mut charge_query: Query<&mut Charge>, time: Res<Time>) {
    charge_query.for_each_mut(|mut c| c.tick(time.delta()))
}

fn spawn_projectile(
    mut commands: Commands,
    mut projectile_spawns: EventReader<SpawnProjectile>,
    mut spawner_query: Query<(&GlobalTransform, &Spawner, Option<&mut Charge>)>,
) {
    for ev in projectile_spawns.iter() {
        let Ok((transform, spawner, charge)) = spawner_query.get_mut(ev.subject) else {
            bevy::log::warn!("spawn event for entity without spawner");
            continue;
        };

        let spawn = match charge {
            Some(mut charge) if charge.has_charge() => {
                charge.use_charge();
                true
            }
            Some(_) => false,
            None => true,
        };

        if spawn {
            commands.add(CreateProjectile::new(
                ProjectilePrefab::QuarterRest {
                    initial_velocity: spawner.initial_velocity,
                },
                transform.translation(),
            ));
        }
    }
}
