use bevy::{asset::{Assets, Handle}, prelude::{Query, Res}};
use bevy_ecs_ldtk::assets::LdtkProject;

pub fn run_if_ldtk_project_resource_available(ldtk_project_assets: Res<Assets<LdtkProject>>,
                                          ldtk_project_entities: Query<&Handle<LdtkProject>>) -> bool {

    ldtk_project_assets.get(ldtk_project_entities.single()).is_some()
}