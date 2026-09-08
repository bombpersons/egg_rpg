use bevy::{math::{IVec2, Rect, Vec2}, prelude::Component};
use bevy_ecs_ldtk::ldtk::Level;

pub const TILE_GRID_SIZE: IVec2 = IVec2::new(16, 16);

// A grid coordinate in world coordinates.
// This is the canonical coordinate for everything that lies on a tile.
//
// (0, 0) is the tile at the very top-left of the LDtk world.
// X increases to the right.
// Y increases upward (bevy convention), so tiles below the world's top-left are negative.
// Z is the LDtk world_depth of the level that the tile belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Component)]
pub struct WorldGridCoords {
    pub x: i32,
    pub y: i32, 
    pub z: i32 // Layer,
}

impl WorldGridCoords {
    // The position of a level's bottom-left corner in bevy world pixels.
    // LDtk positions a level by its top-left corner with Y increasing downward,
    // so we negate and shift by the level height to land in bevy's Y-up space.
    pub fn level_origin_bevy_px(level: &Level) -> IVec2 {
        IVec2::new(level.world_x, 0 - level.world_y - level.px_hei)
    }

    // The world grid coord of a level's bottom-left corner tile.
    pub fn level_origin_grid(level: &Level) -> IVec2 {
        Self::level_origin_bevy_px(level).div_euclid(TILE_GRID_SIZE)
    }

    // The bounds of a level in bevy world pixel space.
    pub fn level_bounds_bevy(level: &Level) -> Rect {
        let min = Self::level_origin_bevy_px(level).as_vec2();
        let size = Vec2::new(level.px_wid as f32, level.px_hei as f32);
        Rect {
            min,
            max: min + size,
        }
    }

    // Convert a point in LDtk world pixel space (top-left origin, Y down) to
    // the world grid tile containing it. Pass the center of the entity:
    // (world_x + wid_px / 2, world_y + hei_px / 2).
    pub fn from_ldtk_world_px_center(center_ydown: IVec2) -> Self {
        Self {
            x: center_ydown.x.div_euclid(TILE_GRID_SIZE.x),
            y: -center_ydown.y.div_euclid(TILE_GRID_SIZE.y),
            z: 0
        }
    }

    // Set the world depth of the coord.
    pub fn with_z(mut self, z: i32) -> Self {
        self.z = z;
        self
    }

    // The center of this tile in bevy world pixel space.
    pub fn to_world_px_center(self) -> Vec2 {
        (IVec2::new(self.x, self.y) * TILE_GRID_SIZE + TILE_GRID_SIZE / 2).as_vec2()
    }
}
