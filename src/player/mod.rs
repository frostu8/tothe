//! Player things.

pub mod controller;
pub mod respawn;

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use std::time::Duration;

use crate::{
    physics::{self, Grounded},
    projectile::spawner::{Charge, Spawner},
    enemy::Hostility,
    GameAssets, GameState,
};
use controller::{ControllerBundle, ControllerOptions, CoyoteJump, UseGamepad};
use respawn::{Respawn, RespawnSystem, WorldRespawn};

/// A player plugin.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player)
            .add_systems(
                Update,
                detect_player_death
                    .after(RespawnSystem::Respawn),
            );
    }
}

/// A marker component for the local player.
///
/// Only one can exist at a time. It is invalid if more than one local player
/// exists, but it is valid for no players to exist.
#[derive(Clone, Component, Default, Debug)]
pub struct LocalPlayer;

/// A startup system that spawns a default player in.
fn spawn_player(mut commands: Commands, assets: Res<GameAssets>) {
    commands
        .spawn((
            SpatialBundle {
                visibility: Visibility::Hidden,
                ..Default::default()
            },
            RigidBody::Fixed,
            LockedAxes::ROTATION_LOCKED,
            LocalPlayer::default(),
            Collider::round_cuboid(3., 3., 0.125),
            Velocity::default(),
            CollisionGroups::new(physics::COLLISION_GROUP_FRIENDLY, Group::all()),
            Grounded::default(),
            CoyoteJump::default(),
            UseGamepad::default(),
            Spawner::default(),
            Charge::new(Duration::from_millis(800), 1).as_full(),
            Friction {
                coefficient: 0.,
                combine_rule: CoefficientCombineRule::Multiply,
            },
            ControllerBundle {
                options: ControllerOptions {
                    enabled: false,
                    max_speed: 64. * 1.5,
                    deadzone: 0.3,
                    friction: 4.,
                    jump_buffer: Duration::from_millis(100),
                    jump_height: 52.,
                    projectile_speed: 256.,
                },
                ..Default::default()
            },
            Respawn::default(),
        ))
        .insert((
            Hostility::Friendly,
            ActiveEvents::COLLISION_EVENTS,
        ))
        .with_children(|parent| {
            parent.spawn((SpriteSheetBundle {
                texture_atlas: assets.player_sheet.clone(),
                sprite: TextureAtlasSprite::new(0),
                transform: Transform::from_xyz(0., 4., 0.),
                ..Default::default()
            },));
        });
}

fn detect_player_death(
    mut collision_events: EventReader<CollisionEvent>,
    mut player_query: Query<(&mut Visibility, &mut ControllerOptions), With<LocalPlayer>>,
    mut world_respawn: ResMut<WorldRespawn>,
    subject_query: Query<&Hostility>,
) {
    for ev in collision_events.iter() {
        let CollisionEvent::Started(c1, c2, _) = ev else {
            continue;
        };

        // find player
        let ((mut player_visibility, mut controller), subject) = {
            if let Ok(player) = player_query.get_mut(*c1) {
                (player, *c2)
            } else if let Ok(player) = player_query.get_mut(*c2) {
                (player, *c1)
            } else {
                continue;
            }
        };

        // find subject
        let Ok(subject_hostility) = subject_query.get(subject) else {
            continue;
        };

        if *subject_hostility == Hostility::Hostile {
            // kill player
            *player_visibility = Visibility::Hidden;
            controller.enabled = false;
            world_respawn.start_respawn();
        }
    }
}

