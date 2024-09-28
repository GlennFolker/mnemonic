pub mod def;
pub mod loader;

use bevy::prelude::*;
use def::{Mtl, Obj, ObjCollection};
use loader::ObjLoader;

pub struct ObjPlugin;
impl Plugin for ObjPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ObjCollection>()
            .init_asset::<Obj>()
            .init_asset::<Mtl>()
            .register_asset_loader(ObjLoader);
    }
}
