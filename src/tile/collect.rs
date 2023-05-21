use dfhack_remote::MatterState;
use rand::Rng;

use super::{BlockTileExt, BlockTilePlantExt};
use crate::{
    context::DFContext,
    palette::{DefaultMaterials, Material},
    rfr::BlockTile,
    shape::{box_from_levels, box_full, slice_const, slice_empty, slice_from_fn, Box3D},
    voxel::{voxels_from_uniform_shape, CollectVoxels},
};

impl CollectVoxels for BlockTile<'_> {
    fn collect_voxels(
        &self,
        map: &crate::map::Map,
        context: &DFContext,
    ) -> Vec<crate::voxel::Voxel> {
        let coords = self.coords();
        if self.hidden() {
            let c = map.neighbouring(coords, |n, _| n.is_some());
            if c.a && c.b && c.n && c.e && c.s && c.w {
                // hidden block surrounded by hidden blocks, skip
                return vec![];
            }
            let shape: Box3D<bool> = box_full();
            return voxels_from_uniform_shape(
                shape,
                coords,
                Material::Default(DefaultMaterials::Hidden),
            );
        }
        let mut voxels = Vec::new();

        // spatters
        for spatter in self.spatters() {
            let mut rng = rand::thread_rng();
            let slice = match spatter.state() {
                // solid spatter is stuff like fruits and leaves, from zero to 10 000.
                // there are a lot of them, so step down the probability
                MatterState::Solid => {
                    slice_from_fn(|_, _| rng.gen_bool(spatter.amount() as f64 / 50_000.0))
                }
                // liquid spatter is blood etc, from 0 to 255.
                // completely covered is a bit weird, half the probability
                MatterState::Liquid => {
                    slice_from_fn(|_, _| rng.gen_bool(spatter.amount() as f64 / 512.0))
                }
                // powder spatter is likely snow, going from 0 to 100. We want 100% snow to covere the ground
                MatterState::Powder => {
                    slice_from_fn(|_, _| rng.gen_bool(spatter.amount() as f64 / 100.0))
                }
                // gas, paste and other, I don't know how the can occur
                _ => slice_empty(),
            };
            let spatter_shape: Box3D<bool> = [
                slice_empty(),
                slice_empty(),
                slice_empty(),
                slice,
                slice_empty(),
            ];
            voxels.extend(voxels_from_uniform_shape(
                spatter_shape,
                self.coords(),
                Material::Generic(spatter.material.get_or_default().clone()),
            ));
        }

        if self.material().mat_type() != 419 {
            // classic tile structure
            voxels.extend(self.collect_structure_voxels(map));
        } else {
            // plant, trees
            voxels.extend(self.collect_plant_voxels(map, context));
        }

        // liquids
        if self.water() > 0 {
            let water_shape: Box3D<bool> =
                box_from_levels(slice_const(self.water().min(7).max(2) as usize));
            voxels.extend(voxels_from_uniform_shape(
                water_shape,
                self.coords(),
                Material::Default(DefaultMaterials::Water),
            ));
        }

        if self.magma() > 0 {
            let magma_shape: Box3D<bool> =
                box_from_levels(slice_const(self.magma().min(7).max(2) as usize));
            voxels.extend(voxels_from_uniform_shape(
                magma_shape,
                self.coords(),
                Material::Default(DefaultMaterials::Magma),
            ));
        }

        voxels
    }
}
