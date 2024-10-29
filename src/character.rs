use std::time::Duration;

use bevy::{ecs::world, prelude::*, sprite::{Material2d, MaterialMesh2dBundle}, transform::components};
use bevy_ecs_tilemap::prelude::*;
use bevy_ecs_ldtk::{assets::{InternalLevels, LdtkJsonWithMetadata}, prelude::*};

use crate::{camera::PlayerFollowCameraBundle, collision::{self, BlockedTilesCache, CurrentLevel, WorldGridCoords, WorldGridCoordsRequired}, post_process::PaletteSwapPostProcessSettings};

const MOVEMENT_TICK: f32 = 20.0 / 60.0;
const ANIMATION_FRAME_TIME: f32 = MOVEMENT_TICK / 2.0;

// A tile that when stepped on will teleport the player
// to another location.
#[derive(Clone, Debug, Component)]
struct WarpTile {
    level_iid: Option<LevelIid>,
    entity_iid: Option<EntityIid>,
}

impl Default for WarpTile {
    fn default() -> Self {
        Self { level_iid: None, entity_iid: None }
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
            level_iid: Some(LevelIid::new(entity_reference.level_iid.to_string())),
            entity_iid: Some(EntityIid::new(entity_reference.entity_iid.to_string()))
        }
    }
}

// How long do we fade out before actually warping?
const WARP_FADE_OUT_TIME: Duration = Duration::from_millis(500);

// Specifies that the player is locked and cannot be moved due to a pending warp.
#[derive(Clone, Component)]
struct WarpLocked {
    level_iid: LevelIid,
    warp_target_entity_iid: EntityIid,
    fade_out_timer: Timer
}

// Specifies that the player is waiting for the new level to be loaded so that they can be warped to their target.
#[derive(Clone, Component)]
struct WarpPending;

fn warp_player(mut commands: Commands,
               mut tile_moved_event_reader: EventReader<TileMovedEvent>,
               player_query: Query<(Entity, &Player, &WorldGridCoords), Without<WarpTile>>,
               warp_tiles: Query<(&WarpTile, &WorldGridCoords)>) 
{   
    // Only triggers when a tile moves from one tile to another.
    for tile_moved_event in tile_moved_event_reader.read() {
        // Find entity in our query. (only interested in potential player)
        if let Ok((player_entity, _, world_grid_coords)) = player_query.get(tile_moved_event.entity) {
            for (warp_tile, tile_grid_coords) in &warp_tiles {
                if world_grid_coords.x == tile_grid_coords.x && world_grid_coords.y == tile_grid_coords.y && world_grid_coords.z == tile_grid_coords.z {

                    // If the warp tile has a valid level.
                    if let Some(level_iid) = &warp_tile.level_iid {
                        println!("Attempting to warp player to new level {}", level_iid);

                        // Is the entity id of the warp target valid?
                        if let Some(warp_target_entity_iid) = &warp_tile.entity_iid {

                            // Warp lock the player.
                            commands.entity(player_entity).insert(WarpLocked {
                                level_iid: level_iid.clone(),
                                warp_target_entity_iid: warp_target_entity_iid.clone(),
                                fade_out_timer: Timer::new(Duration::from_secs_f32(WARP_FADE_OUT_TIME.as_secs_f32()), TimerMode::Once)
                            });

                        }
                    }
                }
            };
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
            *level_select = LevelSelection::Iid(warp_locked.level_iid.clone());

            // Add a pending warp.
            commands.entity(entity).insert(WarpPending);
        }

        // Set the darkness level
        for mut settings in &mut palette_settings {
            settings.darkness = darkness;
        }
    }
}

