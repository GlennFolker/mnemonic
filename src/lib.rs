pub mod map;
pub mod obj;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
        render_asset::RenderAssetUsages,
    },
};
use bevy_asset_loader::prelude::*;
use iyes_progress::prelude::*;
use map::MapPlugin;
use obj::{
    def::{Mtl, Obj},
    ObjPlugin,
};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum GameState {
    #[default]
    Loading,
    Running,
}

#[derive(AssetCollection, Resource)]
pub struct GameAssets {
    #[asset(path = "tiles/liminal/floor.obj#obj-tile")]
    pub floor: Handle<Obj>,
}

#[inline]
pub fn run() {
    App::new()
        .add_plugins((DefaultPlugins.set(ImagePlugin::default_nearest()), MapPlugin, ObjPlugin))
        .init_state::<GameState>()
        .add_plugins(ProgressPlugin::new(GameState::Loading).continue_to(GameState::Running))
        .add_loading_state(LoadingState::new(GameState::Loading).load_collection::<GameAssets>())
        .add_systems(OnEnter(GameState::Running), print)
        .run();
}

fn print(
    mut commands: Commands,
    assets: Res<GameAssets>,
    objs: Res<Assets<Obj>>,
    mtls: Res<Assets<Mtl>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let obj = objs.get(&assets.floor).unwrap();
    let mtl = mtls.get(&obj.material).unwrap();

    commands.spawn(PbrBundle {
        mesh: meshes.add(
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
        ),
        material: materials.add(StandardMaterial {
            base_color_texture: mtl.diffuse_texture.as_ref().map(Handle::clone_weak),
            unlit: true,
            ..default()
        }),
        ..default()
    });

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.0, -2.0).looking_at(Vec3::ZERO, Dir3::Y),
        tonemapping: Tonemapping::None,
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Dir3::Y),
        ..default()
    });
}
