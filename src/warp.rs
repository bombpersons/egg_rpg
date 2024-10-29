use std::{collections::HashMap, time::Duration};

use bevy::{app::{FixedUpdate, Plugin}, prelude::{Bundle, Commands, Component, Entity, EventReader, Query, Res, ResMut, Resource, With, Without}, time::{Time, Timer, TimerMode}};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, prelude::LdtkFields, EntityIid, EntityInstance, GridCoords, LdtkEntity, LevelIid, LevelSelection};

use crate::{character::{Player, TileMovedEvent}, collision::{WorldGridCoords, WorldGridCoordsRequired}, post_process::PaletteSwapPostProcessSettings};

// The target of a warp. 
#[derive(Clone, Debug)]
struct WarpTarget {
    level_iid: LevelIid, // The level to warp to.
    entity_iid: EntityIid // The entity id of the WarpTargetTile.
}

// A tile that when stepped on will teleport the player
// to another location.
#[derive(Clone, Debug, Component)]
struct WarpTile {
    target: Option<WarpTarget>
}

impl Default for WarpTile {
    fn default() -> Self {
        Self { target: None }
    }
}

#[derive(Clone, Debug, Default, Bundle, LdtkEntity)]
struct WarpTileBundle {
    #[from_entity_instance]
    warp_tile: WarpTile,

    #[grid_coords]
    grid_coords: GridCoords,
    world_grid_coords_required: WorldGridCoordsRequired
}

impl From<&EntityInstance> for WarpTile {
    fn from(entity_instance: &EntityInstance) -> Self {
        println!("Warp Tile: {:?}", entity_instance);
        let entity_reference = entity_instance.get_entity_ref_field("Target").expect("Warp should have a valid target!");

        Self {
            target: Some(WarpTarget {
                level_iid: LevelIid::new(entity_reference.level_iid.to_string()),
                entity_iid: EntityIid::new(entity_reference.entity_iid.to_string())
            })
        }
    }
}

#[derive(Clone, Debug, Default, Component)]
struct WarpTargetTile;

#[derive(Clone, Debug, Default, Bundle, LdtkEntity)]
struct WarpTargetTileBundle {
    warp_target_tile: WarpTargetTile,

    #[grid_coords]
    grid_coords: GridCoords,
    world_grid_coords_required: WorldGridCoordsRequired
}

// How long do we fade out before actually warping?
const WARP_FADE_OUT_TIME: Duration = Duration::from_millis(500);

// Specifies that the player is locked and cannot be moved due to a pending warp.
#[derive(Clone, Component)]
struct WarpLocked {
    target: WarpTarget,
    fade_out_timer: Timer
}

// Specifies that the player is waiting for the new level to be loaded so that they can be warped to their target.
#[derive(Clone, Component)]
struct WarpPending;

// Keep a resource that has all the locations and handy stuff for figuring out if where warps go to,
// where they are triggered on the map, etc.
// This way the player just needs to check this resource rather than query a bunch of entities.
#[derive(Default, Resource)]
struct WarpCache {
    warp_tiles: HashMap<WorldGridCoords, WarpTarget>,
    warp_targets: HashMap<EntityIid, WorldGridCoords>
}

// For now just do this every frame, 
fn build_warp_cache(mut warp_cache: ResMut<WarpCache>,
                    warp_tile_query: Query<(&WarpTile, &WorldGridCoords)>,
                    warp_tile_target_query: Query<(&WarpTargetTile, &EntityIid, &WorldGridCoords)>) {

    // Find all the warp points and build a convenient structure for getting their locations and targets.
    let mut warp_tiles = HashMap::new();
    for (warp_tile, coords) in &warp_tile_query {
        if let Some(warp_tile_target) = &warp_tile.target {
            warp_tiles.insert(*coords, warp_tile_target.clone());
        }
    }
    warp_cache.warp_tiles = warp_tiles;

    // Find all the targets.
    let mut warp_targets = HashMap::new();
    for (_, entity_iid, coords) in &warp_tile_target_query {
        warp_targets.insert(entity_iid.clone(), *coords);
    }
    warp_cache.warp_targets = warp_targets;
}

