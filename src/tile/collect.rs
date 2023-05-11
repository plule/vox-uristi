use super::Tile;
use crate::{
    export::ExportSettings,
    palette::{DefaultMaterials, Material},
    shape::{box_from_levels, box_full, slice_const, Box3D},
    voxel::{voxels_from_uniform_shape, CollectVoxels},
};
use dfhack_remote::PlantRawList;

impl CollectVoxels for Tile<'_> {
    fn collect_voxels(
        &self,
        map: &crate::map::Map,
        settings: &ExportSettings,
        plant_raws: &PlantRawList,
    ) -> Vec<crate::voxel::Voxel> {
        let coords = self.0.coords();
        if self.0.hidden() {
            let shape: Box3D<bool> = box_full();
            return voxels_from_uniform_shape(
                shape,
                coords,
                Material::Default(DefaultMaterials::Hidden),
            );
        }
        let mut voxels = Vec::new();

        if self.0.material().mat_type() != 419 {
            // classic tile structure
            let structure_shape = self.structure_shape(map);
            let structure_material = Material::Generic(self.0.material().clone());
            voxels.extend(voxels_from_uniform_shape(
                structure_shape,
                self.0.coords(),
                structure_material,
            ));
        } else {
            // plant, trees
            voxels.extend(self.collect_plant_voxels(map, settings, plant_raws));
        }

        // liquids
        if self.0.water() > 0 {
            let water_shape: Box3D<bool> =
                box_from_levels(slice_const(self.0.water().min(7).max(2) as usize));
            voxels.extend(voxels_from_uniform_shape(
                water_shape,
                self.0.coords(),
                Material::Default(DefaultMaterials::Water),
            ));
        }

        if self.0.magma() > 0 {
            let magma_shape: Box3D<bool> =
                box_from_levels(slice_const(self.0.magma().min(7).max(2) as usize));
            voxels.extend(voxels_from_uniform_shape(
                magma_shape,
                self.0.coords(),
                Material::Default(DefaultMaterials::Magma),
            ));
        }

        voxels
    }
}
