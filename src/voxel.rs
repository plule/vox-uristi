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

pub fn voxels_from_shape<const N: usize>(
    shape: Box3D<Option<&Material>, N>,
    origin: Coords,
) -> Vec<Voxel> {
    (0..N)
        .flat_map(move |x| {
            (0..N).flat_map(move |y| {
                (0..N).flat_map(move |z| {
                    shape[N - 1 - z][y][x].map(|material| {
                        let coords = Coords {
                            x: origin.x * 3 + x as i32,
                            y: origin.y * 3 + y as i32,
                            z: origin.z * 3 + z as i32,
                        };
                        Voxel::new(coords, material)
                    })
                })
            })
        })
        .collect()
}

pub fn voxels_from_uniform_shape<const N: usize>(
    shape: Box3D<bool, N>,
    origin: Coords,
    material: &Material,
) -> Vec<Voxel> {
    let shape = shape.map(|slice| {
        slice.map(|col| col.map(|include| if include { Some(material) } else { None }))
    });
    voxels_from_shape(shape, origin)
}