fn handle_pending_warp(mut commands: Commands,
                       mut player_query: Query<(Entity, &mut WorldGridCoords, &WarpLocked, &WarpPending), Without<WarpTargetTile>>,
                       warp_target_query: Query<(&WarpTargetTile, &WorldGridCoords)>,
                       mut palette_settings: Query<&mut PaletteSwapPostProcessSettings>)
{
    for (entity, mut player_grid_coords, warp_locked, warp_pending) in player_query.iter_mut() {
        println!("Looking for warp tile id: {}", warp_locked.warp_target_entity_iid.as_str());

        // Try and find the entity that we are warping to.
        for (warp_target_tile, world_grid_coords) in &warp_target_query {
            if let Some(target_entity_iid) = &warp_target_tile.entity_iid {

                if *target_entity_iid == warp_locked.warp_target_entity_iid {
                    // Now we need to get the global grid coords for the warp target
                    println!("Found Warp tile with entity id: {}", target_entity_iid.as_str());

                    println!("world coords for warp target: {}, {}, {}", world_grid_coords.x, world_grid_coords.y, world_grid_coords.z);

                    // WARPING!
                    player_grid_coords.x = world_grid_coords.x;
                    player_grid_coords.y = world_grid_coords.y;
                    player_grid_coords.z = world_grid_coords.z;

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
        };
    }
}

#[derive(Clone, Debug, Component)]
struct WarpTargetTile {
    entity_iid: Option<EntityIid>,
}

impl Default for WarpTargetTile {
    fn default() -> Self {
        Self {
            entity_iid: None
        }
    }
}

#[derive(Clone, Debug, Default, Bundle, LdtkEntity)]
struct WarpTargetTileBundle {
    #[from_entity_instance]
    warp_target_tile: WarpTargetTile,

    #[grid_coords]
    grid_coords: GridCoords,
    world_grid_coords_required: WorldGridCoordsRequired
}

impl From<&EntityInstance> for WarpTargetTile {
    fn from(entity_instance: &EntityInstance) -> Self {
        println!("Warp Target Tile: {:?}", entity_instance);

        Self {
            entity_iid: Some(EntityIid::new(entity_instance.iid.clone()))
        }
    }
}

// Makes an entity locked to the tile grid.
#[derive(Component)]
pub struct TileLocked {
    pub position: IVec2
}

fn world_grid_coord_to_world_pixel(world_grid_coords: &WorldGridCoords) -> Vec2 {
    let grid_coords = GridCoords { x: world_grid_coords.x, y: world_grid_coords.y };
    bevy_ecs_ldtk::utils::grid_coords_to_translation(grid_coords, collision::TILE_GRID_SIZE)
}

// A direction that a TileMover could be moving in.
#[derive(PartialEq, Clone, Copy)]
enum MoveDir {
    Up,
    Down,
    Left,
    Right,
    NotMoving
}

#[derive(PartialEq, Clone, Copy)]
enum FacingDir {
    Up,
    Down,
    Left,
    Right
}

// Convert a movedir to an IVec
fn movedir_to_vec(dir: MoveDir) -> IVec2 {
    match dir {
        MoveDir::Up => IVec2::new(0, 1),
        MoveDir::Down => IVec2::new(0, -1),
        MoveDir::Left => IVec2::new(-1, 0),
        MoveDir::Right => IVec2::new(1, 0),
        MoveDir::NotMoving => IVec2::ZERO
    }
}

// Makes an entity able to move between tiles.
#[derive(Component)]
pub struct TileMover {
    want_move_dir: MoveDir, // The direction we want to move.
    moving_dir: MoveDir, // The direction we are currently moving in.
    facing_dir: FacingDir, // The direction we are facing. (the last moving_dir value that wasn't NotMoving)
    timer: Timer // Process a movement when this timer is up.
}

impl Default for TileMover {
    fn default() -> Self {
        Self {
            want_move_dir: MoveDir::NotMoving,
            moving_dir: MoveDir::NotMoving,
            facing_dir: FacingDir::Down,
            timer: Timer::new(Duration::from_secs_f32(MOVEMENT_TICK), TimerMode::Once)
        }
    }
}

// Sent whenever an entity moves to another tile.
#[derive(Event)]
struct TileMovedEvent {
    entity: Entity,
    pos: IVec2
}

// Sent when the player entity is moved into a new level that it wasn't in previously.
#[derive(Event)]
struct PlayerMovedIntoNewLevel {
    level: LevelIid
}

fn tile_movement_tick(time: Res<Time>, blocked_tile_cache: Res<BlockedTilesCache>,
                      mut tile_moved_event_writer: EventWriter<TileMovedEvent>,
                      mut query: Query<(Entity, &mut WorldGridCoords, &mut TileMover)>) {
    for (entity, mut world_grid_coords, mut tile_mover) in query.iter_mut() {
        // Increment timer.
        tile_mover.timer.tick(time.delta());

        // Actually move if the timer is finished.
        if tile_mover.timer.finished() {

            // If we only just finished the timer, then we finished moving this frame.
            // Trigger a TileMovedEvent, because this is when the tile finished actually moving to the new position.
            if tile_mover.timer.just_finished() {
                tile_moved_event_writer.send( TileMovedEvent { entity, pos: IVec2::new(world_grid_coords.x, world_grid_coords.y) });
            }

            // If we aren't moving but want to be, process that.
            if tile_mover.want_move_dir != MoveDir::NotMoving {
                // Find the grid coords that we want to move to.
                let want_move_dir_vec = movedir_to_vec(tile_mover.want_move_dir);
                let position_to_move_to = WorldGridCoords {
                    x: world_grid_coords.x + want_move_dir_vec.x as i32, 
                    y: world_grid_coords.y + want_move_dir_vec.y as i32,
                    z: world_grid_coords.z
                };

                // Determine whether or not we can move into that space.
                if (blocked_tile_cache.blocked_tile_locations.contains(&position_to_move_to)) {
                    continue;
                }

                // Move the to the position immediately. We'll animate moving to that spot.
                world_grid_coords.x = position_to_move_to.x;
                world_grid_coords.y = position_to_move_to.y;

                println!("Moving character to {}, {}", world_grid_coords.x, world_grid_coords.y);

                // Start the timer.
                tile_mover.timer.reset();

                // Store the direction we are moving.
                tile_mover.moving_dir = tile_mover.want_move_dir;

                // Store the direction we are facing.
                tile_mover.facing_dir = match tile_mover.want_move_dir {
                    MoveDir::Up => FacingDir::Up,
                    MoveDir::Down => FacingDir::Down,
                    MoveDir::Left => FacingDir::Left,
                    MoveDir::Right => FacingDir::Right,
                    MoveDir::NotMoving => tile_mover.facing_dir
                };
            }

        }
    }
}

fn tile_movement_lerp(mut query: Query<(&mut WorldGridCoords, &mut TileMover, &mut Transform)>) {
    for (mut world_grid_coords, mut tile_mover, mut transform) in query.iter_mut() {
        let move_dir_vec = movedir_to_vec(tile_mover.moving_dir);
        let moving_to_pos = world_grid_coord_to_world_pixel(&world_grid_coords);
        let moving_from_gridcoord = WorldGridCoords { x: world_grid_coords.x - move_dir_vec.x, y: world_grid_coords.y - move_dir_vec.y, z: world_grid_coords.z };
        let moving_from_pos = world_grid_coord_to_world_pixel(&moving_from_gridcoord);
        
        let z = transform.translation.z;

        // If we are moving, animate that move.
        if !tile_mover.timer.finished() {
            // How far through the timer are we?
            let timer_ratio = tile_mover.timer.elapsed_secs() / tile_mover.timer.duration().as_secs_f32();

            // TODO: make this work
            transform.translation = Vec3::new(moving_from_pos.x, moving_from_pos.y, z).lerp(Vec3::new(moving_to_pos.x, moving_to_pos.y, z), timer_ratio);
        } else {
            // Not moving anymore. 
            transform.translation = Vec3::new(moving_to_pos.x, moving_to_pos.y, z);
            tile_mover.moving_dir = MoveDir::NotMoving;
        }
    }
}

const WALK_ANIMATION_FRAMES_FORWARD: (usize, usize) = (0, 3);
const WALK_ANIMATION_FRAMES_BACKWARD: (usize, usize) = (4, 7);
const WALK_ANIMATION_FRAMES_RIGHT: (usize, usize) = (8, 11);
const WALK_ANIMATION_FRAMES_LEFT: (usize, usize) = (12, 15);

#[derive(Component)]
pub struct WalkAnim {
}

impl Default for WalkAnim {
    fn default() -> Self {
        Self {}
    }
}

fn walk_anim_control(mut query: Query<(&mut AnimationIndices, &mut AnimationTimer, &WalkAnim, &TileMover)>) {
    for (mut anim_indices, mut anim_timer, walk_anim, tile_mover) in query.iter_mut() {
        let mut indices = match tile_mover.facing_dir {
            FacingDir::Up => WALK_ANIMATION_FRAMES_BACKWARD,
            FacingDir::Down => WALK_ANIMATION_FRAMES_FORWARD,
            FacingDir::Left => WALK_ANIMATION_FRAMES_LEFT,
            FacingDir::Right => WALK_ANIMATION_FRAMES_RIGHT,
        };

        // Not moving, so stick to the first frame (standing still)
        if tile_mover.moving_dir == MoveDir::NotMoving {
            indices.1 = indices.0;
        }

        // If the indices are different, reset the animation timer.
        if anim_indices.first != indices.0 || anim_indices.last != indices.1 {
            anim_timer.time_animated = Duration::ZERO;
        }
        
        anim_indices.first = indices.0;
        anim_indices.last = indices.1;
    }
}

#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize
}

impl Default for AnimationIndices {
    fn default() -> Self {
        Self {
            first: 0, last: 1
        }
    }
}

#[derive(Component)]
pub struct AnimationTimer {
    pub time_animated: Duration,
}

impl Default for AnimationTimer {
    fn default() -> Self {
        Self {
            time_animated: Duration::ZERO
        }
    }
}

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(
        &AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlas
    )>,
) {
    for (anim_indices, mut timer, mut sprite) in &mut query {
        // Update our timer.
        timer.time_animated += time.delta();

        // Calculate what frame we should be at.
        let range = anim_indices.last - anim_indices.first;
        let frames_progressed = timer.time_animated.as_secs_f32() / ANIMATION_FRAME_TIME;

        let frames_progressed_round_down = frames_progressed.floor() as usize;

        // Calculate the current frame.
        let current_frame = if range == 0 { // Can't do mod 0, in this case there's only one frame so pick that one.
            anim_indices.first
        } else {
            anim_indices.first + (frames_progressed_round_down % (range + 1))
        };

        sprite.index = current_frame;
    }
}

