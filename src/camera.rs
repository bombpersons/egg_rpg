use bevy::{prelude::*, render::{render_resource::{TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, Extent3d}, camera::{RenderTarget, Viewport, ScalingMode}}, window::{PrimaryWindow, WindowResized}};

use crate::character::Player;

// A camera that only draws a certain area of pixels.
// Uses a render target to draw to, then scales that up to whatever size is required.
#[derive(Component)]
struct PixelCamera {
    size: UVec2,
    position: Vec2
}

impl Default for PixelCamera {
    fn default() -> Self {
        Self {
            size: UVec2::new(160, 144),
            position: Vec2::new(160.0, 160.0),
        }
    }
}

impl PixelCamera {
    fn new(size: UVec2, position: Vec2) -> Self {
        Self {
            size,
            position
        }
    }
}

// If the pixel camera size changes, the render target needs to be changed.
fn pixel_camera_changed(mut query: Query<(&mut OrthographicProjection, &mut Transform, &PixelCamera), Changed<PixelCamera>>) {
    for (mut projection, mut transform, pixel_camera) in query.iter_mut() {
        projection.scaling_mode = ScalingMode::FixedVertical(pixel_camera.size.y as f32);
        transform.translation = Vec3::new(pixel_camera.position.x, pixel_camera.position.y, 0.0);
    }
}

// A bundle to make creating a pixel camera entity easier.
#[derive(Bundle)]
pub struct PixelCameraBundle {
    cam2d_bundle: Camera2dBundle,
    pixel_camera: PixelCamera
}

impl Default for PixelCameraBundle {
    fn default() -> Self {
        Self {
            cam2d_bundle: Camera2dBundle {
                projection: OrthographicProjection {
                    scaling_mode: ScalingMode::FixedVertical(144.0 * 1.0),
                    far: 1000.0,
                    near: -1000.0,
                    ..default()
                },
                ..default()
            },
            pixel_camera: default()
        }
    }
}

// A plugin for adding the required systems.
pub struct PixelCameraPlugin;
impl Plugin for PixelCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, pixel_camera_changed);
    }
}

#[derive(Component)]
struct FollowPlayer;
fn follow_player(player: Query<(&Player, &Transform), Without<FollowPlayer>>, mut query: Query<(&mut Transform, &FollowPlayer), Without<Player>>) {
    if let Ok((_, player_transform)) = player.get_single() {
        for (mut transform, _) in query.iter_mut() {
            transform.translation = player_transform.translation;
        }
    }
}

#[derive(Bundle)]
pub struct PlayerFollowCameraBundle {
    pixel_camera_bundle: PixelCameraBundle,
    follow_player: FollowPlayer
}

impl Default for PlayerFollowCameraBundle {
    fn default() -> Self {
        Self {
            pixel_camera_bundle: default(),
            follow_player: FollowPlayer
        }
    }
}

pub struct PlayerFollowCameraPlugin;
impl Plugin for PlayerFollowCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, follow_player);
    }
}