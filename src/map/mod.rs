use bevy::prelude::*;

use crate::obj::def::Obj;

pub struct MapPlugin;
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Map>();
    }
}

#[derive(Asset, TypePath)]
pub struct Map {
    #[dependency]
    pub tiles: Vec<Handle<Obj>>,
}
