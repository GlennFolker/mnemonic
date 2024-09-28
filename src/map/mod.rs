pub mod tile;

use bevy::prelude::*;
use tile::{Tile, TileLoader};

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Tile>().register_asset_loader(TileLoader).init_asset::<Map>();
    }
}

#[derive(Asset, TypePath)]
pub struct Map {
    #[dependency]
    pub tiles: Vec<Handle<Tile>>,
}
