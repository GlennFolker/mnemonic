use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    utils::HashMap,
};
use nonmax::NonMaxU8;

use crate::{
    content::{TileTexture, Tiles},
    obj::def::{MtlCollection, Obj},
    GameState,
};

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
pub enum EditMode {
    #[default]
    Tile,
}

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<EditMode>()
            .init_asset::<Map>()
            .init_resource::<MapMeshes>()
            .add_systems(
                PostUpdate,
                (update_map_mesh, sync_map_mesh)
                    .chain_ignore_deferred()
                    .run_if(not(in_state(GameState::Loading))),
            );
    }
}

#[derive(Asset, TypePath)]
pub struct Map {
    pub tile_set: Vec<String>,
    pub tiles: Vec<Option<NonMaxU8>>,
    pub size: UVec3,
}

impl Map {
    #[inline]
    pub fn iter_tiles<'a>(
        &'a self,
        tiles: &'a Tiles,
        tile_assets: &'a Assets<Obj>,
    ) -> impl Iterator<Item = (UVec3, &'a Obj)> {
        let [width, length, ..] = self.size.to_array();
        self.tiles.iter().enumerate().filter_map(move |(pos, &tile)| {
            Some((
                UVec3::new(
                    pos as u32 % width,
                    (pos as u32 / width) % length,
                    pos as u32 / (width * length),
                ),
                tile_assets.get(tiles.get(self.tile_set.get(tile?.get() as usize)?)?)?,
            ))
        })
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct MapMeshes(pub HashMap<AssetId<Map>, Handle<Mesh>>);

pub fn sync_map_mesh(
    mut commands: Commands,
    maps: Query<(Entity, &Handle<Map>), Or<(Changed<Handle<Map>>, Without<Handle<Mesh>>)>>,
    mut removed: RemovedComponents<Handle<Map>>,
    map_meshes: Res<MapMeshes>,
) {
    for (e, map) in &maps {
        let Some(mesh) = map_meshes.get(&map.id()) else { continue };
        commands.entity(e).insert(mesh.clone_weak());
    }

    for e in removed.read() {
        commands.entity(e).remove::<Handle<Mesh>>();
    }
}

pub fn update_map_mesh(
    mut events: EventReader<AssetEvent<Map>>,
    maps: Res<Assets<Map>>,
    tiles: Res<Tiles>,
    tile_textures: Res<TileTexture>,
    tile_assets: Res<Assets<Obj>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
    materials: Res<Assets<MtlCollection>>,
    mut map_meshes: ResMut<MapMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let layout = layouts.get(&tile_textures.layout).unwrap();
    for &e in events.read() {
        match e {
            AssetEvent::Unused { id } | AssetEvent::Removed { id } => {
                map_meshes.remove(&id);
            }
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(map) = maps.get(id) else { continue };
                let (handle, mesh) = match map_meshes.remove(&id) {
                    None => (
                        None,
                        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD),
                    ),
                    Some(handle) => match meshes.remove(&handle) {
                        None => (
                            None,
                            Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD),
                        ),
                        Some(mesh) => (Some(handle), mesh),
                    },
                };

                let mut offsets = Vec::new();
                let mut offset = 0u32;

                let mesh = mesh
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_POSITION,
                        map.iter_tiles(&tiles, &tile_assets)
                            .flat_map(|(tile_pos, tile)| {
                                offsets.push(offset);
                                offset += tile.positions.len() as u32;

                                tile.positions.iter().map(move |&pos| pos + tile_pos.as_vec3())
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_UV_0,
                        map.iter_tiles(&tiles, &tile_assets)
                            .flat_map(|(.., tile)| {
                                let material = materials.get(&tile.material).unwrap();
                                let rect = layout.textures[layout
                                    .get_texture_index(material[&tile.material_key].diffuse_texture.as_ref().unwrap().id())
                                    .unwrap()]
                                .as_rect();

                                let min = rect.min / layout.size.as_vec2();
                                let scl = rect.max / layout.size.as_vec2() - min;

                                tile.uvs.iter().map(move |&uv| min + uv * scl)
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_attribute(
                        Mesh::ATTRIBUTE_NORMAL,
                        map.iter_tiles(&tiles, &tile_assets)
                            .flat_map(|(.., tile)| tile.normals.iter().copied())
                            .collect::<Vec<_>>(),
                    )
                    .with_inserted_indices(Indices::U32(
                        map.iter_tiles(&tiles, &tile_assets)
                            .enumerate()
                            .flat_map(|(id, (.., tile))| {
                                let offset = offsets[id];
                                tile.faces
                                    .iter()
                                    .flat_map(move |&[a, b, c]| [a as u32 + offset, b as u32 + offset, c as u32 + offset])
                            })
                            .collect(),
                    ));

                map_meshes.insert_unique_unchecked(id, match handle {
                    None => meshes.add(mesh),
                    Some(handle) => {
                        meshes.insert(&handle, mesh);
                        handle
                    }
                });
            }
            _ => {}
        }
    }
}
