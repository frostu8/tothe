//! Player physics controller.

use bevy::input::gamepad::{GamepadConnection, GamepadConnectionEvent};
use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use crate::camera::{cursor::CursorWorldPosition, PlayerCamera};
use crate::physics::{Grounded, PhysicsSet};
use crate::projectile::spawner::{SpawnProjectile, Spawner, SpawnerSystem};

use std::time::Duration;

/// The controller plugin.
pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            enable_physics_for_controller,
        )
        .add_systems(
            Update,
            tick_coyote_jump_timer.before(ControllerSystem::Apply),
        )
        .add_systems(
            Update,
            detect_gamepad.in_set(ControllerSystem::DetectGamepad),
        )
        .add_systems(
            Update,
            (clear_controller, scan_input)
                .chain()
                .in_set(ControllerSystem::ScanInput),
        )
        .add_systems(
            Update,
            (apply_projectiles, apply_movement)
                .chain()
                .in_set(ControllerSystem::Apply)
                .after(ControllerSystem::ScanInput)
                .after(PhysicsSet::CheckGrounded)
                .before(SpawnerSystem::Spawn),
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum ControllerSystem {
    DetectGamepad,
    ScanInput,
    Apply,
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
    /// Whether the controller is enabled.
    pub enabled: bool,
    /// The max speed of the player.
    pub max_speed: f32,
    /// The deadzone of the player movement; prevents players from inching
    /// forward.
    pub deadzone: f32,
    /// The friction of the player.
    pub friction: f32,
    /// The jump buffer time.
    pub jump_buffer: Duration,
    /// The jump height of the player in world units.
    pub jump_height: f32,
    /// The speed of the bullets the player produces in world units per second.
    pub projectile_speed: f32,
}

impl ControllerOptions {
    pub fn initial_jump_velocity(&self, gravity: f32) -> f32 {
        (2. * gravity.abs() * self.jump_height).sqrt()
    }
}

/// A componet for gamepad control.
#[derive(Component, Default)]
pub struct UseGamepad(Option<Gamepad>);

impl UseGamepad {
    pub fn has_gamepad(&self) -> bool {
        self.0.is_some()
    }
}

/// A component that translates player input into physics movement.
#[derive(Component)]
pub struct Controller {
    x_movement: f32,
    jump: bool,
    jump_buffer: Timer,
    shoot: bool,
    shoot_dir: Vec2,
}

impl Controller {
    /// Gets the direction the player is pointing.
    pub fn shoot_dir(&self) -> Vec2 {
        self.shoot_dir
    }

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

impl Default for Controller {
    fn default() -> Controller {
        Controller {
            x_movement: 0.,
            jump: false,
            jump_buffer: Timer::default(),
            shoot: false,
            shoot_dir: Vec2::X,
        }
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

fn enable_physics_for_controller(
    mut controller_query: Query<(&ControllerOptions, &mut RigidBody), Changed<Controller>>,
) {
    for (controller, mut rigidbody) in controller_query.iter_mut() {
        if controller.enabled {
            *rigidbody = RigidBody::Dynamic;
        } else {
            *rigidbody = RigidBody::Fixed;
        }
    }
}

fn tick_coyote_jump_timer(
    mut coyote_timer_query: Query<(&mut CoyoteJump, Ref<Grounded>)>,
    time: Res<Time>,
) {
    for (mut timer, grounded) in coyote_timer_query.iter_mut() {
        timer.tick(time.delta());

        if grounded.is_changed() {
            if grounded.is_grounded() {
                timer.unlock();
            } else {
                // start timer
                timer.reset();
            }
        }
    }
}

fn detect_gamepad(
    mut use_gamepad_query: Query<(DebugName, &mut UseGamepad)>,
    mut gamepad_connected_events: EventReader<GamepadConnectionEvent>,
) {
    for ev in gamepad_connected_events.iter() {
        match &ev.connection {
            GamepadConnection::Connected(_) => {
                for (name, mut use_gamepad) in use_gamepad_query.iter_mut() {
                    if use_gamepad.0.is_none() {
                        // add gamepad
                        use_gamepad.0 = Some(ev.gamepad);
                        bevy::log::info!("connected gamepad {:?} to player {:?}", ev.gamepad, name);
                    }
                }
            }
            GamepadConnection::Disconnected => {
                // remove gameapd
                for (name, mut use_gamepad) in use_gamepad_query.iter_mut() {
                    if use_gamepad.0 == Some(ev.gamepad) {
                        // add gamepad
                        use_gamepad.0 = None;
                        bevy::log::info!("connected gamepad from player {:?}", name);
                    }
                }
            }
        }
    }
}

fn scan_input(
    mut query: Query<(
        &GlobalTransform,
        &mut Controller,
        &ControllerOptions,
        Option<&UseGamepad>,
    )>,
    cursor_query: Query<&CursorWorldPosition, With<PlayerCamera>>,
    gamepad_button: Res<Input<GamepadButton>>,
    gamepad_axis: Res<Axis<GamepadAxis>>,
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
) {
    for (transform, mut controller, options, gamepad) in query.iter_mut() {
        let gamepad = gamepad.and_then(|g| g.0);

        // x movement
        if let Some(gamepad) = gamepad {
            let dir_x = gamepad_axis
                .get(GamepadAxis {
                    gamepad,
                    axis_type: GamepadAxisType::LeftStickX,
                })
                .unwrap_or_else(|| 0.);

            if dir_x.abs() > options.deadzone {
                controller.x_movement = dir_x;
            }
        } else {
            // sample keyboard
            if keyboard.pressed(KeyCode::A) {
                controller.x_movement -= 1.0;
            } else if keyboard.pressed(KeyCode::D) {
                controller.x_movement += 1.0;
            }
        }

        // jump button
        if keyboard.just_pressed(KeyCode::Space) {
            controller.set_jump(options.jump_buffer)
        }

        if let Some(gamepad) = gamepad {
            if gamepad_button.just_pressed(GamepadButton {
                gamepad,
                button_type: GamepadButtonType::South,
            }) {
                controller.set_jump(options.jump_buffer)
            }

            // for pros only
            if gamepad_button.just_pressed(GamepadButton {
                gamepad,
                button_type: GamepadButtonType::LeftTrigger,
            }) {
                controller.set_jump(options.jump_buffer)
            }
        }

        // shoot button
        controller.shoot |= mouse.just_pressed(MouseButton::Left);

        if let Some(gamepad) = gamepad {
            controller.shoot |= gamepad_button.just_pressed(GamepadButton {
                gamepad,
                button_type: GamepadButtonType::RightTrigger,
            });
        }

        // aim
        if let Some(gamepad) = gamepad {
            let dir_x = gamepad_axis.get(GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::RightStickX,
            });
            let dir_y = gamepad_axis.get(GamepadAxis {
                gamepad,
                axis_type: GamepadAxisType::RightStickY,
            });

            if let Some((x, y)) = dir_x.and_then(|x| dir_y.map(|y| (x, y))) {
                let result = Vec2::new(x, y);

                // shoot direction must always have a direction
                if result.length_squared() > 0.1 {
                    controller.shoot_dir = result.normalize();
                }
            }
        } else if let Ok(cursor_pos) = cursor_query.get_single() {
            let rel_pos = cursor_pos.0 - transform.translation().truncate();

            // normalize
            controller.shoot_dir = rel_pos.normalize();
        }
    }
}

fn clear_controller(mut query: Query<&mut Controller>, time: Res<Time>) {
    for mut controller in query.iter_mut() {
        controller.jump_buffer.tick(time.delta());
        controller.jump = false;
        controller.x_movement = 0.0;
        controller.shoot = false;
    }
}

fn apply_projectiles(
    mut query: Query<(Entity, &Controller, &ControllerOptions, &mut Spawner)>,
    mut spawn_projectile: EventWriter<SpawnProjectile>,
) {
    for (entity, controller, options, mut spawner) in query.iter_mut() {
        if !options.enabled {
            continue;
        }

        spawner.initial_velocity = controller.shoot_dir * options.projectile_speed;

        if controller.shoot {
            spawn_projectile.send(SpawnProjectile::new(entity));
        }
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
        if !options.enabled {
            continue;
        }

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
            || (controller.buffered_jump() && grounded.is_grounded());

        // apply jump
        if jump {
            coyote_jump.lock();
            velocity.linvel.y = options.initial_jump_velocity(physics_options.gravity.y);
        }
    }
}

fn move_toward(current: &mut f32, target: f32, max_movement: f32) {
    let difference = target - *current;
    let movement = difference.abs().min(max_movement);
    *current += movement.copysign(difference);
}
