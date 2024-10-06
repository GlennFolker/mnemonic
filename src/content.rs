use bevy::{
    ecs::system::SystemState,
    prelude::*,
    render::{render_asset::RenderAssetUsages, render_resource::TextureFormat, renderer::RenderDevice},
    utils::HashMap,
};
use bevy_asset_loader::prelude::*;

use crate::obj::def::{MtlCollection, Obj};

#[derive(AssetCollection, Resource, Deref)]
pub struct Tiles {
    #[asset(paths("tiles/liminal/floor.obj#obj:tile"), collection(mapped, typed))]
    pub tiles: HashMap<String, Handle<Obj>>,
}

#[derive(Resource)]
pub struct TileTexture {
    pub layout: Handle<TextureAtlasLayout>,
    pub atlas: Handle<Image>,
}

impl FromWorld for TileTexture {
    fn from_world(world: &mut World) -> Self {
        let (tiles, objs, mut materials, mut images, mut layouts, render_device) = SystemState::<(
            Res<Tiles>,
            Res<Assets<Obj>>,
            ResMut<Assets<MtlCollection>>,
            ResMut<Assets<Image>>,
            ResMut<Assets<TextureAtlasLayout>>,
            Res<RenderDevice>,
        )>::new(world)
        .get_mut(world);

        let mut used_images = HashMap::new();
        for obj in tiles.values() {
            let obj = objs.get(obj).unwrap();
            let mtl = materials.get_mut(&obj.material).unwrap();
            for mtl in mtl.values_mut() {
                if let Some(ref mut diffuse_texture) = mtl.diffuse_texture {
                    used_images.entry(diffuse_texture.id()).or_insert_with(|| {
                        let handle = std::mem::replace(diffuse_texture, diffuse_texture.clone_weak());
                        images.remove(&handle).unwrap()
                    });
                }
            }
        }

        let mut builder = TextureAtlasBuilder::default();
        builder
            .max_size(UVec2::splat(render_device.limits().max_texture_dimension_2d))
            .format(TextureFormat::Rgba8UnormSrgb)
            .auto_format_conversion(true)
            .padding(UVec2::splat(4));

        for (&id, image) in &used_images {
            builder.add_texture(Some(id), &image);
        }

        let (layout, mut atlas) = builder.build().unwrap();
        atlas.asset_usage = RenderAssetUsages::RENDER_WORLD;

        Self {
            layout: layouts.add(layout),
            atlas: images.add(atlas),
        }
    }
}
