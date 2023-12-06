//! UI things.

use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::window::PrimaryWindow;

use crate::camera::{cursor::CursorWorldPosition, PlayerCamera};
use crate::player::{
    controller::{Controller, ControllerSystem, UseGamepad},
    LocalPlayer,
};
use crate::{GameAssets, GameState};

/// Plugin for UI stuff.
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup_ui_elements)
            .add_systems(Update, scale_world_ui)
            .add_systems(
                Update,
                (
                    sync_player_crosshair,
                    sync_beta_crosshair,
                    update_cursor_grab,
                )
                    .after(ControllerSystem::ScanInput),
            );
    }
}

/// Image elements that are scaled so that every pixel on the image is 1 pixel
/// in the world.
#[derive(Clone, Component, Debug, Default)]
pub struct ScaleWorld;

/// The crosshair for the player.
///
/// If the player is in gamepad mode:
/// * This is at a fixed distance from the player.
/// * Follows the right stick axis.
///
/// If the player is in mouse-keyboard move:
/// * This is fixed at the cursor position.
#[derive(Clone, Component, Debug, Default)]
pub struct PlayerCrosshair;

/// Intermediary crosshair that only displays the direction the player is
/// aiming.
#[derive(Clone, Component, Debug)]
pub struct BetaCrosshair(pub f32);

fn setup_ui_elements(mut commands: Commands, assets: Res<GameAssets>) {
    // create crosshair
    commands.spawn((
        ImageBundle {
            style: Style {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                ..Default::default()
            },
            image: UiImage {
                texture: assets.crosshair.clone(),
                flip_x: false,
                flip_y: false,
            },
            ..Default::default()
        },
        PlayerCrosshair,
        ScaleWorld,
    ));

    // create beta crosshairs
    for i in 1..3 {
        commands.spawn((
            ImageBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    display: Display::Flex,
                    ..Default::default()
                },
                image: UiImage {
                    texture: assets.crosshair_beta.clone(),
                    flip_x: false,
                    flip_y: false,
                },
                ..Default::default()
            },
            BetaCrosshair(i as f32 * 16.),
            ScaleWorld,
        ));
    }
}

fn scale_world_ui(
    mut ui_query: Query<(&mut Style, &UiImage), With<ScaleWorld>>,
    camera_query: Query<(&Camera, &OrthographicProjection), With<PlayerCamera>>,
    images: Res<Assets<Image>>,
) {
    let Ok((camera, projection)) = camera_query.get_single() else {
        return;
    };

    let Some(viewport_size) = camera.logical_viewport_size() else {
        return;
    };

    let size = match projection.scaling_mode {
        ScalingMode::FixedVertical(height) => {
            let aspect = viewport_size.x / viewport_size.y;
            Vec2::new(height * aspect, height)
        }
        _ => unimplemented!(),
    };

    for (mut style, ui_image) in ui_query.iter_mut() {
        // get image
        let Some(image) = images.get(&ui_image.texture) else {
            continue;
        };

        let size_pix = image.size() / size * viewport_size;

        style.width = Val::Px(size_pix.x);
        style.height = Val::Px(size_pix.y);
    }
}

fn update_cursor_grab(
    player_query: Query<&UseGamepad, With<LocalPlayer>>,
    mut primary_window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(gamepad) = player_query.get_single() else {
        return;
    };

    let Ok(mut window) = primary_window_query.get_single_mut() else {
        return;
    };

    window.cursor.visible = gamepad.has_gamepad();
}

fn sync_beta_crosshair(
    mut crosshair_query: Query<(&Node, &BetaCrosshair, &mut Style)>,
    player_query: Query<(&GlobalTransform, &Controller), With<LocalPlayer>>,
    camera_query: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
) {
    // get controller state
    let Ok((transform, controller)) = player_query.get_single() else {
        return;
    };

    // get camera state
    let Ok((camera_transform, camera)) = camera_query.get_single() else {
        return;
    };

    let Some(viewport_size) = camera.logical_viewport_size() else {
        return;
    };

    for (node, crosshair, mut style) in crosshair_query.iter_mut() {
        let pos = controller.shoot_dir() * crosshair.0;
        let pos = transform.translation() + pos.extend(0.);

        let Some(ndc_pos) = camera.world_to_ndc(camera_transform, pos) else {
            return;
        };

        // flip y
        let mut ndc_pos = ndc_pos.truncate();
        ndc_pos.y = -ndc_pos.y;

        // get pixels
        let pos = (ndc_pos + Vec2::ONE) / 2. * viewport_size;

        let node_size = node.size();

        style.left = Val::Px(pos.x - node_size.x / 2.);
        style.top = Val::Px(pos.y - node_size.y / 2.);
    }
}

fn sync_player_crosshair(
    mut crosshair_query: Query<(&Node, &mut Style), With<PlayerCrosshair>>,
    player_query: Query<(&GlobalTransform, &Controller, &UseGamepad), With<LocalPlayer>>,
    camera_query: Query<(&GlobalTransform, &Camera, &CursorWorldPosition), With<PlayerCamera>>,
) {
    // get controller state
    let Ok((transform, controller, gamepad)) = player_query.get_single() else {
        return;
    };

    // get camera state
    let Ok((camera_transform, camera, cursor_pos)) = camera_query.get_single() else {
        return;
    };

    let Some(viewport_size) = camera.logical_viewport_size() else {
        return;
    };

    // get position
    let world_pos = if gamepad.has_gamepad() {
        transform.translation() + (controller.shoot_dir() * 48.).extend(1.)
    } else {
        cursor_pos.0.extend(1.)
    };

    // undo transform
    let Some(ndc_pos) = camera.world_to_ndc(camera_transform, world_pos) else {
        return;
    };

    // flip y
    let mut ndc_pos = ndc_pos.truncate();
    ndc_pos.y = -ndc_pos.y;

    // get pixels
    let pos = (ndc_pos + Vec2::ONE) / 2. * viewport_size;

    for (node, mut style) in crosshair_query.iter_mut() {
        let node_size = node.size();

        style.left = Val::Px(pos.x - node_size.x / 2.);
        style.top = Val::Px(pos.y - node_size.y / 2.);
    }
}
