use crate::{
    map::{Coords, Map},
    palette::Material,
};

#[derive(Debug)]
pub struct Voxel<'a> {
    pub coord: Coords,
    pub material: &'a Material,
}

impl<'a> Voxel<'a> {
    pub fn new(coord: Coords, material: &'a Material) -> Self {
        Self { coord, material }
    }
}

pub trait CollectVoxels {
    fn collect_voxels<'a>(&'a self, map: &Map) -> Vec<Voxel<'a>>;
}
