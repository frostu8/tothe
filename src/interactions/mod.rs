//! How nodes can communicate with each other.

pub mod acceptor;

use bevy::app::PluginGroupBuilder;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::enemy::Hostility;

use std::sync::Arc;
use std::borrow::Cow;

/// All interaction plugins.
pub struct InteractionPlugins;

impl PluginGroup for InteractionPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(PipePlugin)
            .add(acceptor::AcceptorPlugin)
    }
}

pub enum InteractionSystem {
    /// Systems that process signals.
    ReceiveSignal,
}

/// A single instance of a signal.
#[derive(Clone, Component, Debug)]
pub struct Signal {
    /// The data of the signal.
    pub data: SignalData,
    /// The source that this signal is travelling from.
    pub source: Entity,
    /// The destination of the signal.
    pub destination: Option<Entity>,
    /// The position it is between signals. A number between 0 and 1.
    pub position: f32,
    /// How far this signal will go in a single second.
    pub speed: f32,
}

impl Signal {
    /// Creates a fresh signal starting from a junction.
    pub fn at(data: SignalData, source: Entity) -> Signal {
        Signal {
            data,
            source,
            destination: None,
            position: 0.,
            speed: 0.,
        }
    }
}

/// The data contained in a signal.
#[derive(Clone, Debug)]
pub struct SignalData {
    /// The hostility of the signal.
    ///
    /// Affects how projectiles on the other end are produced.
    pub hostility: Hostility,
}

/// An event that is fired when a signal moves from an entity.
///
/// This is the main method that interaction entities communicate between each
/// other.
#[derive(Debug, Event)]
pub struct SignalEvent {
    /// The sender.
    pub sender: Entity,
    /// The receiver in question.
    pub receiver: Entity,
    /// The signal the receiver received.
    ///
    /// Has a [`Signal`] component that can be queried.
    pub signal: Entity,
    /// Overfill position.
    pub overfill: f32,
}

/// Pipe plugin.
pub struct PipePlugin;

impl Plugin for PipePlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<Junction>()
            .add_event::<SignalEvent>()
            .add_systems(PreUpdate, handle_signal_events)
            .add_systems(
                Update,
                (signal_travel, update_signal_transform)
                    .chain(),
            )
            .add_systems(Update, debug_draw_pipes);
    }
}

/// Indicates a span in the real world that a signal must travel in real time.
///
/// Has circular connections; e.g. an entity that is connected to this pipe may
/// be another `Junction` and it will have a reference back to this entity.
#[derive(Clone, Component, Debug, Default, Reflect)]
pub struct Junction {
    /// The paths the pipe can take from the origin of this entity.
    pub pipes: Vec<Pipe>,
}

impl Junction {
    /// Clears all the current pipes.
    pub fn clear(&mut self) {
        self.pipes.clear();
    }

    /// Adds a new entity as a default [`Pipe`].
    pub fn push_pipe(&mut self, receiver: Entity) {
        self.pipes.push(Pipe::new(receiver))
    }
}

/// A single pipe.
#[derive(Clone, Debug, Reflect)]
pub struct Pipe {
    /// The entity at the other end of the pipe.
    pub receiver: Entity,
}

impl Pipe {
    /// Creates a new pipe with [`Pipe::size`] initialized to all ones.
    pub fn new(receiver: Entity) -> Pipe {
        Pipe {
            receiver,
        }
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
            (graph[index] * (1. - part)) + (graph[index+1] * part)
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
            (Err(_), Err(_)) => {
                Cow::Borrowed(&[1.])
            }
        }
    }
}

fn handle_signal_events(
    mut commands: Commands,
    mut signal_events: EventReader<SignalEvent>,
    mut signal_query: Query<&mut Signal>,
    junction_query: Query<&Junction>,
) {
    for ev in signal_events.iter() {
        let Ok(mut signal) = signal_query.get_mut(ev.signal) else {
            continue;
        };

        let Ok(junction) = junction_query.get(ev.receiver) else {
            continue;
        };

        // move signal and maybe duplicate
        let mut outputs = junction
            .pipes
            .iter()
            .filter(|pipe| pipe.receiver != ev.sender);

        // move signal to first output
        if let Some(output) = outputs.next() {
            signal.source = ev.receiver;
            signal.destination = Some(output.receiver);
            signal.speed = 12.; // TODO
            signal.position = ev.overfill;
        } else {
            // destroy signal
            commands.entity(ev.signal).despawn_recursive();
            continue;
        }

        // create other signals
        for output in outputs {
            commands.spawn((
                TransformBundle::default(),
                Signal {
                    data: signal.data.clone(),
                    source: ev.receiver,
                    destination: Some(output.receiver),
                    position: ev.overfill,
                    speed: 12., // TODO
                },
            ));
        }
    }
}

fn signal_travel(
    mut signals_query: Query<(Entity, &mut Signal)>,
    mut signal_events: EventWriter<SignalEvent>,
    time: Res<Time>,
) {
    for (signal_entity, mut signal) in signals_query.iter_mut() {
        if let Some(dest) = signal.destination {
            // move signal forward
            signal.position += signal.speed * time.delta_seconds();

            if signal.position >= 1. {
                // send signal event
                signal_events.send(SignalEvent {
                    sender: signal.source,
                    receiver: dest,
                    signal: signal_entity,
                    overfill: signal.position - 1.,
                });
            }
        }
    }
}

fn update_signal_transform(
    transforms: Query<&GlobalTransform>,
    mut signals_query: Query<(&mut Transform, &Signal)>,
    buldges: BuldgeQuery,
    // for testing
    mut gizmos: Gizmos,
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

        transform.translation = position.extend(transform.translation.z);

        // find scale
        let scale = buldges.at(signal.source, destination, signal.position.min(0.999999));

        transform.scale = Vec3::splat(scale);

        gizmos.circle(
            transform.translation,
            Vec3::Z,
            scale * 4.,
            Color::BLUE,
        );
    }
}

fn debug_draw_pipes(
    junction_query: Query<(Entity, &Junction)>,
    transform_query: Query<&GlobalTransform>,
    buldges: BuldgeQuery,
    mut gizmos: Gizmos,
) {
    use std::collections::HashSet;

    let mut visited: HashSet<Entity> = HashSet::new();

    for (entity, junction) in junction_query.iter() {
        let Ok(start) = transform_query.get(entity) else {
            continue;
        };

        for pipe in &junction.pipes {
            if !visited.contains(&pipe.receiver) {
                let Ok(end) = transform_query.get(pipe.receiver) else {
                    continue;
                };

                // draw connection
                let start = start.translation().truncate();
                let end = end.translation().truncate();

                let difference = end - start;

                let strength = buldges.graph(entity, pipe.receiver);

                for i in 0..strength.len() {
                    let size = strength[i];
                    let color = Color::rgb(size, 0., 1. - size);

                    let percent = i as f32 / strength.len() as f32;

                    let start = difference * percent + start;
                    let end = difference * (1. / strength.len() as f32) + start;

                    gizmos.line_2d(start, end, color);
                }
            }
        }

        // add to visited
        visited.insert(entity);
    }
}

