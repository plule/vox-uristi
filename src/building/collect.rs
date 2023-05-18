use super::{BuildingInstanceExt, BuildingType};
use crate::{
    direction::DirectionFlat,
    export::ExportSettings,
    map::Map,
    shape,
    voxel::{voxels_from_uniform_shape, CollectVoxels, FromPrefab, Voxel},
    Coords, WithCoords,
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
        // Look for a static mesh
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

        // No static mesh, apply a dynamic one
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
            BuildingType::Bridge { direction } => {
                let mut voxels = Vec::new();
                let sn = matches!(direction, Some(DirectionFlat::North | DirectionFlat::South));
                let ew = matches!(direction, Some(DirectionFlat::East | DirectionFlat::West));
                let bounding_box = self.bounding_box();
                for x in bounding_box.x.clone() {
                    for y in bounding_box.y.clone() {
                        let w = sn && x == *bounding_box.x.start();
                        let e = sn && x == *bounding_box.x.end();
                        let n = ew && y == *bounding_box.y.start();
                        let s = ew && y == *bounding_box.y.end();
                        let shape = [
                            shape::slice_empty(),
                            shape::slice_empty(),
                            shape::slice_empty(),
                            [[w || n, n, e || n], [w, false, e], [w || s, s, e || s]],
                            shape::slice_full(),
                        ];
                        let mut shape_voxels = voxels_from_uniform_shape(
                            shape,
                            Coords::new(x, y, self.origin().z),
                            self.material(),
                        );
                        voxels.append(&mut shape_voxels);
                    }
                }
                return voxels;
            }
            _ => return vec![],
        };
        voxels_from_uniform_shape(shape, coords, self.material())
    }
}