// What happens when the player walks onto a warp tile?
fn warp_player(mut commands: Commands,
               warp_cache: Res<WarpCache>,
               mut tile_moved_event_reader: EventReader<TileMovedEvent>,
               player_query: Query<(Entity, &Player, &WorldGridCoords), (Without<WarpLocked>)>) 
{   
    // Only triggers when a tile moves from one tile to another.
    for tile_moved_event in tile_moved_event_reader.read() {

        // Find entity in our query. (only interested in potential player)
        if let Ok((player_entity, _, world_grid_coords)) = player_query.get(tile_moved_event.entity) {

            // Did we step onto a warp tile?
            if let Some(warp_target) = warp_cache.warp_tiles.get(world_grid_coords) {
                println!("Attempting to warp player to new level {}", warp_target.level_iid);

                // Warp lock the player.
                commands.entity(player_entity).insert(WarpLocked {
                    target: warp_target.clone(),
                    fade_out_timer: Timer::new(Duration::from_secs_f32(WARP_FADE_OUT_TIME.as_secs_f32()), TimerMode::Once)
                });
            }
        }
    }
}

// Slowly fade out the rect. Once we've faded out completely, actually warp the player.
fn warp_fade_out(time: Res<Time>, 
                 mut commands: Commands,
                 mut level_select: ResMut<LevelSelection>, 
                 mut player_query: Query<(Entity, &mut WarpLocked), With<Player>>,
                 mut palette_settings: Query<&mut PaletteSwapPostProcessSettings>) {

    if let Ok((entity, mut warp_locked)) = player_query.get_single_mut() {
        // Reduce our timer.
        warp_locked.fade_out_timer.tick(time.delta());

        // How dark do we need to be?
        let mut darkness = (warp_locked.fade_out_timer.fraction() * 4.0) as i32;

        if warp_locked.fade_out_timer.just_finished() {
            // Load the target level.
            *level_select = LevelSelection::Iid(warp_locked.target.level_iid.clone());

            // Add a pending warp.
            commands.entity(entity).insert(WarpPending);
        }

        // Set the darkness level
        if !warp_locked.fade_out_timer.finished() {
            for mut settings in &mut palette_settings {
                settings.darkness = darkness;
            }
        }
    }
}

fn handle_pending_warp(mut commands: Commands,
                       warp_cache: Res<WarpCache>,
                       mut player_query: Query<(Entity, &mut WorldGridCoords, &WarpLocked, &WarpPending)>,
                       mut palette_settings: Query<&mut PaletteSwapPostProcessSettings>)
{
    for (entity, mut player_grid_coords, warp_locked, warp_pending) in player_query.iter_mut() {
        println!("Looking for warp tile id: {}", warp_locked.target.entity_iid.as_str());

        // Try and find the entity that we are warping to.
        if let Some(target_grid_coord) = warp_cache.warp_targets.get(&warp_locked.target.entity_iid) {
            println!("world coords for warp target: {}, {}, {}", target_grid_coord.x, target_grid_coord.y, target_grid_coord.z);

            // WARPING!
            player_grid_coords.x = target_grid_coord.x;
            player_grid_coords.y = target_grid_coord.y;
            player_grid_coords.z = target_grid_coord.z;

            // Remove the pending warp component and locked component.
            commands.entity(entity).remove::<WarpPending>();
            commands.entity(entity).remove::<WarpLocked>();

            // Reset the fade out.
            for mut settings in &mut palette_settings {
                settings.darkness = 0;
            }

            println!("Found warp tile, warped to ({}, {}, {})", player_grid_coords.x, player_grid_coords.y, player_grid_coords.z);
        }
    }
}

pub struct WarpPlugin;
impl Plugin for WarpPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // Manage Warp tiles.
        app.init_resource::<WarpCache>();
        app.register_ldtk_entity::<WarpTileBundle>("Warp");
        app.register_ldtk_entity::<WarpTargetTileBundle>("WarpTarget");
        app.add_systems(FixedUpdate, (build_warp_cache, warp_player, handle_pending_warp, warp_fade_out));
    }
}