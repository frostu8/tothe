//! Visual signals.

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;

use super::{InteractionSystem, Signal};

use std::borrow::Cow;
use std::sync::Arc;

use crate::{GameAssets, GameState};

/// Adds visuals to signals.
pub struct VisualSignalPlugin;

impl Plugin for VisualSignalPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Update,
                create_signal_visual
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                update_signal_transform
                    .after(InteractionSystem::TravelSignal),
            );
    }
}

/// Defines how a signal will visually "buldge" while travelling through pipes,
/// as an array of floats, where 1 is normal size and 0 is hidden.
#[derive(Clone, Component, Debug)]
pub struct Buldge {
    incoming: Arc<[f32]>,
    outgoing: Arc<[f32]>,
}

impl Buldge {
    pub fn new<const N: usize>(from: [f32; N]) -> Buldge {
        let mut incoming = from.clone();
        incoming.reverse();

        Buldge {
            outgoing: Arc::new(from),
            incoming: Arc::new(incoming),
        }
    }

    pub fn no_cover() -> Buldge {
        Buldge::new([0., 0., 0., 0.15, 0.5, 0.75, 1., 1.])
    }
}

#[derive(SystemParam)]
pub struct BuldgeQuery<'w, 's> {
    query: Query<'w, 's, &'static Buldge>,
}

impl<'w, 's> BuldgeQuery<'w, 's> {
    /// Finds the size at a certain point on the continum (lineraly
    /// interpolated).
    pub fn at(&self, from: Entity, to: Entity, pos: f32) -> f32 {
        assert!(pos >= 0.);
        assert!(pos < 1.);

        let graph = self.graph(from, to);

        // turn into index
        if graph.len() > 1 {
            let len_1 = graph.len() - 1;

            let index = (pos * len_1 as f32).floor();
            let part = (pos * len_1 as f32) - index;

            let index = index as usize;

            // lerp
            (graph[index] * (1. - part)) + (graph[index + 1] * part)
        } else {
            graph[0]
        }
    }

    /// Gets the buldge graph from a junction to another.
    pub fn graph<'a>(&'a self, from: Entity, to: Entity) -> Cow<'a, [f32]> {
        match (self.query.get(from), self.query.get(to)) {
            (Ok(from), Ok(to)) => {
                // take average
                let res = (0..std::cmp::min(from.outgoing.len(), to.incoming.len()))
                    .map(|i| from.outgoing[i].min(to.incoming[i]))
                    .collect::<Vec<_>>();

                Cow::Owned(res)
            }
            (Ok(from), Err(_)) => {
                // outgoing only
                Cow::Borrowed(&from.outgoing)
            }
            (Err(_), Ok(to)) => {
                // incoming only
                Cow::Borrowed(&to.incoming)
            }
            (Err(_), Err(_)) => Cow::Borrowed(&[1.]),
        }
    }
}

fn create_signal_visual(
    mut commands: Commands,
    new_signal_query: Query<Entity, Added<Signal>>,
    assets: Res<GameAssets>,
) {
    for entity in new_signal_query.iter() {
        // create buldge matte
        commands.spawn(SpriteBundle {
            texture: assets.signal_matte.clone(),
            ..Default::default()
        }).set_parent(entity);
    }
}

fn update_signal_transform(
    transforms: Query<&GlobalTransform>,
    mut signals_query: Query<(&mut Transform, &Signal)>,
    buldges: BuldgeQuery,
    // for testing
    //mut gizmos: Gizmos,
) {
    for (mut transform, signal) in signals_query.iter_mut() {
        let Ok(source) = transforms.get(signal.source) else {
            continue;
        };

        let Some(destination) = signal.destination else {
            continue;
        };

        let Ok(dest) = transforms.get(destination) else {
            continue;
        };

        let start = source.translation().truncate();
        let end = dest.translation().truncate();

        // lerp
        let position = start.lerp(end, signal.position);

        transform.translation = position.extend(30.);

        // find scale
        let scale = buldges.at(signal.source, destination, signal.position.min(0.999999));

        transform.scale = Vec3::splat(scale);

        //gizmos.circle(transform.translation, Vec3::Z, scale * 4., Color::BLUE);
    }
}