#[derive(Component)]
pub struct Player {
}

impl Default for Player {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Bundle, Default, LdtkEntity)]
pub struct PlayerBundle {
    #[sprite_sheet_bundle("egg_stomp-Sheet.png", 16, 16, 16, 1, 0, 0, 0)]
    pub spritesheet_bundle: LdtkSpriteSheetBundle,
    pub anim_indices: AnimationIndices,

    pub anim_timer: AnimationTimer,
    pub tile_mover: TileMover,
    pub walk_anim: WalkAnim,
    pub player: Player,

    #[grid_coords]
    pub grid_coords: GridCoords,
    world_grid_coords_required: WorldGridCoordsRequired,

    current_level: CurrentLevel,

    #[worldly]
    wordly: Worldly
}

pub struct CharacterPlugin;
impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        // Manage Warp tiles.
        app.register_ldtk_entity::<WarpTileBundle>("Warp");
        app.register_ldtk_entity::<WarpTargetTileBundle>("WarpTarget");
        app.add_systems(FixedUpdate, (warp_player, handle_pending_warp));
        app.add_systems(FixedUpdate, (warp_fade_out));

        // Manage character movement.        
        app.register_ldtk_entity::<PlayerBundle>("Player");
        app.add_systems(FixedUpdate, (animate_sprite, move_player));
        app.add_systems(FixedUpdate, (tile_movement_tick,
                                                        tile_movement_lerp,
                                                        walk_anim_control));

        app.add_event::<TileMovedEvent>();
    }
}

fn move_player(keys: Res<ButtonInput<KeyCode>>, mut query: Query<(&Player, &mut TileMover)>) {
    for (player, mut tile_mover) in query.iter_mut() {
        tile_mover.want_move_dir = if keys.pressed(KeyCode::ArrowUp) {
            MoveDir::Up
        } else if keys.pressed(KeyCode::ArrowDown) {
            MoveDir::Down
        }
        else if keys.pressed(KeyCode::ArrowLeft) {
            MoveDir::Left
        }
        else if keys.pressed(KeyCode::ArrowRight) {
            MoveDir::Right
        } else {
            MoveDir::NotMoving
        }
    }
}