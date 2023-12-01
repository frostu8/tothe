//! Particle residual effects from projectiles.

use bevy::prelude::*;

use std::ops::Range;
use std::time::Duration;

use crate::{GameAssets, GameState};
use crate::enemy::Hostility;
use super::{ContactBehavior, HitEvent};

/// Residue effects.
pub struct ResiduePlugin;

impl Plugin for ResiduePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, update_residue)
            .add_systems(Update, create_residue.run_if(in_state(GameState::InGame)));
    }
}

/// A residue.
///
/// After playing the animation (defined by a range in the texture atlas), it
/// will despawn.
#[derive(Clone, Component, Debug, Default)]
pub struct Residue {
    /// The range of animation frames.
    pub animation_range: Range<usize>,
    /// The duration of each frame.
    pub timer: Timer,
}

impl Residue {
    pub fn new(range: Range<usize>, duration: Duration) -> Residue {
        Residue {
            animation_range: range,
            timer: Timer::new(duration, TimerMode::Once),
        }
    }
}

fn update_residue(
    mut commands: Commands,
    mut residue_query: Query<(Entity, &mut Residue, &mut TextureAtlasSprite)>,
    time: Res<Time>,
) {
    for (entity, mut residue, mut sprite) in residue_query.iter_mut() {
        // tick
        residue.timer.tick(time.delta());

        if residue.timer.finished() {
            residue.animation_range.start += 1;

            if residue.animation_range.start == residue.animation_range.end {
                commands.entity(entity).despawn_recursive();
            } else {
                residue.timer.reset();
            }
        }

        sprite.index = residue.animation_range.start;
    }
}

fn create_residue(
    mut commands: Commands,
    subject_query: Query<(&GlobalTransform, &Hostility)>,
    mut hit_events: EventReader<HitEvent>,
    assets: Res<GameAssets>,
) {
    for ev in hit_events.iter() {
        if matches!(ev.result, ContactBehavior::Absorb) {
            // create residue at location
            let Ok((location, hostility)) = subject_query.get(ev.projectile) else {
                continue;
            };

            commands.spawn((
                SpriteSheetBundle {
                    texture_atlas: assets.projectile_sheet.clone(),
                    sprite: TextureAtlasSprite {
                        color: hostility.color(),
                        ..TextureAtlasSprite::new(18)
                    },
                    transform: Transform::from_translation(location.translation()),
                    ..Default::default()
                },
                Residue::new(18..20, Duration::from_millis(100)),
            ));
        }
    }
}

