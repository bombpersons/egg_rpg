use std::{collections::HashMap, time::Duration};

use bevy::{app::{FixedUpdate, Plugin}, asset::{Assets, Handle}, prelude::{run_once, Bundle, Commands, Component, Entity, EventReader, IntoSystemConfigs, Query, Res, ResMut, Resource, With, Without}, time::{Time, Timer, TimerMode}};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, assets::{InternalLevels, LdtkJsonWithMetadata, LdtkProject}, prelude::LdtkFields, EntityIid, EntityInstance, GridCoords, LdtkEntity, LevelIid, LevelSelection};

use crate::{character::{Player, TileMovedEvent}, collision::{self, WorldGridCoords, WorldGridCoordsRequired, TILE_GRID_SIZE}, post_process::PaletteSwapPostProcessSettings, util::run_if_ldtk_project_resource_available};

// The target of a warp. 
#[derive(Clone, Debug)]
struct WarpTarget {
    level_iid: LevelIid, // The level to warp to.
    entity_iid: EntityIid // The entity id of the WarpTargetTile.
}

// How long do we fade out before actually warping?
const WARP_FADE_OUT_TIME: Duration = Duration::from_millis(500);

// Specifies that the player is locked and cannot be moved due to a pending warp.
#[derive(Clone, Component)]
struct WarpPending {
    target: WarpTarget,
    fade_out_timer: Timer
}

// Keep a resource that has all the locations and handy stuff for figuring out if where warps go to,
// where they are triggered on the map, etc.
// This way the player just needs to check this resource rather than query a bunch of entities.
#[derive(Default, Debug, Resource)]
struct WarpCache {
    warp_tiles: HashMap<WorldGridCoords, WarpTarget>,
    warp_targets: HashMap<EntityIid, WorldGridCoords>
}

// For now just do this every frame, 
fn build_warp_cache(mut warp_cache: ResMut<WarpCache>,
                    ldtk_project_assets: Res<Assets<LdtkProject>>,
                    ldtk_project_entities: Query<&Handle<LdtkProject>>) {

    // Get the ldtk project data.
    let ldtk_project = ldtk_project_assets.get(ldtk_project_entities.single()).expect("ldtk project should be loaded before track_level system runs.");

    // The warp tiles and targets should be stored in the table of contents, so we can get 
    // all of the in the entire world before any levels are loaded.
    // Cache this data in our own resource so we can access it easily.
    for entry in &ldtk_project.json_data().toc {
        if entry.identifier == "Warp" {
            for instance in &entry.instances_data {

                // Get the z coord from the level it belongs to.
                let mut z = 0;
                for level in &ldtk_project.json_data().levels {
                    if level.iid == instance.iids.level_iid {
                        z = level.world_depth;
                    }
                }

                // Convert the world position of the instance to worldgridcoords.
                let world_grid_coords = WorldGridCoords {
                    x: (instance.world_x + instance.wid_px/2) / TILE_GRID_SIZE.x,
                    y: -(instance.world_y + instance.hei_px/2) / TILE_GRID_SIZE.y,
                    z: z
                };

                // Get the target this one points to.
                if let Some(serde_json::Value::Object(fields)) = &instance.fields {
                    if let Some(serde_json::Value::Object(target)) = fields.get("Target") {

                        // Entity iid.
                        let entity_iid = if let Some(serde_json::Value::String(entity_iid)) = target.get("entityIid") {
                            Some(EntityIid::new(entity_iid.clone()))
                        } else {
                            None
                        };

                        // Level iid
                        let level_iid = if let Some(serde_json::Value::String(level_iid)) = target.get("levelIid") {
                            Some(LevelIid::new(level_iid.clone()))
                        } else {
                            None
                        };

                        // If we have both... add the item to our cache.
                        if let (Some(entity_iid), Some(level_iid)) = (entity_iid, level_iid) {
                            warp_cache.warp_tiles.insert(world_grid_coords, WarpTarget {
                                entity_iid,
                                level_iid
                            });
                        }
                    }
                }
            }
        }

        if entry.identifier == "WarpTarget" {
            for instance in &entry.instances_data {
                
                // Get the z coord from the level it belongs to.
                let mut z = 0;
                for level in &ldtk_project.json_data().levels {
                    if level.iid == instance.iids.level_iid {
                        z = level.world_depth;
                    }
                }

                // Convert the world position of the instance to worldgridcoords.
                let world_grid_coords = WorldGridCoords {
                    x: (instance.world_x + instance.wid_px/2) / TILE_GRID_SIZE.x,
                    y: -(instance.world_y + instance.hei_px/2) / TILE_GRID_SIZE.y,
                    z: z
                };

                warp_cache.warp_targets.insert(EntityIid::new(instance.iids.entity_iid.clone()), world_grid_coords);
            }
        }
    }
}

