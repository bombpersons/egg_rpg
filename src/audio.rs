use std::time::Duration;

use bevy::{app::{App, FixedUpdate, Plugin, Update}, asset::{AssetServer, Assets, Handle}, audio::{AudioSink, AudioSinkPlayback, AudioSource, AudioSourceBundle, PlaybackMode, PlaybackSettings}, prelude::{Added, Bundle, Commands, Component, Entity, Event, EventReader, EventWriter, Image, IntoSystemConfigs, Query, Res, With}, scene::ron::de, sprite::TextureAtlasLayout, time::Time};
use bevy_ecs_ldtk::{app::LdtkEntityAppExt, assets::{LdtkProject, LevelMetadataAccessor}, ldtk::{LayerInstance, TilesetDefinition}, prelude::{LdtkEntity, LdtkFields}, EntityIid, EntityInstance};

use crate::{character::Player, level_loading::{CurrentLevel, CurrentLevelChangedEvent}, post_process::PaletteSwapPostProcessSettings, util::run_if_ldtk_project_resource_available};

#[derive(Default, Component)]
struct BGM {
}

#[derive(Default, Bundle)]
struct BGMPlayerBundle {
    bgm: BGM,
    audio_bundle: AudioSourceBundle
}

enum FadeStyle {
    FadeIn, FadeOut
}

impl Default for FadeStyle {
    fn default() -> Self {
        return FadeStyle::FadeIn;
    }
}

#[derive(Default, Component)]
struct Fade {
    fade_style: FadeStyle,
    time: Duration,
    time_used: Duration
}

fn enact_fade(mut commands: Commands,
              delta_time: Res<Time>,
              mut fade_query: Query<(Entity, &mut Fade, &AudioSink)>) {

    let delta = delta_time.delta();
    
    // For each fade increment their timer and update the audio sink settings to control the volume.
    for (entity, mut fade, sink) in &mut fade_query {
        fade.time_used += delta;

        // Calculate what the new volume should be.
        sink.set_volume(match fade.fade_style {
            FadeStyle::FadeIn => {
                (fade.time_used.as_secs_f32() / fade.time.as_secs_f32()).clamp(0.0, 1.0)
            },
            FadeStyle::FadeOut => {
                (1.0 - (fade.time_used.as_secs_f32() / fade.time.as_secs_f32())).clamp(0.0, 1.0)
            }
        });

        println!("{}", sink.volume());

        // If we have used up our time, remove the entity for a fade out, remove the fade component for a fade in.
        if fade.time_used >= fade.time {
            match fade.fade_style {
                FadeStyle::FadeIn => {
                    commands.entity(entity).remove::<Fade>();
                },
                FadeStyle::FadeOut => {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

#[derive(Event)]
enum BGMControlEvent {
    FadeTo(Handle<AudioSource>, Duration),
    Change(Handle<AudioSource>),
    Stop
}

fn bgm_change(mut commands: Commands,
              bgm_query: Query<Entity, With<BGM>>,
              mut bgm_control_event_reader: EventReader<BGMControlEvent>) {

    for event in bgm_control_event_reader.read() {
        match event {
            BGMControlEvent::Change(audio_source) => {
                // Remove all other bgm players and add a new one.
                for bgm_entity in &bgm_query {
                    commands.entity(bgm_entity).despawn();
                }

                // A new player.
                commands.spawn(BGMPlayerBundle {
                    audio_bundle: AudioSourceBundle {
                        source: audio_source.clone(),
                        settings: PlaybackSettings {
                            mode: PlaybackMode::Loop,
                            paused: false,
                            ..Default::default()
                        }
                    },
                    ..Default::default()
                });
            },
            BGMControlEvent::FadeTo(audio_source, duration) => {
                // Fade out all the current bgm players.
                for bgm_entity in &bgm_query {
                    commands.entity(bgm_entity).insert(Fade {
                        fade_style: FadeStyle::FadeOut,
                        time: *duration,
                        ..Default::default()
                    });
                }

                // Insert a bgm player that has a fade in with the same duration.
                commands.spawn(BGMPlayerBundle {
                    audio_bundle: AudioSourceBundle {
                        source: audio_source.clone(),
                        settings: PlaybackSettings {
                            mode: PlaybackMode::Loop,
                            paused: false,
                            ..Default::default()
                        }
                    },
                    ..Default::default()
                }).insert(Fade {
                    fade_style: FadeStyle::FadeIn,
                    time: *duration,
                    ..Default::default()
                });
            }
            _ => {}
        }
    }
}

fn check_bgm(mut commands: Commands,
             player_query: Query<(&EntityIid, &CurrentLevel), With<Player>>,
             mut bgm_control_event_writer: EventWriter<BGMControlEvent>,
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

                        // Send a message to change the background music.
                        bgm_control_event_writer.send(BGMControlEvent::FadeTo(bgm_handle, Duration::from_secs(1)));
                    }
                
                }
            }
        }
    }
}

pub struct AudioPlugin; 
impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BGMControlEvent>();
        app.add_systems(Update, (enact_fade, bgm_change, check_bgm.run_if(run_if_ldtk_project_resource_available)));
    }
}
