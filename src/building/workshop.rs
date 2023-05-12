use crate::{
    shape::{self, Box3D},
    voxel::{voxels_from_uniform_shape, Voxel},
};

use super::Building;

impl Building<'_> {
    pub fn collect_workshop_voxels(&self) -> Vec<Voxel> {
        #[rustfmt::skip]
        let shape: Box3D<bool, 9> = [
            shape::slice_empty(),
            shape::slice_empty(),
            [
                [false, false, false, false, false, false, true, true, true],
                [false, false, false, false, false, false, true, true, true],
                [true, true, true, false, false, false, true, true, true],
                [true, true, true, false, false, false, false, false, false],
                [true, true, true, false, false, false, false, false, false],
                [true, true, true, false, false, false, false, false, false],
                [true, true, true, true, true, true, true, false, false],
                [true, true, true, true, true, true, true, false, false],
                [true, true, true, true, true, true, true, false, false],
            ],
            [
                [false, false, false, false, false, false, true, false, true],
                [false, false, false, false, false, false, false, false, false],
                [true, false, true, false, false, false, true, false, true],
                [false, false, false, false, true, false, false, false, false],
                [false, false, false, false, false, false, false, false, false],
                [false, false, false, false, false, false, false, false, false],
                [true, false, true, false, false, false, true, false, false],
                [false, false, false, false, false, false, false, false, false],
                [true, false, false, false, false, false, true, false, false],
            ],
            shape::slice_full(),
        ];
        voxels_from_uniform_shape(shape, self.origin(), self.material())
    }
}
