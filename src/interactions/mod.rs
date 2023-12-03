//! How nodes can communicate with each other.

pub mod acceptor;
pub mod pipe;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

use crate::enemy::Hostility;

/// All interaction plugins.
pub struct InteractionPlugins;

impl PluginGroup for InteractionPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(BaseInteractionsPlugin)
            .add(pipe::PipePlugin)
            .add(acceptor::AcceptorPlugin)
    }
}

/// Base interactions plugin.
pub struct BaseInteractionsPlugin;

impl Plugin for BaseInteractionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SignalEvent>();
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
