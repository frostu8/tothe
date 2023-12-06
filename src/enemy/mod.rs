//! Enemy things.

pub mod prefab;

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use crate::level::Iid;
use crate::physics;
use crate::platform::ActivateEvent;
use crate::projectile::{HitEvent, Projectile, ProjectileSystem};

use std::time::Duration;

/// Enemy plugin.
pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, upgrade_activate_on_death)
            .add_systems(
                Update,
                despawn_dead_enemies.before(EnemySystem::RegisterHits),
            )
            .add_systems(
                Update,
                check_for_enemy_hits
                    .in_set(EnemySystem::RegisterHits)
                    .before(ProjectileSystem::Despawn)
                    .after(ProjectileSystem::Bounce)
                    .after(ProjectileSystem::Event),
            )
            .add_systems(Update, tint_dying_enemies.after(EnemySystem::RegisterHits));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum EnemySystem {
    /// Register hits on enemies.
    RegisterHits,
    /// Despawns dead enemies.
    Despawn,
}

/// Enemy prefab bundle.
#[derive(Bundle)]
pub struct EnemyBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
    pub collider: Collider,
    pub collision_groups: CollisionGroups,
    pub enemy: Enemy,
}

impl Default for EnemyBundle {
    fn default() -> EnemyBundle {
        EnemyBundle {
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            computed_visibility: ComputedVisibility::default(),
            collider: Collider::default(),
            collision_groups: CollisionGroups::new(physics::COLLISION_GROUP_HOSTILE, Group::all()),
            enemy: Enemy::default(),
        }
    }
}

/// A marker component for enemies.
#[derive(Clone, Component, Debug, Default)]
pub struct Enemy {
    /// Registers projectile hits but doesn't actually die.
    pub invincible: bool,
}

impl Enemy {
    /// Creates a new [invincible] enemy.
    ///
    /// [invincible]: Enemy::invincible
    pub fn invincible() -> Enemy {
        Enemy { invincible: true }
    }
}

/// Sends an [`ActivateEvent`] on death.
#[derive(Clone, Component, Debug, Default)]
pub struct ActivateOnDeath(Option<Entity>);

/// Slightly indirect version of [`ActivateOnDeath`].
#[derive(Clone, Component, Debug, Default)]
pub struct ActivateOnDeathByIid(Option<String>);

/// A timer for an enemy to [die](https://youtu.be/h3k5EAN97wE).
#[derive(Clone, Component, Debug)]
pub struct DeathTimer(Timer);

impl Default for DeathTimer {
    fn default() -> DeathTimer {
        DeathTimer(Timer::new(Duration::from_millis(100), TimerMode::Once))
    }
}

/// Deterines if something is an enemy or a friendly (the player).
#[derive(Clone, Copy, Component, Debug, Default, PartialEq, Eq, Hash)]
pub enum Hostility {
    #[default]
    Friendly,
    Hostile,
}

impl Hostility {
    /// Returns the collision groups appropriate for a projectile of this
    /// hostility.
    pub fn collision_groups_projectile(self) -> CollisionGroups {
        match self {
            Hostility::Friendly => CollisionGroups::new(
                physics::COLLISION_GROUP_PROJECTILE,
                physics::COLLISION_GROUP_SOLID | physics::COLLISION_GROUP_HOSTILE,
            ),
            Hostility::Hostile => CollisionGroups::new(
                physics::COLLISION_GROUP_PROJECTILE,
                physics::COLLISION_GROUP_SOLID | physics::COLLISION_GROUP_FRIENDLY,
            ),
        }
    }

    /// Returns the associated color of the `Hostility`.
    pub const fn color(self) -> Color {
        match self {
            Hostility::Friendly => Color::rgb(0.37254, 0.80392, 0.89411),
            Hostility::Hostile => Color::rgb(0.96470, 0.15686, 0.15686),
        }
    }
}

fn upgrade_activate_on_death(
    mut commands: Commands,
    query: Query<(Entity, &ActivateOnDeathByIid)>,
    iid_query: Query<(Entity, &Iid)>,
) {
    for (entity, iid_request) in query.iter() {
        let Some(iid_request) = &iid_request.0 else {
            continue;
        };

        // find iid
        let mut found_iid: Option<Entity> = None;

        for (iid_entity, iid) in iid_query.iter() {
            if iid.0 == *iid_request {
                found_iid = Some(iid_entity);
                break;
            }
        }

        if let Some(found_entity) = found_iid {
            commands
                .entity(entity)
                .insert(ActivateOnDeath(Some(found_entity)));
            commands.entity(entity).remove::<ActivateOnDeathByIid>();
        }
    }
}

fn check_for_enemy_hits(
    mut commands: Commands,
    mut projectile_hit_events: EventReader<HitEvent>,
    mut projectile_query: Query<&mut Projectile>,
    enemies_query: Query<(Entity, &Enemy), Without<DeathTimer>>,
) {
    for ev in projectile_hit_events.iter() {
        let Ok((enemy_entity, enemy)) = enemies_query.get(ev.entity) else {
            continue;
        };

        // despawn projectile
        if let Ok(mut projectile) = projectile_query.get_mut(ev.projectile) {
            projectile.absorbed = true;
        }

        if !enemy.invincible {
            commands.entity(enemy_entity).insert(DeathTimer::default());
        }
    }
}

fn despawn_dead_enemies(
    mut commands: Commands,
    mut enemies_query: Query<(Entity, &mut DeathTimer, Option<&ActivateOnDeath>)>,
    mut activate_events: EventWriter<ActivateEvent>,
    time: Res<Time>,
) {
    for (entity, mut death_timer, activate) in enemies_query.iter_mut() {
        death_timer.0.tick(time.delta());

        if death_timer.0.finished() {
            commands.entity(entity).despawn_recursive();

            if let Some(activate) = activate.and_then(|a| a.0) {
                activate_events.send(ActivateEvent(activate));
            }
        }
    }
}

fn tint_dying_enemies(mut enemies_query: Query<&mut TextureAtlasSprite, Added<DeathTimer>>) {
    for mut sprite in enemies_query.iter_mut() {
        sprite.color = Color::WHITE * 255.;
    }
}
