use std::{collections::HashMap, thread::current};

use bevy::{app::{Plugin, Update}, asset::{Assets, Handle}, color::{palettes, Color, Srgba}, ecs::query::QuerySingleError, log::tracing_subscriber::layer, math::Vec3, prelude::{Added, Bundle, Component, Entity, Parent, Query, Res, ResMut, Resource, With, Without}};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, assets::LdtkProject, prelude::LdtkFields, EntityInstance, LdtkEntity, LevelIid};

use crate::{character::Player, level_loading::CurrentLevel, post_process::PaletteSwapPostProcessSettings};

// Represents a palette to be used.
// There are 4 possible colors (like on the gameboy).
#[derive(Clone, Debug, Component)]
pub struct Palette {
    colours: [Color; 4]
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            colours: [
                Color::srgb(0.0, 0.0, 0.0),
                Color::srgb(0.3, 0.3, 0.3),
                Color::srgb(0.7,0.7, 0.7),
                Color::srgb(1.0, 1.0, 1.0)
            ]
        }
    }
}

impl From<&EntityInstance> for Palette {
    fn from(entity_instance: &EntityInstance) -> Self {
        let colours = entity_instance.get_colors_field("Colours").expect("Palette should contain colours field!");
        if colours.len() != 4 {
            panic!("A palette should contain 4 (four) colours! No more or less than that!");
        }

        Self {
            colours: colours[0..4].try_into().unwrap()
        }
    }
}

#[derive(Clone, Debug, Default, Bundle, LdtkEntity)]
struct PaletteBundle {
    #[from_entity_instance]
    palette: Palette
}

#[derive(Resource, Debug, Default)]
struct PaletteCache {
    palettes: HashMap<LevelIid, Palette>
}

// When a new palette is added, we're going to add it to our cache.
fn update_palette_cache(mut palette_cache: ResMut<PaletteCache>,
                        palettes_added: Query<(&Parent, &Palette), Added<Palette>>,
                        parent_query: Query<&Parent>,
                        level_query: Query<&LevelIid>) {

    // For each palette added, find the grandparent level and fill in our cache.
    for (parent, palette) in &palettes_added {
        if let Ok(level_parent) = parent_query.get(parent.get()) {
            if let Ok(level_iid) = level_query.get(level_parent.get()) {

                println!("Adding {:?}", palette);
                
                // Cool!
                palette_cache.palettes.insert(level_iid.clone(), palette.clone());
            }
        }
    }
}


// Update the palette swaping post processing to match whatever palette is in the level the player is in.
fn check_palette(player_query: Query<&CurrentLevel, With<Player>>,
                 palette_cache: Res<PaletteCache>,
                 mut palette_settings_query: Query<&mut PaletteSwapPostProcessSettings>,
                 level_query: Query<&LevelIid>) {
    
    // So we want to do this only when the players level is actually loaded,
    // other wise we won't be able to get the palette data and even if we could,
    // it would be too early to do so.
    
    // So we're going to get the current level, then check if it's loaded before doing anything.
    if let Ok(current_level) = player_query.get_single() {
        if let Some(current_level_iid) = &current_level.level_iid {

            // Is it loaded?
            for level_iid in &level_query {
                if level_iid == current_level_iid {
                    // Okay it's loaded.

                    // Grab the palette for this level.
                    if let Some(palette) = palette_cache.palettes.get(level_iid) {
                        
                        // Switch the palette.
                        if let Ok(mut palette_settings) = palette_settings_query.get_single_mut() {
                            for (index, colour) in palette.colours.iter().enumerate() {
                                let linear = colour.to_linear();
                                palette_settings.colours[index] = Vec3::new(linear.red, linear.green, linear.blue);
                            }
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
        app.init_resource::<PaletteCache>();

        app.register_ldtk_entity::<PaletteBundle>("Palette");
        
        app.add_systems(Update, (update_palette_cache, check_palette));
    }
}