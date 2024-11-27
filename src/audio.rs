use bevy::{app::{App, FixedUpdate, Plugin, Update}, asset::{AssetServer, Assets, Handle}, audio::{AudioSource, AudioSourceBundle, PlaybackMode, PlaybackSettings}, prelude::{Added, Bundle, Commands, Component, Entity, EventReader, Image, IntoSystemConfigs, Query, Res, With}, sprite::TextureAtlasLayout};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, assets::{LdtkProject, LevelMetadataAccessor}, ldtk::{LayerInstance, TilesetDefinition}, prelude::{LdtkEntity, LdtkFields}, EntityIid, EntityInstance};

use crate::{character::Player, level_loading::{CurrentLevel, CurrentLevelChangedEvent}, post_process::PaletteSwapPostProcessSettings, util::run_if_ldtk_project_resource_available};

#[derive(Default, Component)]
struct BGM {
}

#[derive(Default, Bundle)]
pub struct BGMBundle {
    bgm: BGM,
    audio_bundle: AudioSourceBundle
}

fn check_bgm(mut commands: Commands,
             player_query: Query<(&EntityIid, &CurrentLevel), With<Player>>,
             mut bgm_query: Query<(Entity), With<BGM>>,
             mut current_level_event_reader: EventReader<CurrentLevelChangedEvent>,
             ldtk_project_entities: Query<&Handle<LdtkProject>>,
             ldtk_project_assets: Res<Assets<LdtkProject>>,
             asset_server: Res<AssetServer>) {

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
                if entity_iid == player_entity_iid {

                    // The player entered a new level and it loaded.
                    let level = ldtk_project.get_raw_level_by_iid(level_iid.get()).expect("Level supposedly loaded should exist!");
                    if let Ok(bgm_path) = level.get_file_path_field("BGM") {

                        // Let's load the this bgm track and play it.
                        let bgm_handle = asset_server.load::<AudioSource>(bgm_path);

                        // If there's already a bgm player, remove it first.
                        if let Ok(entity) = bgm_query.get_single() {
                            commands.entity(entity).despawn();
                        }

                        // Make a new bgm player.
                        commands.spawn(BGMBundle {
                            audio_bundle: AudioSourceBundle { 
                                source: bgm_handle,
                                settings: PlaybackSettings {
                                    mode: PlaybackMode::Loop,
                                    paused: false,
                                    ..Default::default()
                                }
                            },
                            ..Default::default()
                        });
                    }
                
                }
            }
        }
    }
}

pub struct AudioPlugin; 
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, check_bgm.run_if(run_if_ldtk_project_resource_available));
    }
}
