pub mod def;
pub mod loader;
pub mod parser;

use bevy::prelude::*;
use def::{Obj, ObjCollection};
use loader::ObjLoader;

pub struct ObjPlugin;
impl Plugin for ObjPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ObjCollection>()
            .init_asset::<Obj>()
            .register_asset_loader(ObjLoader);
    }
}
