//! Simple moving platforms.

use bevy::prelude::*;

use bevy_rapier2d::prelude::*;

use bevy_ecs_ldtk::{
    app::{LdtkEntity, LdtkEntityAppExt as _},
    ldtk::{ldtk_fields::LdtkFields, LayerInstance, TilesetDefinition},
    EntityInstance,
};

use crate::level::Iid;
use crate::{GameAssets, GameState};

/// Platform plugin.
pub struct MovingPlatformPlugin;

impl Plugin for MovingPlatformPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ActivateEvent>()
            .register_ldtk_entity::<MovingPlatformBundle>("MovingPlatform")
            .add_systems(PostUpdate, on_added_platform)
            .add_systems(
                Update,
                update_platform_width
                    .run_if(in_state(GameState::InGame))
                    .in_set(PlatformSystem::UpdateWidth),
            )
            .add_systems(
                Update,
                animate_platform_gear.in_set(PlatformSystem::AnimateGear),
            )
            .add_systems(Update, listen_for_activation)
            .add_systems(
                FixedUpdate,
                move_platform.in_set(PlatformSystem::MovePlatform),
            );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, SystemSet)]
pub enum PlatformSystem {
    /// Updates the width of platforms.
    UpdateWidth,
    /// Updates the gear.
    AnimateGear,
    /// Actually moves the platform.
    MovePlatform,
}

/// An event for activating stuff (mostly platforms).
#[derive(Event)]
pub struct ActivateEvent(pub Entity);

/// A bundle for a moving platform
///
/// Scaling this horizontally will tile it in a special way.
#[derive(Bundle)]
pub struct MovingPlatformBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
    pub collider: Collider,
    pub rigidbody: RigidBody,
    pub moving_platform: MovingPlatform,
    pub platform_width: PlatformWidth,
    pub accumulated_distance: AccumulatedDistance,
    pub iid: Iid,
}

impl Default for MovingPlatformBundle {
    fn default() -> MovingPlatformBundle {
        MovingPlatformBundle {
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            collider: Collider::cuboid(24., 8.),
            rigidbody: RigidBody::KinematicPositionBased,
            moving_platform: Default::default(),
            platform_width: PlatformWidth(0),
            accumulated_distance: Default::default(),
            iid: Default::default(),
        }
    }
}

impl LdtkEntity for MovingPlatformBundle {
    fn bundle_entity(
        entity_instance: &EntityInstance,
        layer_instance: &LayerInstance,
        _tileset: Option<&Handle<Image>>,
        _tileset_definition: Option<&TilesetDefinition>,
        _asset_server: &AssetServer,
        _texture_atlases: &mut Assets<TextureAtlas>,
    ) -> Self {
        let mut target_grid_position = entity_instance
            .get_point_field("Target")
            .expect("valid target")
            .clone();

        // convert to bevy coordinates
        target_grid_position.y = layer_instance.c_hei - target_grid_position.y - 1;

        let target_pixel_position = target_grid_position * layer_instance.grid_size
            + IVec2::splat(layer_instance.grid_size / 2);
        let target_position = Vec2::new(
            target_pixel_position.x as f32,
            target_pixel_position.y as f32,
        );

        // get gear
        let gear_position = entity_instance
            .get_maybe_int_field("GearPosition")
            .expect("valid gear position")
            .clone()
            .map(|e| e as usize);

        MovingPlatformBundle {
            iid: entity_instance.into(),
            moving_platform: MovingPlatform::new(target_position, gear_position),
            ..Default::default()
        }
    }
}

/// A moving platform.
///
/// Scaling this horizontally will tile it in a special way.
#[derive(Clone, Component, Debug)]
pub struct MovingPlatform {
    /// How fast the platform will travel until it reaches its destination, in
    /// world units per second.
    pub speed: f32,
    /// The original position of the platform in local space.
    ///
    /// This will be overrided with the current [`Tranform`] when this entity
    /// spawns.
    pub original_location: Vec2,
    /// The target location of the moving platform in local space.
    pub target_location: Vec2,
    /// Target location in between the start and final destination. Must be a
    /// value between `0.` and `1.`.
    pub lerp: f32,
    /// Where the gear appears.
    pub gear_location: Option<usize>,
    /// The phase of the gear.
    pub gear_phase: usize,
}

impl MovingPlatform {
    /// Creates a new `MovingPlatform` with a target location and gear pos.
    pub fn new(target_location: Vec2, gear_location: Option<usize>) -> MovingPlatform {
        MovingPlatform {
            target_location,
            gear_location,
            ..Default::default()
        }
    }

    fn gear_sprite_index(&self) -> usize {
        3 + self.gear_phase % 3
    }
}

