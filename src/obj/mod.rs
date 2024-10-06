pub mod def;
pub mod loader;
pub mod parser;

use bevy::prelude::*;
use def::{MtlCollection, Obj, ObjCollection};
use loader::{MtlLoader, ObjLoader};

pub struct ObjPlugin;
impl Plugin for ObjPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ObjCollection>()
            .init_asset::<Obj>()
            .init_asset::<MtlCollection>()
            .register_asset_loader(ObjLoader)
            .register_asset_loader(MtlLoader);
    }
}
