//! Enemy things.

use bevy::prelude::*;

/// Deterines if something is an enemy or a friendly (the player).
#[derive(Clone, Copy, Component, Debug, Default, PartialEq, Eq, Hash)]
pub enum Hostility {
    #[default]
    Friendly,
    Hostile,
}

impl Hostility {
    /// Returns the associated color of the `Hostility`.
    pub const fn color(self) -> Color {
        match self {
            Hostility::Friendly => Color::rgb(0.37254, 0.80392, 0.89411),
            Hostility::Hostile => Color::rgb(0.96470, 0.15686, 0.15686),
        }
    }
}
