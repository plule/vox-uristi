use crate::{
    export::ExportSettings,
    palette::{DefaultMaterials, Material},
    rfr::BlockTile,
    shape::{box_from_levels, box_full, slice_const, Box3D},
    voxel::{voxels_from_uniform_shape, CollectVoxels},
};
use dfhack_remote::PlantRawList;

use super::{BlockTileExt, BlockTilePlantExt};

impl CollectVoxels for BlockTile<'_> {
    fn collect_voxels(
        &self,
        map: &crate::map::Map,
        settings: &ExportSettings,
        plant_raws: &PlantRawList,
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

        if self.material().mat_type() != 419 {
            // classic tile structure
            voxels.extend(self.collect_structure_voxels(map));
        } else {
            // plant, trees
            voxels.extend(self.collect_plant_voxels(map, settings, plant_raws));
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
