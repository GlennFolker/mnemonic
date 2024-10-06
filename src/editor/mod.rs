use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    prelude::*,
};
use nonmax::NonMaxU8;

use crate::{content::TileTexture, map::Map, GameState};

pub struct EditorPlugin;
impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Editor), init_editor_map);
    }
}

fn init_editor_map(
    mut commands: Commands,
    mut maps: ResMut<Assets<Map>>,
    tile_texture: Res<TileTexture>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        maps.add(Map {
            tile_set: vec!["tiles/liminal/floor.obj".into()],
            tiles: [0].into_iter().map(|id| NonMaxU8::new(id)).collect(),
            size: UVec3::new(2, 1, 1),
        }),
        materials.add(StandardMaterial {
            reflectance: 0.0,
            base_color_texture: Some(tile_texture.atlas.clone_weak()),
            ..default()
        }),
        TransformBundle::default(),
        VisibilityBundle::default(),
    ));

    let cam_pos = Vec3::new(-20.0, 20.0, 20.0);
    commands.spawn((
        Camera3dBundle {
            camera: Camera { hdr: true, ..default() },
            projection: Projection::Orthographic(OrthographicProjection {
                scale: 0.025,
                ..default()
            }),
            transform: Transform::from_translation(cam_pos).looking_at(Vec3::ZERO, Vec3::Y),
            tonemapping: Tonemapping::None,
            ..default()
        },
        BloomSettings::NATURAL,
    ));

    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_translation(cam_pos + Vec3::new(7.0, 10.0, 5.0)).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
