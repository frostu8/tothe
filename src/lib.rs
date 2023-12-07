//! `tothe` library.

pub mod camera;
pub mod drum;
pub mod enemy;
pub mod interactions;
pub mod level;
pub mod physics;
pub mod platform;
pub mod player;
pub mod projectile;
pub mod ui;

use bevy::prelude::*;

use bevy_ecs_ldtk::{LdtkAsset, LdtkWorldBundle};

use bevy_asset_loader::prelude::*;

/// Generic game plugin.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>()
            .add_plugins((
                camera::CameraPlugin,
                camera::hint::CameraHintPlugin,
                camera::cursor::CameraCursorPlugin,
                level::LevelPlugin,
                level::pipe::LevelPipePlugin,
                projectile::ProjectilePlugin,
                projectile::residue::ResiduePlugin,
                projectile::spawner::ProjectileSpawnerPlugin,
                physics::PhysicsPlugin,
                platform::MovingPlatformPlugin,
                player::PlayerPlugin,
                player::controller::ControllerPlugin,
                player::respawn::RespawnPlugin,
                interactions::InteractionPlugins,
                ui::UiPlugin,
            ))
            .add_plugins((
                enemy::EnemyPlugin,
                enemy::prefab::EnemyPrefabPlugin,
                drum::DrumPlugin,
            ))
            .add_loading_state(
                LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::InGame),
            )
            .add_collection_to_loading_state::<_, GameAssets>(GameState::AssetLoading)
            .add_systems(OnEnter(GameState::InGame), spawn_world);
    }
}

/// Global assets.
#[derive(AssetCollection, Resource)]
pub struct GameAssets {
    #[asset(path = "world/world.ldtk")]
    pub world: Handle<LdtkAsset>,
    #[asset(texture_atlas(tile_size_x = 16., tile_size_y = 16., columns = 3, rows = 2))]
    #[asset(path = "world/platform.png")]
    pub platform_atlas: Handle<TextureAtlas>,
    #[asset(path = "world/drum.png")]
    pub drum_image: Handle<Image>,
    #[asset(texture_atlas(tile_size_x = 16., tile_size_y = 16., columns = 2, rows = 1))]
    #[asset(path = "player/player.png")]
    pub player_sheet: Handle<TextureAtlas>,
    #[asset(texture_atlas(tile_size_x = 16., tile_size_y = 16., columns = 6, rows = 4))]
    #[asset(path = "projectiles/projectiles.png")]
    pub projectile_sheet: Handle<TextureAtlas>,
    #[asset(texture_atlas(tile_size_x = 32., tile_size_y = 24., columns = 7, rows = 1))]
    #[asset(path = "enemy/howard/howard.png")]
    pub enemy_howard: Handle<TextureAtlas>,
    #[asset(path = "signal/signal_matte.png")]
    pub signal_matte: Handle<Image>,
    #[asset(path = "signal/signal_mask.png")]
    pub signal_mask: Handle<Image>,
    #[asset(path = "player/crosshair.png")]
    pub crosshair: Handle<Image>,
    #[asset(path = "player/crosshair_beta.png")]
    pub crosshair_beta: Handle<Image>,
    #[asset(path = "player/conceal.png")]
    pub conceal: Handle<Image>,
    #[asset(path = "player/conceal_wedge.png")]
    pub conceal_wedge: Handle<Image>,
}

/// Game state.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    AssetLoading,
    InGame,
}

/// The main world.
#[derive(Clone, Component, Default, Debug)]
pub struct GameWorld;

pub fn spawn_world(mut commands: Commands, assets: Res<GameAssets>) {
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: assets.world.clone(),
            //transform: Transform::from_scale(Vec3::splat(16.)),
            ..Default::default()
        },
        GameWorld,
    ));
}