// What happens when the player walks onto a warp tile?
fn warp_player(mut commands: Commands,
               warp_cache: Res<WarpCache>,
               mut tile_moved_event_reader: EventReader<TileMovedEvent>,
               player_query: Query<(Entity, &Player, &WorldGridCoords), Without<WarpPending>>) 
{   
    // Only triggers when a tile moves from one tile to another.
    for tile_moved_event in tile_moved_event_reader.read() {

        // Find entity in our query. (only interested in potential player)
        if let Ok((player_entity, _, world_grid_coords)) = player_query.get(tile_moved_event.entity) {

            // Did we step onto a warp tile?
            if let Some(warp_target) = warp_cache.warp_tiles.get(world_grid_coords) {
                println!("Attempting to warp player to new level {}", warp_target.level_iid);

                // Warp lock the player.
                commands.entity(player_entity).insert(WarpPending {
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
                 warp_cache: Res<WarpCache>,
                 mut player_query: Query<(Entity, &mut WorldGridCoords, &mut WarpPending), With<Player>>,
                 mut palette_settings: Query<&mut PaletteSwapPostProcessSettings>,
                 level_query: Query<&LevelIid>) {

    if let Ok((entity, mut player_grid_coords, mut warp_locked)) = player_query.get_single_mut() {
        // Reduce our timer.
        warp_locked.fade_out_timer.tick(time.delta());

        // How dark do we need to be?
        let darkness = (warp_locked.fade_out_timer.fraction() * 4.0) as i32;

        if warp_locked.fade_out_timer.just_finished() {
            // Load the target level.
            //*level_select = LevelSelection::Iid(warp_locked.target.level_iid.clone());

            // The warp target locations are already known since we
            // parsed the TOC in the ldtk file json data.
            // So we can warp right now rather than having to wait for the new level to load.

            // Try and find the entity that we are warping to.
            if let Some(target_grid_coord) = warp_cache.warp_targets.get(&warp_locked.target.entity_iid) {
                println!("world coords for warp target: {}, {}, {}", target_grid_coord.x, target_grid_coord.y, target_grid_coord.z);

                // WARPING!
                player_grid_coords.x = target_grid_coord.x;
                player_grid_coords.y = target_grid_coord.y;
                player_grid_coords.z = target_grid_coord.z;

                println!("Found warp tile, warped to ({}, {}, {})", player_grid_coords.x, player_grid_coords.y, player_grid_coords.z);
            }
        }

        // Set the darkness level
        if !warp_locked.fade_out_timer.finished() {
            for mut settings in &mut palette_settings {
                settings.darkness = darkness;
            }
        }

        // If the timer is finished, we might be waiting for the level we're warping to, to load. 
        // So let's check if it's loaded, and if it is then we can reset the darkness.
        if warp_locked.fade_out_timer.finished() {

            // Check if the target level is loaded.
            for level_iid in &level_query {
                if *level_iid == warp_locked.target.level_iid {
                    
                    // Okay it's loaded. Remove the pending warp component and reset our darkness.
                    commands.entity(entity).remove::<WarpPending>();
                    for mut settings in &mut palette_settings {
                        settings.darkness = 0;
                    }
                }
            }
        }
    }
}

pub struct WarpPlugin;
impl Plugin for WarpPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {

        // Manage Warp tiles.

        // Initialize the warp cache, run the build system only once and only when the ldtk project is available.
        app.init_resource::<WarpCache>();
        app.add_systems(FixedUpdate, build_warp_cache.run_if(run_if_ldtk_project_resource_available).run_if(run_once()));

        // Handle walking onto tiles and actually warping to new locations.
        app.add_systems(FixedUpdate, (warp_player, warp_fade_out));
    }
}