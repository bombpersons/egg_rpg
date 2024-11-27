// Manage how we load levels.
// We want levels to be loaded around the player, including
// neighbouring levels so that it seems seemless.

// bevy_ecs_ldtk does have a feature for this but it ends up including levels with differing world_depths at the same time.
// this results in multiple levels being overlayed on top of each other.

// so instead of letting bevy_ecs_ldtk do it, we're gonna do it manually.

use std::{collections::{HashMap, HashSet}, thread::current};

use bevy::{app::{FixedUpdate, Plugin}, asset::{Assets, Handle}, log::Level, math::{Rect, Vec2, Vec3Swizzles}, prelude::{run_once, Added, Commands, Component, Entity, Event, EventReader, EventWriter, GlobalTransform, IntoSystemConfigs, Query, Res, ResMut, Resource, With}};
use bevy_ecs_ldtk::{assets::{LdtkProject, LevelMetadataAccessor}, EntityIid, LevelEvent, LevelIid, LevelSet, Worldly};

use crate::{character::Player, collision::WorldGridCoords, util::run_if_ldtk_project_resource_available};

// This just tracks what level an entity is currently contained within.
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

#[derive(Component)]
pub struct CurrentLevelLoading;

#[derive(Event)]
pub enum CurrentLevelChangedEvent {
    Changed(EntityIid, Option<LevelIid>, Option<LevelIid>),
    ChangedAndLoaded(EntityIid, LevelIid)
}

// If an entity has a world grid coord component, then we can use that position to determine which level bounds it intersects
// with! Then, other systems that need to know what level a wordly entity is located within can know easily.
fn track_level(mut commands: Commands,
               mut wordly_query: Query<(Entity, &EntityIid, &WorldGridCoords, &GlobalTransform, &mut CurrentLevel)>,
               mut current_level_event_writer: EventWriter<CurrentLevelChangedEvent>,
               level_query: Query<&LevelIid>,
               ldtk_projects: Query<&Handle<LdtkProject>>,
               ldtk_project_assets: Res<Assets<LdtkProject>>) {
    
    // Get the ldtk project .
    let ldtk_project = ldtk_project_assets.get(ldtk_projects.single()).expect("ldtk project should be loaded before track_level system runs.");

    // For each worldy entity that we are keeping track of.
    for (entity, entity_iid, world_grid_coords, global_transform, mut current_level) in &mut wordly_query {

        // The level we've chosen that intersects.
        let mut selected_level = None;

        // Go through each level and see which bounds we are contained within.
        for level in &ldtk_project.json_data().levels {
            let level_bounds = Rect {
                min: Vec2::new(
                    level.world_x as f32,
                    (0 - level.world_y - level.px_hei) as f32
                ),
                max: Vec2::new(
                (level.world_x + level.px_wid) as f32,
                    -level.world_y as f32,
                ),
            };

            // We're within the 2d bounds...
            if level_bounds.contains(global_transform.translation().xy()) {

                // Check if our z coordinate is the same?
                if world_grid_coords.z == level.world_depth {

                    // We are contained by this level bounds.
                    selected_level = Some(LevelIid::new(level.iid.clone()));

                    // Stop looking since there shouldn't be any overlapping levels.
                    break;
                }
            }
        }

        // If the level changed, then write an event for that changed.
        // Other systems might find this useful.
        if current_level.level_iid != selected_level {

            // Level changed.
            current_level_event_writer.send(CurrentLevelChangedEvent::Changed(
                entity_iid.clone(),
                current_level.level_iid.clone(),
                selected_level.clone()
            ));

            // Is the new level already loaded? This will often be the case
            // if the player is moving to a neighbouring level.
            let mut loaded = false;
            if let Some(selected_level) = &selected_level {
                for level in &level_query {
                    if level == selected_level {
                        loaded = true;
                    }
                }

                if loaded == true {
                    current_level_event_writer.send(CurrentLevelChangedEvent::ChangedAndLoaded(
                        entity_iid.clone(),
                        selected_level.clone()
                    ));
                } else {
                    // It's going to be loaded eventually, add a pending component.
                    commands.entity(entity).insert(CurrentLevelLoading);
                }
            }
        }

        // Set the level.
        current_level.level_iid = selected_level;
    }
}

