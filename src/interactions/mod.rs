//! How nodes can communicate with each other.

pub mod acceptor;
pub mod generator;
pub mod visual;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

use crate::enemy::Hostility;

pub use visual::Buldge;

/// All interaction plugins.
pub struct InteractionPlugins;

impl PluginGroup for InteractionPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(PipePlugin)
            .add(acceptor::AcceptorPlugin)
            .add(generator::GeneratorPlugin)
            .add(visual::VisualSignalPlugin)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum InteractionSystem {
    /// Systems that process signals.
    ReceiveSignal,
    /// Systems that move signals around.
    TravelSignal,
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
        app.register_type::<Junction>()
            .add_event::<SignalEvent>()
            .add_systems(
                PreUpdate,
                handle_signal_events.in_set(InteractionSystem::ReceiveSignal),
            )
            .add_systems(
                Update,
                signal_travel.in_set(InteractionSystem::TravelSignal),
            );
        //.add_systems(Update, debug_draw_pipes);
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
        Pipe { receiver }
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
            signal.speed = 8.; // TODO
            signal.position = ev.overfill;
        } else {
            // destroy signal
            commands.entity(ev.signal).despawn_recursive();
            continue;
        }

        // create other signals
        for output in outputs {
            commands.spawn((
                SpatialBundle::default(),
                Signal {
                    data: signal.data.clone(),
                    source: ev.receiver,
                    destination: Some(output.receiver),
                    position: ev.overfill,
                    speed: 8., // TODO
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

#[allow(dead_code)]
fn debug_draw_pipes(
    junction_query: Query<(Entity, &Junction)>,
    transform_query: Query<&GlobalTransform>,
    buldges: visual::BuldgeQuery,
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
