//! Player physics controller.

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

/// The controller plugin.
pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (scan_input, apply_movement).chain());
    }
}

/// A bundle for a player controller.
#[derive(Bundle, Default)]
pub struct ControllerBundle {
    pub options: ControllerOptions,
    pub controller: Controller,
}

/// A config for a [`Controller`].
#[derive(Component, Default)]
pub struct ControllerOptions {
    /// The max speed of the player.
    pub max_speed: f32,
    /// The friction of the player.
    pub friction: f32,
}

/// A component that translates player input into physics movement.
#[derive(Component, Default)]
pub struct Controller {
    x_movement: f32,
}

fn scan_input(
    mut query: Query<&mut Controller>,
    keyboard: Res<Input<KeyCode>>,
) {
    for mut controller in query.iter_mut() {
        // reset movement
        controller.x_movement = 0.0;

        // sample keyboard
        if keyboard.pressed(KeyCode::A) {
            controller.x_movement -= 1.0;
        } else if keyboard.pressed(KeyCode::D) {
            controller.x_movement += 1.0;
        }
    }
}

fn apply_movement(
    mut query: Query<(&Controller, &ControllerOptions, &mut Velocity)>,
) {
    for (controller, options, mut velocity) in query.iter_mut() {
        let ControllerOptions {
            max_speed,
            friction,
        } = *options;

        move_toward(&mut velocity.linvel.x, controller.x_movement * max_speed, friction)
    }
}

fn move_toward(current: &mut f32, target: f32, max_movement: f32) {
    let difference = target - *current;
    let movement = difference.abs().min(max_movement);
    *current += movement.copysign(difference);
}

