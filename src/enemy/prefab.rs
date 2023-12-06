//! Prefab stuff.

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use bevy_ecs_ldtk::{
    app::{LdtkEntity, LdtkEntityAppExt as _},
    ldtk::{ldtk_fields::LdtkFields, LayerInstance, TilesetDefinition},
    EntityInstance,
};

use super::{ActivateOnDeathByIid, EnemyBundle};

use crate::{GameAssets, GameState};

pub struct EnemyPrefabPlugin;

impl Plugin for EnemyPrefabPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<HowardBundle>("Howard")
            .add_systems(
                Update,
                setup_enemy_prefab.run_if(in_state(GameState::InGame)),
            );
    }
}

/// Enemy prefab stuff.
#[derive(Clone, Component, Debug)]
pub enum EnemyPrefab {
    /// Howard.
    ///
    /// See [`HowardBundle`].
    Howard,
}

/// Howard.
#[derive(Bundle)]
pub struct HowardBundle {
    enemy_bundle: EnemyBundle,
    enemy_prefab: EnemyPrefab,
    texture_atlas: Handle<TextureAtlas>,
    sprite: TextureAtlasSprite,
    activate_on_death: ActivateOnDeathByIid,
}

impl LdtkEntity for HowardBundle {
    // Required method
    fn bundle_entity(
        entity_instance: &EntityInstance,
        _layer_instance: &LayerInstance,
        _tileset: Option<&Handle<Image>>,
        _tileset_definition: Option<&TilesetDefinition>,
        _asset_server: &AssetServer,
        _texture_atlases: &mut Assets<TextureAtlas>,
    ) -> Self {
        // get activation ref first
        let activate_ref = entity_instance
            .get_maybe_entity_ref_field("ActivateOnDeath")
            .ok() // may not exist
            .and_then(|a| a.as_ref())
            .map(|a| a.entity_iid.clone());

        HowardBundle {
            enemy_bundle: EnemyBundle {
                collider: Collider::cuboid(8., 8.),
                ..Default::default()
            },
            enemy_prefab: EnemyPrefab::Howard,
            activate_on_death: ActivateOnDeathByIid(activate_ref),
            texture_atlas: Default::default(),
            sprite: Default::default(),
        }
    }
}

fn setup_enemy_prefab(
    mut enemy_prefab_query: Query<(&mut Handle<TextureAtlas>, &EnemyPrefab), Added<EnemyPrefab>>,
    assets: Res<GameAssets>,
) {
    for (mut texture_handle, enemy_prefab) in enemy_prefab_query.iter_mut() {
        match enemy_prefab {
            EnemyPrefab::Howard => *texture_handle = assets.enemy_howard.clone(),
        }
    }
}
