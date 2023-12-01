//! `tothe` library.

pub mod camera;
pub mod enemy;
pub mod environment;
pub mod physics;
pub mod player;
pub mod projectile;
//pub mod interactions;

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
                environment::EnvironmentPlugin,
                projectile::ProjectilePlugin,
                projectile::residue::ResiduePlugin,
                physics::PhysicsPlugin,
                player::PlayerPlugin,
                player::controller::ControllerPlugin,
                player::respawn::RespawnPlugin,
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
    #[asset(texture_atlas(tile_size_x = 16., tile_size_y = 16., columns = 2, rows = 1))]
    #[asset(path = "player/player.png")]
    pub player_sheet: Handle<TextureAtlas>,
    #[asset(texture_atlas(tile_size_x = 16., tile_size_y = 16., columns = 6, rows = 4))]
    #[asset(path = "projectiles/projectiles.png")]
    pub projectile_sheet: Handle<TextureAtlas>,
}

/// Game state.
#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    AssetLoading,
    InGame,
}

pub fn spawn_world(mut commands: Commands, assets: Res<GameAssets>) {
    commands.spawn(LdtkWorldBundle {
        ldtk_handle: assets.world.clone(),
        //transform: Transform::from_scale(Vec3::splat(16.)),
        ..Default::default()
    });
}
