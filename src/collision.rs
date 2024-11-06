use std::{collections::HashSet, thread::current};

use bevy::prelude::*;
use bevy_ecs_ldtk::{prelude::*, utils::{ldtk_grid_coords_to_grid_coords, ldtk_pixel_coords_to_grid_coords, ldtk_pixel_coords_to_translation, translation_to_grid_coords}};
use bevy_ecs_tilemap::prelude::*;
use bevy::{app::{App, Plugin, Update}, asset::{Assets, Handle}, ecs::{entity, world}, math::IVec2, prelude::{Added, Bundle, Commands, Component, Entity, EventReader, Query, Res, ResMut, Resource, With, World}};
use bevy_ecs_ldtk::{app::LdtkIntCellAppExt, assets::{LdtkProject, LevelMetadataAccessor}, EntityInstance, GridCoords, IntGridCell, LdtkIntCell, LevelEvent};

use bevy_ecs_ldtk::app::LdtkEntity;
use bevy_inspector_egui::egui::Grid;
use ldtk::loaded_level::LoadedLevel;

use crate::util;

pub const TILE_GRID_SIZE: IVec2 = IVec2::new(16, 16);
const BLOCKED_TILE_GRID_CELL: i32 = 1;

// This will be swapped out for a valid worldgridcoords
#[derive(Debug, Default, Clone, Component)]
pub struct WorldGridCoordsRequired;

// A grid coordinate in world coordinates
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Component)]
pub struct WorldGridCoords {
    pub x: i32,
    pub y: i32, 
    pub z: i32 // Layer,
}

// A system that should set the world grid coords component to have correct world grid coordinates.
fn world_grid_coords_required(mut commands: Commands,
                              mut world_grid_coords_query: Query<(Entity, &GridCoords, &Parent), Added<WorldGridCoordsRequired>>,
                              parent_query: Query<&Parent, Without<WorldGridCoordsRequired>>,
                              level_query: Query<&LevelIid>,
                              ldtk_project_entities: Query<&Handle<LdtkProject>>,
                              ldtk_project_assets: Res<Assets<LdtkProject>>) {

    // Get the ldtk project so that we can get the level data from it.
    let ldtk_project = ldtk_project_assets.get(ldtk_project_entities.single())
                                    .expect("LdtkProject should be loaded when level is spawned");
    
    for (entity, grid_coords, parent) in world_grid_coords_query.iter_mut() {
        // The Parent's Parent is the level entity, so we can use that
        // to get the offset for the grid coords \o/
        let layer_entity = parent.get();
        if let Ok(level_parent) = parent_query.get(layer_entity) {
            // Find the level iid
            if let Ok(level_iid) = level_query.get(level_parent.get()) {
                // Now finally get the level.
                let level = ldtk_project.get_raw_level_by_iid(level_iid.get()).expect("world grid coord tile should have a grandparent level!");
                
                //println!("level world pos: {}, {}, {}", level.world_x, level.world_y, level.world_depth);

                // Yay, this is the adjustment required *phew*
                let mut level_origin_adjusted = IVec2::new(level.world_x, 0 - level.world_y - level.px_hei);
                level_origin_adjusted = level_origin_adjusted / TILE_GRID_SIZE;

                //println!("level world pos adjusted: {}, {}", level_origin_adjusted.x, level_origin_adjusted.y);

                // Hurray now we can get the absolute origin of the level and adjust our coordinates.
                // Insert the valid world grid coord component
                let world_grid_coords = WorldGridCoords {
                    x: level_origin_adjusted.x + grid_coords.x,
                    y: level_origin_adjusted.y + grid_coords.y,
                    z: level.world_depth
                };
                commands.entity(entity).insert(world_grid_coords);

                // Remove the old component
                commands.entity(entity).remove::<WorldGridCoordsRequired>();
            }
        }
    }
}

// An entity that can't be walked through.
#[derive(Clone, Debug, Default, Component)]
pub struct Blocking; 

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct BlockedTileBundle {
    blocked_tile: Blocking,
    world_grid_coords_required: WorldGridCoordsRequired
}

// Maintain a cache of all the tile locations that are blocked
// This way we can easily tell if a location can't be occupied by an tile entity.
#[derive(Default, Resource)]
pub struct BlockedTilesCache {
    pub blocked_tile_locations: HashSet<WorldGridCoords>
}

// Whenever a level is loaded, then rebuild our cache.
fn build_blocked_tile_cache(mut blocked_tiles_cache: ResMut<BlockedTilesCache>,
                            blocked_tiles: Query<&WorldGridCoords, With<Blocking>>) {
    
    // Collect all of the blocked tiles that currently exist.
    let mut blocked_tile_locations = HashSet::new();
    for world_grid_coords in blocked_tiles.iter() {
        blocked_tile_locations.insert(*world_grid_coords);
    }

    // Build up a new cache.
    let new_blocked_tiles_cache = BlockedTilesCache {
        blocked_tile_locations: blocked_tile_locations
    };

    // Update the resource.
    *blocked_tiles_cache = new_blocked_tiles_cache;
}

pub struct CollisionPlugin;
impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_int_cell::<BlockedTileBundle>(BLOCKED_TILE_GRID_CELL);
            
        // The resource for the cache.
        app.init_resource::<BlockedTilesCache>();

        // These should only run if the ldtk project is available.
        app.add_systems(FixedUpdate, (world_grid_coords_required, build_blocked_tile_cache).run_if(util::run_if_ldtk_project_resource_available));
    }
}