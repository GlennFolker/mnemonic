use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::obj::def::Obj;

#[derive(AssetCollection, Resource)]
pub struct Tiles {
    #[asset(path = "tiles/liminal/floor.obj#obj-tile")]
    pub liminal_floor: Handle<Obj>,
}
