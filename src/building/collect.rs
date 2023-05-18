use super::{
    BuildingInstanceBridgeExt, BuildingInstanceExt, BuildingInstanceFurnitureExt, BuildingType,
};
use crate::{
    export::ExportSettings,
    map::Map,
    shape,
    voxel::{voxels_from_uniform_shape, CollectVoxels, FromPrefab, Voxel},
    WithCoords,
};
use dfhack_remote::{BuildingDefinition, BuildingInstance, PlantRawList};
use std::collections::HashMap;

impl CollectVoxels for BuildingInstance {
    fn collect_voxels(
        &self,
        map: &Map,
        _settings: &ExportSettings,
        _plant_raws: &PlantRawList,
        building_defs: &HashMap<(i32, i32, i32), BuildingDefinition>,
    ) -> Vec<Voxel> {
        let building_type = self.building_type.get_or_default();
        if let Some(building_definition) = building_defs.get(&(
            building_type.building_type(),
            building_type.building_subtype(),
            building_type.building_custom(),
        )) {
            if let Some(prefab) = crate::prefabs::MODELS.building(building_definition.id()) {
                return self.voxels_from_prefab(prefab, map);
            }
        }
        let coords = self.origin();
        let shape = match self.building_type() {
            BuildingType::GrateFloor | BuildingType::BarsFloor => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_from_fn(|x, y| {
                    (coords.x + x as i32) % 2 == 0 || (coords.y + y as i32) % 2 == 0
                }),
            ],
            BuildingType::Table | BuildingType::TractionBench => {
                let edges = map.neighbouring_flat(self.coords(), |_, buildings| {
                    !buildings.iter().any(|building| {
                        matches!(
                            building.building_type(),
                            BuildingType::Table | BuildingType::TractionBench
                        )
                    })
                });
                [
                    shape::slice_empty(),
                    shape::slice_empty(),
                    shape::slice_full(),
                    [
                        [edges.n && edges.w, false, edges.n && edges.e],
                        [false, false, false],
                        [edges.s && edges.w, false, edges.s && edges.e],
                    ],
                    shape::slice_empty(),
                ]
            }
            BuildingType::WindowGem | BuildingType::WindowGlass => self.window_shape(map),
            BuildingType::Door => self.door_shape(map),
            BuildingType::Bridge { direction } => {
                return self.bridge_collect_voxels(direction);
            }
            _ => return vec![],
        };
        voxels_from_uniform_shape(shape, coords, self.material())
    }
}