impl Default for MovingPlatform {
    fn default() -> MovingPlatform {
        MovingPlatform {
            speed: 160.,
            original_location: Vec2::default(),
            target_location: Vec2::default(),
            lerp: 0.,
            gear_location: None,
            gear_phase: 0,
        }
    }
}

/// Cached distance travelled for [`MovingPlatform`].
#[derive(Clone, Component, Debug, Default)]
pub struct AccumulatedDistance(f32);

/// Cached width for [`MovingPlatform`].
#[derive(Clone, Component, Debug, Default)]
pub struct PlatformWidth(usize);

#[derive(Clone, Component, Debug)]
struct PlatformGear;

fn on_added_platform(
    mut added_platforms: Query<(&Transform, &mut MovingPlatform), Added<MovingPlatform>>,
) {
    for (transform, mut platform) in added_platforms.iter_mut() {
        platform.original_location = transform.translation.truncate();
    }
}

fn listen_for_activation(
    mut activation_events: EventReader<ActivateEvent>,
    mut platforms_query: Query<&mut MovingPlatform>,
) {
    for ev in activation_events.iter() {
        let Ok(mut platform) = platforms_query.get_mut(ev.0) else {
            continue;
        };

        platform.lerp = 1.;
    }
}

fn update_platform_width(
    mut commands: Commands,
    mut platforms_query: Query<
        (
            Entity,
            &Transform,
            Option<&Children>,
            &MovingPlatform,
            &mut PlatformWidth,
        ),
        Changed<Transform>,
    >,
    assets: Res<GameAssets>,
) {
    for (platform_entity, transform, children, platform, mut platform_width) in
        platforms_query.iter_mut()
    {
        let scale = transform.scale.x;

        // update width
        let tile_width = (scale * 3.).floor() as usize;

        if tile_width == platform_width.0 {
            continue;
        }

        platform_width.0 = tile_width;

        // despawn old tiles
        if let Some(children) = children {
            children
                .iter()
                .for_each(|&e| commands.entity(e).despawn_recursive());
        }

        // create tiles
        for i in 0..tile_width {
            let (gear, sprite_idx) = match platform.gear_location {
                Some(loc) if loc == i => (true, platform.gear_sprite_index()),
                _ => {
                    (
                        false,
                        match i {
                            0 => 0,                        // first
                            i if i >= tile_width - 1 => 2, // last
                            _ => 1,                        // middle
                        },
                    )
                }
            };

            let x = (i as f32 / tile_width as f32) + (1. / tile_width as f32) / 2. - 0.5;
            let x = x * 16. * (1. / scale * tile_width as f32); // offset pixels

            let transform = Transform::from_xyz(x, 0., 0.)
                * Transform::from_scale(Vec3::new(1. / scale, 1., 1.));

            bevy::log::info!("transform = {:?}", transform);

            let mut entity = commands.spawn(SpriteSheetBundle {
                transform,
                texture_atlas: assets.platform_atlas.clone(),
                sprite: TextureAtlasSprite::new(sprite_idx),
                ..Default::default()
            });

            entity.set_parent(platform_entity);

            if gear {
                entity.insert(PlatformGear);
            }
        }
    }
}

fn animate_platform_gear(
    platforms_query: Query<(&Children, &MovingPlatform), Changed<MovingPlatform>>,
    mut gear_query: Query<&mut TextureAtlasSprite, With<PlatformGear>>,
) {
    for (children, platform) in platforms_query.iter() {
        let mut gears = gear_query.iter_many_mut(children);

        while let Some(mut sprite) = gears.fetch_next() {
            sprite.index = platform.gear_sprite_index();
        }
    }
}

fn move_platform(
    mut platforms_query: Query<(
        &mut Transform,
        &mut MovingPlatform,
        &mut AccumulatedDistance,
    )>,
    time: Res<FixedTime>,
) {
    for (mut transform, mut platform, mut acc) in platforms_query.iter_mut() {
        let mut current = transform.translation.truncate();
        let target = platform
            .original_location
            .lerp(platform.target_location, platform.lerp);

        let dist = move_toward(
            &mut current,
            target,
            platform.speed * time.period.as_secs_f32(),
        );

        transform.translation = current.extend(2.);

        acc.0 += dist;

        // get gear phase change TODO magic
        let phase_change = (acc.0 / 16.).floor();

        acc.0 -= phase_change * 8.;
        platform.gear_phase += phase_change as usize;
    }
}

fn move_toward(current: &mut Vec2, target: Vec2, max_movement: f32) -> f32 {
    let difference = target - *current;

    if difference.length_squared() > max_movement * max_movement {
        *current += difference.normalize() * max_movement;
        max_movement
    } else {
        *current = target;
        difference.length()
    }
}
