use std::time::Duration;

use bevy::{ecs::world, prelude::*, scene::ron::de, sprite::{Material2d, MaterialMesh2dBundle}, transform::components};
use bevy_ecs_tilemap::prelude::*;
use bevy_ecs_ldtk::{assets::{InternalLevels, LdtkJsonWithMetadata}, prelude::*};

use crate::{camera::PlayerFollowCameraBundle, collision::{self, BlockedTilesCache, Blocking, WorldGridCoords, WorldGridCoordsRequired}, level_loading::CurrentLevel, post_process::PaletteSwapPostProcessSettings};

const MOVEMENT_TICK: f32 = 20.0 / 60.0;
const ANIMATION_FRAME_TIME: f32 = MOVEMENT_TICK / 2.0;

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
pub struct TileMovedEvent {
    pub entity: Entity,
    pub pos: IVec2
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

                // Store the direction we are facing.
                tile_mover.facing_dir = match tile_mover.want_move_dir {
                    MoveDir::Up => FacingDir::Up,
                    MoveDir::Down => FacingDir::Down,
                    MoveDir::Left => FacingDir::Left,
                    MoveDir::Right => FacingDir::Right,
                    MoveDir::NotMoving => tile_mover.facing_dir
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

#[derive(Default, Component)]
pub struct Actor;

#[derive(Bundle, Default)]
struct ActorBundle {
    pub spritesheet_bundle: LdtkSpriteSheetBundle,
    pub anim_indices: AnimationIndices,

    pub anim_timer: AnimationTimer,
    pub tile_mover: TileMover,
    pub walk_anim: WalkAnim,

    pub grid_coords: GridCoords,
    world_grid_coords_required: WorldGridCoordsRequired,

    blocking: Blocking
}

impl LdtkEntity for ActorBundle {
    fn bundle_entity(entity_instance: &EntityInstance,
                     layer_instance: &LayerInstance,
                     tileset: Option<&Handle<Image>>,
                     tileset_definition: Option<&TilesetDefinition>,
                     asset_server: &AssetServer,
                     texture_atlases: &mut Assets<TextureAtlasLayout>) -> Self {
        
        println!("SPAWNING ACTOR");

        // Get the spritesheet filename.
        let spritesheet_path = entity_instance.get_file_path_field("Spritesheet").expect("Actor or Player should have a Spritesheet!");

        // Load/Get the spritesheet from our assets.
        let spritesheet_texture = asset_server.load(spritesheet_path);

        // Layout for the texture atlas
        let spritesheet_layout = TextureAtlasLayout::from_grid(UVec2::splat(16), 16, 1, None, None);
        let spritesheet_texture_atlas_layout = texture_atlases.add(spritesheet_layout);

        // Spawn the actor / player entity.
        ActorBundle {
            // The spritesheet and animation components.
            spritesheet_bundle: LdtkSpriteSheetBundle {
                sprite_bundle: SpriteBundle {
                    transform: Transform::default(),
                    texture: spritesheet_texture.clone(),
                    ..default()
                },
                texture_atlas: TextureAtlas {
                    layout: spritesheet_texture_atlas_layout,
                    index: 0
                }
            },
            grid_coords: GridCoords::from_entity_info(entity_instance, layer_instance),
            ..Default::default()
        }
    }
}

#[derive(Default, Component)]
pub struct Player;

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    actor_bundle: ActorBundle,

    player: Player,
    current_level: CurrentLevel,

    worldly: Worldly
}

impl LdtkEntity for PlayerBundle {
fn bundle_entity(entity_instance: &EntityInstance,
                 layer_instance: &LayerInstance,
                 tileset: Option<&Handle<Image>>,
                 tileset_definition: Option<&TilesetDefinition>,
                 asset_server: &AssetServer,
                 texture_atlases: &mut Assets<TextureAtlasLayout>) -> Self {
    
        PlayerBundle {
            actor_bundle: ActorBundle::bundle_entity(entity_instance, layer_instance, tileset, tileset_definition, asset_server, texture_atlases),
            worldly: Worldly::from_entity_info(entity_instance),
            ..Default::default()
        }
    }
}

pub struct CharacterPlugin;
impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {

        // Register the player and actor entities.
        app.register_ldtk_entity::<ActorBundle>("Actor");
        app.register_ldtk_entity::<PlayerBundle>("Player");

        // Actor creation
        //app.add_systems(FixedUpdate, actor_added);

        // Manage character movement.        
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