use bevy::{prelude::*, utils::HashMap};
use bitflags::bitflags;

#[derive(Asset, TypePath, Deref)]
pub struct ObjCollection {
    #[deref]
    pub objects: HashMap<String, Handle<Obj>>,
}

#[derive(Asset, TypePath, Default)]
pub struct Obj {
    #[dependency]
    pub material: Handle<MtlCollection>,
    pub material_key: String,
    pub positions: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub normals: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,
}

#[derive(Asset, TypePath, Deref, DerefMut)]
pub struct MtlCollection {
    #[deref]
    pub materials: HashMap<String, Mtl>,
}

#[derive(TypePath, Default)]
pub struct Mtl {
    pub diffuse_texture: Option<Handle<Image>>,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct Cull: u8 {
        const UP = 1;
        const DOWN = 1 << 1;
        const X = 1 << 2;
        const Z = 1 << 3;
        const NEG_X = 1 << 4;
        const NEG_Z = 1 << 5;
    }
}

impl Obj {
    // TODO Calculate face culling in respect to adjacent tiles.
    pub fn calculate_culls(&mut self) {}
}
