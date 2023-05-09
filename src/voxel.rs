use crate::{
    map::{Coords, Map},
    palette::Material,
    shape::Box3D,
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

pub fn voxels_from_shape<const B: usize, const H: usize>(
    shape: Box3D<Option<&Material>, B, H>,
    origin: Coords,
) -> Vec<Voxel> {
    (0..B)
        .flat_map(move |x| {
            (0..B).flat_map(move |y| {
                (0..H).flat_map(move |z| {
                    shape[H - 1 - z][y][x].map(|material| {
                        let coords = Coords {
                            x: origin.x * B as i32 + x as i32,
                            y: origin.y * B as i32 + y as i32,
                            z: origin.z * H as i32 + z as i32,
                        };
                        Voxel::new(coords, material)
                    })
                })
            })
        })
        .collect()
}

pub fn voxels_from_uniform_shape<const B: usize, const H: usize>(
    shape: Box3D<bool, B, H>,
    origin: Coords,
    material: &Material,
) -> Vec<Voxel> {
    let shape = shape.map(|slice| {
        slice.map(|col| col.map(|include| if include { Some(material) } else { None }))
    });
    voxels_from_shape(shape, origin)
}
