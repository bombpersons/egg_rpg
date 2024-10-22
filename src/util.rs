use bevy::math::IVec2;
use bevy_ecs_ldtk::ldtk::Level;

const TILE_GRID_SIZE: IVec2 = IVec2::new(16, 16);

fn get_level_origin_grid_coord(level: &Level) {
    // Yay, this is the adjustment required *phew*
    IVec2::new(level.world_x, 0 - level.world_y - level.px_hei) / TILE_GRID_SIZE;
}