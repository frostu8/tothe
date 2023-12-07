//! Drums!

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use bevy_ecs_ldtk::{
    app::{LdtkEntity, LdtkEntityAppExt as _},
    ldtk::{LayerInstance, TilesetDefinition},
    EntityInstance,
};

use crate::projectile::{ProjectileSystem, HitEvent, prefab::{CreateProjectile, ProjectilePrefab}};
use crate::enemy::Hostility;
use crate::{physics, GameState, GameAssets};

pub struct DrumPlugin;

impl Plugin for DrumPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_ldtk_entity::<DrumBundle>("Drum")
            .add_systems(
                Update,
                handle_projectiles
                    .after(ProjectileSystem::Event),
            )
            .add_systems(
                PostUpdate,
                setup_added_drums
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

/// A drum will produce allied beat notes when hit.
#[derive(Clone, Component, Debug, Default)]
pub struct Drum;

#[derive(Bundle)]
pub struct DrumBundle {
    transform: Transform,
    global_transform: GlobalTransform,
    visibility: Visibility,
    computed_visibility: ComputedVisibility,
    image: Handle<Image>,
    sprite: Sprite,
    collider: Collider,
    collision_groups: CollisionGroups,
    drum: Drum,
}

impl Default for DrumBundle {
    fn default() -> DrumBundle {
        DrumBundle {
            transform: Transform::default(),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            computed_visibility: ComputedVisibility::default(),
            collider: Collider::cuboid(24., 16.),
            collision_groups: CollisionGroups::new(
                physics::COLLISION_GROUP_SOLID,
                Group::all(),
            ),
            image: Default::default(),
            sprite: Sprite::default(),
            drum: Drum,
        }
    }
}

impl LdtkEntity for DrumBundle {
    fn bundle_entity(
        _entity_instance: &EntityInstance,
        _layer_instance: &LayerInstance,
        _tileset: Option<&Handle<Image>>,
        _tileset_definition: Option<&TilesetDefinition>,
        _asset_server: &AssetServer,
        _texture_atlases: &mut Assets<TextureAtlas>
    ) -> Self {
        DrumBundle::default()
    }
}

fn setup_added_drums(
    mut added_drums_query: Query<&mut Handle<Image>, Added<Drum>>,
    assets: Res<GameAssets>,
) {
    for mut image in added_drums_query.iter_mut() {
        *image = assets.drum_image.clone();
    }
}

fn handle_projectiles(
    mut commands: Commands,
    mut projectile_hit_events: EventReader<HitEvent>,
    drum_query: Query<&GlobalTransform, With<Drum>>,
    projectile_query: Query<&Hostility>,
) {
    for ev in projectile_hit_events.iter() {
        let Ok(drum_transform) = drum_query.get(ev.entity) else {
            continue;
        };

        let Ok(hostility) = projectile_query.get(ev.projectile) else {
            continue;
        };

        let mut location = drum_transform.translation();
        location.y += 14.;

        // create projectile
        // FIXME magic
        commands.add(CreateProjectile::new(ProjectilePrefab::Beat { initial_velocity: Vec2::Y * 16. }, location)
            .hostility(hostility.clone()));
    }
}

