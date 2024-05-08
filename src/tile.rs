mod generic;
mod tree;
use std::collections::HashSet;

use crate::{
    context::DFContext,
    palette::{DefaultMaterials, Material},
    rfr::BlockTile,
    shape::{box_from_fn, box_from_levels, box_full, slice_const, Box3D},
    voxel::voxels_from_uniform_shape,
    GenBoolSafe, StableRng, VoxelCoords, WithDFCoords,
};
use dfhack_remote::{MatterState, TiletypeMaterial, TiletypeShape};
pub use generic::BlockTileExt;
use rand::Rng;
pub use tree::BlockTilePlantExt;

#[derive(Debug, Clone, Default)]
pub struct TileVoxels {
    pub terrain: Vec<dot_vox::Voxel>,
    pub liquid: Vec<dot_vox::Voxel>,
    pub spatter: Vec<dot_vox::Voxel>,
    pub fire: Vec<dot_vox::Voxel>,
    pub void: Vec<dot_vox::Voxel>,
}

impl WithDFCoords for BlockTile<'_> {
    fn coords(&self) -> crate::DFMapCoords {
        self.global_coords()
    }
}

impl BlockTile<'_> {
    pub fn build(
        &self,
        map: &crate::map::Map,
        context: &DFContext,
        palette: &mut crate::palette::Palette,
    ) -> TileVoxels {
        let mut rng = self.stable_rng();

        let mut voxels = TileVoxels::default();

        if self.hidden() {
            let shape: Box3D<bool> = box_full();

            voxels.void.extend(voxels_from_uniform_shape(
                shape,
                self.local_coords(),
                palette.get(&Material::Default(DefaultMaterials::Hidden), context),
            ));
            return voxels;
        }

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
                voxels
                    .terrain
                    .extend(self.build_trees(map, context, palette));
            }
            _ => {
                // classic tile structure
                voxels
                    .terrain
                    .extend(self.build_structure(map, context, palette));
            }
        }

        // liquids
        if self.water() > 0 {
            let water_shape: Box3D<bool> =
                box_from_levels(slice_const(self.water().min(7).max(2) as usize));
            voxels.liquid.extend(voxels_from_uniform_shape(
                water_shape,
                self.local_coords(),
                palette.get(&Material::Default(DefaultMaterials::Water), context),
            ));
        }

        if self.magma() > 0 {
            let magma_shape: Box3D<bool> =
                box_from_levels(slice_const(self.magma().min(7).max(2) as usize));
            voxels.liquid.extend(voxels_from_uniform_shape(
                magma_shape,
                self.local_coords(),
                palette.get(&Material::Default(DefaultMaterials::Magma), context),
            ));
        }

        // spatters
        let occupied: HashSet<VoxelCoords> = voxels
            .terrain
            .iter()
            .map(|v| VoxelCoords::new(v.x as i32, v.y as i32, v.z as i32))
            .collect();

        for spatter in self.spatters() {
            // spatters sit on top of existing voxels, when there is some space
            let material = Material::Generic(spatter.material.get_or_default().clone());
            for voxel in &voxels.terrain {
                let coords = VoxelCoords::new(voxel.x as i32, voxel.y as i32, voxel.z as i32)
                    + VoxelCoords::new(0, 0, 1);
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
                        voxels.spatter.push(dot_vox::Voxel {
                            x: coords.x as u8,
                            y: coords.y as u8,
                            z: coords.z as u8,
                            i: palette.get(&material, context),
                        });
                    }
                }
            }
        }

        // Fire is identified as a special tiletype material
        if self.tile_type().material() == TiletypeMaterial::FIRE {
            let shape: Box3D<bool> = box_from_fn(|_, _, _| rng.gen_bool(0.1));
            let material = palette.get(&Material::Default(DefaultMaterials::Fire), context);
            voxels.fire.extend(voxels_from_uniform_shape(
                shape,
                self.local_coords(),
                material,
            ));
        }
        voxels
    }
}
