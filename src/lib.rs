pub mod content;
pub mod map;
pub mod obj;

use avian3d::prelude::*;
use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
        render_asset::RenderAssetUsages,
    },
};
use bevy_asset_loader::prelude::*;
use content::Tiles;
use iyes_progress::prelude::*;
use map::MapPlugin;
use obj::{def::Obj, ObjPlugin};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    Loading,
    Running,
}

#[inline]
pub fn run() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            PhysicsPlugins::default().with_length_unit(2.0),
            #[cfg(feature = "dev")]
            PhysicsDebugPlugin::default(),
            MapPlugin,
            ObjPlugin,
        ))
        .init_state::<GameState>()
        .add_plugins(ProgressPlugin::new(GameState::Loading).continue_to(GameState::Running))
        .add_loading_state(LoadingState::new(GameState::Loading).load_collection::<Tiles>())
        .add_systems(OnEnter(GameState::Running), init)
        .add_systems(Update, pan.run_if(in_state(GameState::Running)))
        .run();
}

fn init(
    mut commands: Commands,
    tiles: Res<Tiles>,
    mut ambient: ResMut<AmbientLight>,
    objs: Res<Assets<Obj>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    ambient.brightness = 1200.0;

    let obj = objs.get(&tiles.liminal_floor).unwrap();
    let mesh = meshes.add(
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                VertexAttributeValues::Float32x3(obj.vertices.iter().map(|v| v.0.to_array()).collect()),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_UV_0,
                VertexAttributeValues::Float32x2(obj.vertices.iter().map(|v| v.1.to_array()).collect()),
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_NORMAL,
                VertexAttributeValues::Float32x3(obj.vertices.iter().map(|v| v.2.to_array()).collect()),
            )
            .with_inserted_indices(Indices::U32(
                obj.faces
                    .iter()
                    .flat_map(|&[a, b, c]| [a as u32, b as u32, c as u32])
                    .collect(),
            )),
    );

    for [x, y, z] in (-2..=2).flat_map(|x| (-2..=2).map(move |z| [x as f32, 0.0, z as f32])) {
        commands.spawn(PbrBundle {
            mesh: mesh.clone(),
            material: obj.material.clone_weak(),
            transform: Transform::from_xyz(x, y, z),
            ..default()
        });
    }

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

fn pan(mut camera: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    for mut trns in &mut camera {
        trns.rotate_around(Vec3::ZERO, Quat::from_axis_angle(Vec3::Y, (time.delta_seconds() * 24.0).to_radians()));
    }
}
