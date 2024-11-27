use std::{collections::HashMap, thread::current};

use bevy::{app::{Plugin, Update}, asset::{Assets, Handle}, color::{palettes, Color, Srgba}, ecs::query::QuerySingleError, log::tracing_subscriber::layer, math::Vec3, prelude::{Added, Bundle, Component, Entity, EventReader, IntoSystemConfigs, Parent, Query, Res, ResMut, Resource, With, Without}};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, assets::{LdtkProject, LevelMetadataAccessor}, prelude::LdtkFields, EntityIid, EntityInstance, LdtkEntity, LevelIid};

use crate::{character::Player, level_loading::{CurrentLevel, CurrentLevelChangedEvent}, post_process::PaletteSwapPostProcessSettings, util::run_if_ldtk_project_resource_available};

// impl Default for Palette {
//     fn default() -> Self {
//         Self {
//             colours: [
//                 Color::srgb(0.0, 0.0, 0.0),
//                 Color::srgb(0.3, 0.3, 0.3),
//                 Color::srgb(0.7,0.7, 0.7),
//                 Color::srgb(1.0, 1.0, 1.0)
//             ]
//         }
//     }
// }

// Update the palette swaping post processing to match whatever palette is in the level the player is in.
fn check_palette(player_query: Query<(&EntityIid, &CurrentLevel), With<Player>>,
                 mut palette_settings_query: Query<&mut PaletteSwapPostProcessSettings>,
                 mut current_level_event_reader: EventReader<CurrentLevelChangedEvent>,
                 ldtk_project_entities: Query<&Handle<LdtkProject>>,
                 ldtk_project_assets: Res<Assets<LdtkProject>>) {

    // Get the ldtk project so that we can get the level data from it.
    let ldtk_project = ldtk_project_assets.get(ldtk_project_entities.single())
                        .expect("LdtkProject should be loaded when level is spawned");
    
    // Get the player entity
    if let Ok((player_entity_iid, _)) = player_query.get_single() {

        // So we want to do this only when the players level is actually loaded,
        // other wise we won't be able to get the palette data and even if we could,
        // it would be too early to do so.
        for event in current_level_event_reader.read() {
            if let CurrentLevelChangedEvent::ChangedAndLoaded(entity_iid, level_iid) = event {

                // Does this apply to the player?
                if entity_iid == player_entity_iid {

                    // Cool! So the player has entered a new level AND importantly it's actually been loaded too!
                    let level = ldtk_project.data().get_raw_level_by_iid(level_iid.get()).expect("Level supposedly loaded should exist!");
                    let colours : [Color; 4] = level.get_colors_field("Palette").expect("All levels should have a palette field!")[0..4].try_into().unwrap();

                    // Get the palette settings entity to change the colors.
                    if let Ok(mut palette_settings) = palette_settings_query.get_single_mut() {
                        for (index, colour) in colours.iter().enumerate() {
                            let linear = colour.to_linear();
                            palette_settings.colours[index] = Vec3::new(linear.red, linear.green, linear.blue);
                        }
                    }
                }

            }
        }
    }
}

pub struct PalettePlugin;
impl Plugin for PalettePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, check_palette.run_if(run_if_ldtk_project_resource_available));
    }
}