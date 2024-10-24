use std::time::Duration;

use bevy::{prelude::*, reflect::erased_serde::__private::serde::__private::de, render::camera::Viewport};
use bevy::time;
use bevy_ecs_tilemap::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use palette::PalettePlugin;

mod palette;
mod collision;
mod camera;
mod character;
mod util;
mod post_process;

const FIXED_TIMESTEP: f64 = 1.0 / 60.0;

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    // commands.spawn(Camera2dBundle {
    //     // projection: OrthographicProjection {
    //     //     //scaling_mode: bevy::render::camera::ScalingMode::WindowSize(1000.0),
    //     //     ..Default::default()
    //     // },
    //     ..Default::default()
    // });

    commands.spawn(camera::PlayerFollowCameraBundle::default());

    commands.spawn(LdtkWorldBundle {
        ldtk_handle: asset_server.load("world.ldtk"),
        ..Default::default()
    });

    // let egg_char_anim_handle = asset_server.load("egg_stomp-Sheet.png");
    // let egg_char_anim_atlas = TextureAtlas::from_grid(egg_char_anim_handle, Vec2::new(16.0, 16.0), 16, 1, None, None);
    // let egg_char_anim_atlas_handle = texture_atlases.add(egg_char_anim_atlas);
    
    // commands.spawn(character::PlayerBundle {
    //     spritesheet_bundle: SpriteSheetBundle {
    //         texture_atlas: egg_char_anim_atlas_handle,
    //         sprite: TextureAtlasSprite::new(1),
    //         transform: Transform::from_translation(Vec3::new(160.0, 160.0, 3.0)),
    //         ..default()
    //     },
    //     anim_indices: character::AnimationIndices { first: 0, last: 3 },
    //     anim_timer: character::AnimationTimer { time_animated: Duration::ZERO },
    //     grid_coords: GridCoords {
    //         x: 10,
    //         y: 10
    //     },
    //     tile_mover: default(),
    //     walk_anim: default(),
    //     player: character::Player {}
    // });
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Plat :)"),
                ..Default::default()
            }),
            ..default()
        }).set(ImagePlugin::default_nearest()))
        //.add_plugins(WorldInspectorPlugin::new())
        .add_plugins(TilemapPlugin)
        .add_plugins(LdtkPlugin)
        .add_systems(Startup, startup)
        .insert_resource(LdtkSettings {
            int_grid_rendering: IntGridRendering::Invisible,
            level_spawn_behavior: LevelSpawnBehavior::UseWorldTranslation { load_level_neighbors: true },
            ..default()
        })
        .insert_resource(LevelSelection::Indices(LevelIndices { level: 0, world: None }))
        .add_plugins(collision::CollisionPlugin)
        .add_plugins(camera::PlayerFollowCameraPlugin)
        .add_plugins(character::CharacterPlugin)
        .add_plugins(PalettePlugin)

        .insert_resource(Time::<Fixed>::from_seconds(FIXED_TIMESTEP))

        .add_plugins(post_process::PaletteSwapPostProcessPlugin)
        .run();
}