#[derive(Resource, Debug, Default)]
struct LevelNeighboursCache {
    neighbours: HashMap<LevelIid, HashSet<LevelIid>>
}

fn cache_level_neighbours(mut cache: ResMut<LevelNeighboursCache>,
                          ldtk_projects: Query<&Handle<LdtkProject>>,
                          ldtk_project_assets: Res<Assets<LdtkProject>>) {
    
    // Get the ldtk project .
    let ldtk_project = ldtk_project_assets.get(ldtk_projects.single()).expect("ldtk project should be loaded before track_level system runs.");
    
    // Clear the cache.
    cache.neighbours.clear();

    // Loop through all the levels and re-calculate their neighbours.
    for level in &ldtk_project.json_data().levels {
        let mut levelset = HashSet::new();
        for neighbour in &level.neighbours {

            // Take the neighbour list here, and filter out any levels that don't share a world_depth.
            for neighbour_level in &ldtk_project.json_data().levels {
                if neighbour_level.iid == neighbour.level_iid {
                    if neighbour_level.world_depth == level.world_depth {

                        // This one matches!
                        levelset.insert(LevelIid::new(&neighbour_level.iid));
                    }
                }
            }
        }

        cache.neighbours.insert(LevelIid::new(level.iid.clone()), levelset);
    }
}

fn load_levels(neighbours_cache: Res<LevelNeighboursCache>,
               mut current_level_changed_reader: EventReader<CurrentLevelChangedEvent>,
               player_query: Query<&EntityIid, (With<Player>, With<CurrentLevel>)>,
               mut level_set_query: Query<&mut LevelSet>) {

    // Is the player about?
    if let Ok(player_iid) = player_query.get_single() {

        // Go over all the level changed events.
        for current_level_changed_event in current_level_changed_reader.read() {

            // Only interested in a level changed event for the player.
            if let CurrentLevelChangedEvent::Changed(changed_entity_iid, _, Some(new_level_iid)) = current_level_changed_event {
                if changed_entity_iid == player_iid {

                    // Get the neighbouring levels (from our handy cache that excludes neighbours not on the same world_depth)
                    if let Some(neighbours) = neighbours_cache.neighbours.get(new_level_iid) {

                        // Grab the the level set and update it.
                        if let Ok(mut level_set) = level_set_query.get_single_mut() {

                            // All of the neighbours
                            let mut levels_to_be_loaded = HashSet::new();
                            for neighbour in neighbours {
                                levels_to_be_loaded.insert(neighbour.clone());
                            }

                            // And don't forget the level that we are currently on, otherwise we'd unload that =/
                            levels_to_be_loaded.insert(new_level_iid.clone());

                            // Update the level set component.
                            level_set.iids = levels_to_be_loaded;
                        }

                    }

                }
            }
        }
    }
}

fn check_levels_loaded(mut commands: Commands,
                       current_level_query: Query<(Entity, &EntityIid, &CurrentLevel), With<CurrentLevelLoading>>,
                       level_query: Query<&LevelIid, Added<LevelIid>>,
                       mut current_level_event_writer: EventWriter<CurrentLevelChangedEvent>) {
    for (entity, entity_iid, current_level) in &current_level_query {
        if let Some(current_level_iid) = &current_level.level_iid {
            // Check if the level is loaded?
            for level_iid in &level_query {
                if current_level_iid == level_iid {
                    // Cool it's loaded!
                    // Send a message conveying that fact, and remove the CurrentLevelLoading component.
                    commands.entity(entity).remove::<CurrentLevelLoading>();

                    // Send the message.
                    current_level_event_writer.send(CurrentLevelChangedEvent::ChangedAndLoaded(
                        entity_iid.clone(),
                        level_iid.clone()
                    ));
                }
            }
        }
    }
}

pub struct LevelLoadingPlugin;
impl Plugin for LevelLoadingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // Events
        app.add_event::<CurrentLevelChangedEvent>();

        // Resources.
        app.init_resource::<LevelNeighboursCache>();

        // Caching neighbours.
        app.add_systems(FixedUpdate, cache_level_neighbours.run_if(run_if_ldtk_project_resource_available).run_if(run_once()));

        // Level tracking and level loading.
        app.add_systems(FixedUpdate, track_level.run_if(run_if_ldtk_project_resource_available));
        app.add_systems(FixedUpdate, (load_levels, check_levels_loaded));
    }
}