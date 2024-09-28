use bevy::prelude::*;
use bitflags::bitflags;

#[derive(Asset, TypePath)]
pub struct ObjCollection {
    #[dependency]
    pub objects: Vec<Handle<Obj>>,
    #[dependency]
    pub materials: Vec<Handle<Mtl>>,
}

#[derive(Asset, TypePath, Default)]
pub struct Obj {
    #[dependency]
    pub material: Handle<Mtl>,
    pub vertices: Vec<(Vec3, Vec2, Vec3)>,
    pub faces: Vec<[usize; 3]>,
}

#[derive(Asset, TypePath, Default)]
pub struct Mtl {
    #[dependency]
    pub diffuse_texture: Option<Handle<Image>>,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct Cull: u8 {
        const UP = 1;
        const DOWN = 1 << 1;
    }
}
