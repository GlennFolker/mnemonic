use bevy::prelude::*;
use bitflags::bitflags;

#[derive(Asset, TypePath)]
pub struct ObjCollection {
    #[dependency]
    pub objects: Vec<Handle<Obj>>,
    #[dependency]
    pub materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Asset, TypePath, Default)]
pub struct Obj {
    #[dependency]
    pub material: Handle<StandardMaterial>,
    pub vertices: Vec<(Vec3, Vec2, Vec3)>,
    pub faces: Vec<[usize; 3]>,
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
