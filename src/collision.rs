use std::{collections::HashSet, thread::current};

use bevy::prelude::*;
use bevy_ecs_ldtk::{prelude::*, utils::{ldtk_grid_coords_to_grid_coords, ldtk_pixel_coords_to_grid_coords, ldtk_pixel_coords_to_translation, translation_to_grid_coords}};
use bevy_ecs_tilemap::prelude::*;
use bevy::{app::{App, Plugin, Update}, asset::{Assets, Handle}, ecs::{entity, world}, math::IVec2, prelude::{Added, Bundle, Commands, Component, Entity, EventReader, Query, Res, ResMut, Resource, With, World}};
use bevy_ecs_ldtk::{app::LdtkIntCellAppExt, assets::{LdtkProject, LevelMetadataAccessor}, EntityInstance, GridCoords, IntGridCell, LdtkIntCell, LevelEvent};

use bevy_ecs_ldtk::app::LdtkEntity;
use bevy_inspector_egui::egui::Grid;
use ldtk::loaded_level::LoadedLevel;

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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Component)]
pub struct CurrentLevel {
    pub level_iid: Option<LevelIid>
}

impl Default for CurrentLevel {
    fn default() -> Self {
        Self {
            level_iid: None
        }
    }
}

// If an entity has a world grid coord component, then we can use that position to determine which level bounds it intersects
// with! Then, other systems that need to know what level a wordly entity is located within can know easily.
fn track_level(mut wordly_query: Query<(&WorldGridCoords, &GlobalTransform, &mut CurrentLevel), With<Worldly>>,
              levels: Query<(&LevelIid, &GlobalTransform)>,
              ldtk_projects: Query<&Handle<LdtkProject>>,
              ldtk_project_assets: Res<Assets<LdtkProject>>) {
    
    // Get the ldtk project .
    let ldtk_project = ldtk_project_assets.get(ldtk_projects.single()).expect("ldtk project should be loaded before track_level system runs.");

    // For each worldy entity that we are keeping track of.
    for (world_grid_coords, global_transform, mut current_level) in &mut wordly_query {

        // The level we've chosen that intersects.
        let mut selected_level = None;

        // Go through each level and see which bounds we are contained within.
        for (level_iid, level_transform) in levels.iter() {
            let level = ldtk_project
                .get_raw_level_by_iid(level_iid.get())
                .expect("level should exist in only project");

            let level_bounds = Rect {
                min: Vec2::new(
                    level_transform.translation().x,
                    level_transform.translation().y,
                ),
                max: Vec2::new(
                    level_transform.translation().x + level.px_wid as f32,
                    level_transform.translation().y + level.px_hei as f32,
                ),
            };

            // We're within the 2d bounds...
            if level_bounds.contains(global_transform.translation().xy()) {

                // Check if our z coordinate is the same?
                if world_grid_coords.z == level.world_depth {

                    // We are contained by this level bounds.
                    selected_level = Some(level_iid.clone());

                    // Stop looking for this entity.
                    break;
                }
            }
        }

        // Set the level.
        current_level.level_iid = selected_level
    }
}

// A system that should set the world grid coords component to have correct world grid coordinates.
fn world_grid_coords_added(mut commands: Commands,
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

                //println!("Blocked tile at: {}, {}, {}", world_grid_coords.x, world_grid_coords.y, world_grid_coords.z);    
            }
        }
    }
}

// A tile that can't be walked through.
#[derive(Clone, Debug, Component)]
struct BlockedTile;
impl Default for BlockedTile {
    fn default() -> Self {
        Self
    }
}

#[derive(Clone, Debug, Default, Bundle, LdtkIntCell)]
pub struct BlockedTileBundle {
    blocked_tile: BlockedTile,
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
                            blocked_tiles: Query<&WorldGridCoords, With<BlockedTile>>) {
    
    // Collect all of the blocked tiles that currently exist.
    let mut blocked_tile_locations = HashSet::new();
    for (world_grid_coords) in blocked_tiles.iter() {
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
        // Register blocked tiles to spawn based on the int cell value (1)
        app.register_ldtk_int_cell::<BlockedTileBundle>(BLOCKED_TILE_GRID_CELL);

        // The resource for the cache.
        app.init_resource::<BlockedTilesCache>();

        // The system for keeping it up to date.
        app.add_systems(Update, (world_grid_coords_added, track_level, build_blocked_tile_cache));
    }
}