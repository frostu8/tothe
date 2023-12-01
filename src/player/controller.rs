//! Player physics controller.

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use crate::physics::{Grounded, PhysicsSet};
use crate::projectile::ProjectileBundle;
use crate::{GameAssets, GameState};

use std::time::Duration;

/// The controller plugin.
pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_coyote_jump_timer,
                scan_input,
                apply_movement,
                clear_controller_jump,
            )
                .chain()
                .after(PhysicsSet::CheckGrounded),
        )
        .add_systems(
            Update,
            create_projectiles.run_if(in_state(GameState::InGame)),
        );
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
    /// The jump buffer time.
    pub jump_buffer: Duration,
    /// The jump height of the player in world units.
    pub jump_height: f32,
}

impl ControllerOptions {
    pub fn initial_jump_velocity(&self, gravity: f32) -> f32 {
        (2. * gravity.abs() * self.jump_height).sqrt()
    }
}

/// A component that translates player input into physics movement.
#[derive(Component, Default)]
pub struct Controller {
    x_movement: f32,
    jump: bool,
    jump_buffer: Timer,
    shoot: bool,
}

impl Controller {
    /// Sets the jump timer.
    ///
    /// This happens when the user presses a button to jump. Instead of a dumb
    /// bool, we use a small buffer technique.
    pub fn set_jump(&mut self, buffer: Duration) {
        self.jump = true;
        self.jump_buffer = Timer::new(buffer, TimerMode::Once);
    }

    /// Checks if there is a buffered jump.
    pub fn buffered_jump(&self) -> bool {
        !self.jump_buffer.finished()
    }
}

/// A component for coyote jumping.
#[derive(Component)]
pub struct CoyoteJump {
    timer: Timer,
    locked: bool,
}

impl CoyoteJump {
    /// Creates a new `CoyoteJump`.
    pub fn new(duration: Duration) -> CoyoteJump {
        CoyoteJump {
            timer: Timer::new(duration, TimerMode::Once),
            locked: false,
        }
    }

    /// Locks the coyote jump until the player is grounded.
    pub fn lock(&mut self) {
        self.locked = true;
    }

    /// Locks the coyote jump.
    pub fn unlock(&mut self) {
        self.locked = false;
    }

    /// Checks if the player can perform a "coyote jump."
    pub fn can_jump(&self) -> bool {
        !self.timer.finished() && !self.locked
    }

    fn tick(&mut self, delta: Duration) {
        self.timer.tick(delta);
    }

    fn reset(&mut self) {
        self.timer.reset();
    }
}

impl Default for CoyoteJump {
    fn default() -> CoyoteJump {
        CoyoteJump::new(Duration::from_millis(100))
    }
}

fn tick_coyote_jump_timer(
    mut coyote_timer_query: Query<(&mut CoyoteJump, Ref<Grounded>)>,
    time: Res<Time>,
) {
    for (mut timer, grounded) in coyote_timer_query.iter_mut() {
        timer.tick(time.delta());

        if grounded.is_changed() {
            if grounded.grounded {
                timer.unlock();
            } else {
                // start timer
                timer.reset();
            }
        }
    }
}

fn scan_input(
    mut query: Query<(&mut Controller, &ControllerOptions)>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
) {
    for (mut controller, options) in query.iter_mut() {
        // reset movement
        controller.x_movement = 0.0;

        // sample keyboard
        if keyboard.pressed(KeyCode::A) {
            controller.x_movement -= 1.0;
        } else if keyboard.pressed(KeyCode::D) {
            controller.x_movement += 1.0;
        }

        if keyboard.just_pressed(KeyCode::Space) {
            controller.set_jump(options.jump_buffer)
        }

        controller.shoot = mouse.just_pressed(MouseButton::Left);
    }
}

fn clear_controller_jump(mut query: Query<&mut Controller>, time: Res<Time>) {
    for mut controller in query.iter_mut() {
        controller.jump_buffer.tick(time.delta());
        controller.jump = false;
    }
}

fn apply_movement(
    mut query: Query<(
        &Controller,
        &ControllerOptions,
        &Grounded,
        &mut CoyoteJump,
        &mut Velocity,
    )>,
    physics_options: Res<RapierConfiguration>,
) {
    for (controller, options, grounded, mut coyote_jump, mut velocity) in query.iter_mut() {
        let ControllerOptions {
            max_speed,
            friction,
            ..
        } = *options;

        move_toward(
            &mut velocity.linvel.x,
            controller.x_movement * max_speed,
            friction,
        );

        let jump = (controller.jump && coyote_jump.can_jump())
            || (controller.buffered_jump() && grounded.grounded);

        // apply jump
        if jump {
            coyote_jump.lock();
            velocity.linvel.y = options.initial_jump_velocity(physics_options.gravity.y);
        }
    }
}

fn create_projectiles(
    mut commands: Commands,
    player_query: Query<(&GlobalTransform, &Controller)>,
    assets: Res<GameAssets>,
) {
    for (transform, controller) in player_query.iter() {
        if controller.shoot {
            // produce a bullet
            commands.spawn((
                ProjectileBundle {
                    transform: Transform::from_translation(transform.translation()),
                    gravity_scale: GravityScale(0.),
                    ..Default::default()
                },
                Velocity {
                    linvel: Vec2::new(256., 0.),
                    angvel: 0.,
                },
                assets.projectile_sheet.clone(),
                TextureAtlasSprite::new(0),
                VisibilityBundle::default(),
            ));
        }
    }
}

fn move_toward(current: &mut f32, target: f32, max_movement: f32) {
    let difference = target - *current;
    let movement = difference.abs().min(max_movement);
    *current += movement.copysign(difference);
}
