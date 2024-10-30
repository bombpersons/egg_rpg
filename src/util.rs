use bevy::{asset::{Assets, Handle}, math::IVec2, prelude::{Query, Res}};
use bevy_ecs_ldtk::{assets::LdtkProject, ldtk::Level};

const TILE_GRID_SIZE: IVec2 = IVec2::new(16, 16);

pub fn get_level_origin_grid_coord(level: &Level) {
    // Yay, this is the adjustment required *phew*
    IVec2::new(level.world_x, 0 - level.world_y - level.px_hei) / TILE_GRID_SIZE;
}

pub fn run_if_ldtk_project_resource_available(ldtk_project_assets: Res<Assets<LdtkProject>>,
                                          ldtk_project_entities: Query<&Handle<LdtkProject>>) -> bool {

    ldtk_project_assets.get(ldtk_project_entities.single()).is_some()
}