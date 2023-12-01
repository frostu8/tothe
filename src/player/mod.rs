//! Player things.

pub mod controller;
pub mod respawn;

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use std::time::Duration;

use crate::{
    physics::{self, Grounded},
    GameAssets, GameState,
};
use controller::{ControllerBundle, ControllerOptions, CoyoteJump};
use respawn::Respawn;

/// A player plugin.
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), spawn_player);
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
            RigidBody::Dynamic,
            LockedAxes::ROTATION_LOCKED,
            LocalPlayer::default(),
            Collider::round_cuboid(3., 3., 0.125),
            //Collider::cuboid(4., 4.),
            Velocity::default(),
            //ActiveEvents::COLLISION_EVENTS,
            CollisionGroups::new(physics::COLLISION_GROUP_FRIENDLY, Group::all()),
            Grounded::default(),
            CoyoteJump::default(),
            Friction {
                coefficient: 0.,
                combine_rule: CoefficientCombineRule::Multiply,
            },
            ControllerBundle {
                options: ControllerOptions {
                    max_speed: 64. * 1.5,
                    friction: 4.,
                    jump_buffer: Duration::from_millis(100),
                    jump_height: 52.,
                },
                ..Default::default()
            },
            Respawn::default(),
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
