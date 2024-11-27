use bevy::{app::{App, Plugin}, asset::{AssetServer, Assets, Handle}, audio::{AudioSource, AudioSourceBundle, PlaybackMode, PlaybackSettings}, prelude::{Added, Bundle, Commands, Component, Image, Query}, sprite::TextureAtlasLayout};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, ldtk::{LayerInstance, TilesetDefinition}, prelude::{LdtkEntity, LdtkFields}, EntityInstance};

#[derive(Default, Component)]
struct BGM {
}

#[derive(Default, Bundle)]
struct BGMBundle {
    bgm: BGM,
    audio_bundle: AudioSourceBundle
}

impl LdtkEntity for BGMBundle {
    fn bundle_entity(
            entity_instance: &EntityInstance,
            layer_instance: &LayerInstance,
            tileset: Option<&Handle<Image>>,
            tileset_definition: Option<&TilesetDefinition>,
            asset_server: &AssetServer,
            texture_atlases: &mut Assets<TextureAtlasLayout>,
        ) -> Self {
        
        // Get the file path to the music.
        let file_path = entity_instance.get_file_path_field("MusicPath").expect("BGM entity should have a path for the music track.");
        
        println!("Loading background music: {}", file_path);

        // Try and load it.
        let music_handle = asset_server.load(file_path);
        
        // Create the bundle.
        BGMBundle {
            audio_bundle: AudioSourceBundle {
                source: music_handle,
                settings: PlaybackSettings { 
                    mode: PlaybackMode::Loop,
                    paused: false,
                    ..Default::default()
                }
            },
            ..Default::default()
        }
    }
}

pub struct AudioPlugin; 
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.register_ldtk_entity::<BGMBundle>("BGM");
    }
}
