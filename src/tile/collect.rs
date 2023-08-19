use std::collections::HashSet;

use dfhack_remote::{MatterState, TiletypeMaterial, TiletypeShape};
use rand::Rng;

use super::{BlockTileExt, BlockTilePlantExt};
use crate::{
    context::DFContext,
    palette::{DefaultMaterials, Material},
    rfr::BlockTile,
    shape::{box_from_fn, box_from_levels, box_full, slice_const, Box3D},
    voxel::{voxels_from_uniform_shape, CollectVoxels, Voxel},
    GenBoolSafe, StableRng, VoxelCoords,
};

impl CollectVoxels for BlockTile<'_> {
    fn collect_voxels(
        &self,
        map: &crate::map::Map,
        context: &DFContext,
    ) -> Vec<crate::voxel::Voxel> {
        let coords = self.coords();
        let mut rng = self.stable_rng();
        if self.hidden() {
            let c = map.neighbouring(coords, |tile| tile.block_tile.is_some());
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

        match (self.tile_type().material(), self.tile_type().shape()) {
            (
                TiletypeMaterial::ROOT
                | TiletypeMaterial::MUSHROOM
                | TiletypeMaterial::PLANT
                | TiletypeMaterial::TREE_MATERIAL,
                _,
            )
            | (
                _,
                TiletypeShape::SAPLING
                | TiletypeShape::TWIG
                | TiletypeShape::SHRUB
                | TiletypeShape::BRANCH,
            ) => {
                // plant, trees
                voxels.extend(self.collect_tree_voxels(map, context));
            }
            _ => {
                // classic tile structure
                voxels.extend(self.collect_structure_voxels(map));
            }
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

        // spatters
        let occupied: HashSet<VoxelCoords> = voxels.iter().map(|v| v.coord).collect();
        let mut spatter_voxels = Vec::new();
        for spatter in self.spatters() {
            // spatters sit on top of existing voxels, when there is some space
            let material = Material::Generic(spatter.material.get_or_default().clone());
            for voxel in &voxels {
                let coords = voxel.coord + VoxelCoords::new(0, 0, 1);
                if !occupied.contains(&coords) {
                    let gen = match spatter.state() {
                        // solid spatter is stuff like fruits and leaves, from zero to 10 000.
                        // there are a lot of them, so step down the probability
                        MatterState::Solid => rng.gen_bool_safe(spatter.amount() as f64 / 50_000.0),
                        // liquid spatter is blood etc, from 0 to 255.
                        // completely covered is a bit weird, half the probability
                        MatterState::Liquid => rng.gen_bool_safe(spatter.amount() as f64 / 512.0),
                        // powder spatter is likely snow, going from 0 to 100. We want 100% snow to covere the ground
                        MatterState::Powder => rng.gen_bool_safe(spatter.amount() as f64 / 100.0),
                        // gas, paste and other, I don't know how the can occur
                        _ => false,
                    };
                    if gen {
                        spatter_voxels.push(Voxel::new(coords, material.clone()));
                    }
                }
            }
        }
        voxels.extend(spatter_voxels.into_iter());

        // Fire is identified as a special tiletype material
        if self.tile_type().material() == TiletypeMaterial::FIRE {
            let shape: Box3D<bool> = box_from_fn(|_, _, _| rng.gen_bool(0.1));
            voxels.extend(voxels_from_uniform_shape(
                shape,
                self.coords(),
                Material::Default(DefaultMaterials::Fire),
            ));
        }

        voxels
    }
}
