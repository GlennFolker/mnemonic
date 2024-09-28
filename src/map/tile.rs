use std::io::Error as IoError;

use bevy::{
    asset::{
        io::Reader,
        ron::{self, de::SpannedError},
        AssetLoadError, AssetLoader, AsyncReadExt, LoadContext, ParseAssetPathError,
    },
    prelude::*,
    render::{render_asset::RenderAssetUsages, texture::ImageLoaderSettings},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Asset, TypePath)]
pub enum Tile {
    Cube(#[dependency] Handle<Image>),
}

#[derive(Serialize, Deserialize)]
pub enum TileFile {
    Cube { texture: String },
}

#[derive(Error, Debug)]
pub enum TileError {
    #[error(transparent)]
    Load(#[from] AssetLoadError),
    #[error(transparent)]
    Path(#[from] ParseAssetPathError),
    #[error(transparent)]
    Ron(#[from] SpannedError),
    #[error(transparent)]
    Io(#[from] IoError),
}

pub struct TileLoader;
impl AssetLoader for TileLoader {
    type Asset = Tile;
    type Settings = ();
    type Error = TileError;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let file = ron::de::from_bytes::<TileFile>(&{
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            bytes
        })?;

        let path = load_context.asset_path().clone();
        match file {
            TileFile::Cube { texture } => {
                let image = load_context
                    .loader()
                    .with_settings(|settings: &mut ImageLoaderSettings| settings.asset_usage = RenderAssetUsages::MAIN_WORLD)
                    .direct()
                    .load::<Image>(path.resolve_embed(&texture)?)
                    .await
                    .map_err(|e| e.error)?;

                Ok(Tile::Cube(load_context.add_loaded_labeled_asset("texture", image)))
            }
        }
    }

    #[inline]
    fn extensions(&self) -> &[&str] {
        &["tile"]
    }
}
