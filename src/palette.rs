use bevy::{app::{Plugin, Update}, asset::{Assets, Handle}, color::{palettes, Color, Srgba}, ecs::query::QuerySingleError, log::tracing_subscriber::layer, math::Vec3, prelude::{Bundle, Component, Entity, Parent, Query, Res, With, Without}};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, assets::LdtkProject, prelude::LdtkFields, EntityInstance, LdtkEntity, LevelIid};

use crate::{character::Player, collision::CurrentLevel, post_process::PaletteSwapPostProcessSettings};

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

// Update the palette swaping post processing to match whatever palette is in the level the player is in.
fn check_palette(player_query: Query<&CurrentLevel, With<Player>>,
                 palette_query: Query<(&Parent, &Palette)>,
                 parent_query: Query<&Parent, Without<Palette>>,
                 mut palette_settings_query: Query<&mut PaletteSwapPostProcessSettings>,
                 level_query: Query<&LevelIid>) {
    
    match player_query.get_single() {
        Ok(player_level) => {
            if let Some(player_level_iid) = &player_level.level_iid {
                // Awesome the player is in some level.
                // Now we need to find the palette entity that's in this level.
                for (parent, palette) in &palette_query {
                    let layer_entity = parent.get();
                    if let Ok(level_parent) = parent_query.get(layer_entity) {
                        if let Ok(palette_level_iid) = level_query.get(level_parent.get()) {

                            // We have the level iid the palette is associated with, does it match?
                            if player_level_iid == palette_level_iid {

                                // It matches, so switch over the colors to match this palette.
                                if let Ok(mut palette_settings) = palette_settings_query.get_single_mut() {
                                    let colour_one = palette.colours[0].to_linear();
                                    palette_settings.colour_one = Vec3::new(colour_one.red, colour_one.green, colour_one.blue);

                                    let colour_two = palette.colours[1].to_linear();
                                    palette_settings.colour_two = Vec3::new(colour_two.red, colour_two.green, colour_two.blue);

                                    let colour_three = palette.colours[2].to_linear();
                                    palette_settings.colour_three = Vec3::new(colour_three.red, colour_three.green, colour_three.blue);

                                    let colour_four = palette.colours[3].to_linear();
                                    palette_settings.colour_four = Vec3::new(colour_four.red, colour_four.green, colour_four.blue);
                                }
                            }
                        }
                    }
                }

            }
        },
        Err(QuerySingleError::MultipleEntities(msg)) => {
            println!("More than one player?");
        },
        Err(QuerySingleError::NoEntities(msg)) => {
            println!("No player?");
        }
    }
}

pub struct PalettePlugin;
impl Plugin for PalettePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_ldtk_entity::<PaletteBundle>("Palette");
        
        app.add_systems(Update, (check_palette));
    }
}