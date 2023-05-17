use super::{
    BuildingInstanceBridgeExt, BuildingInstanceExt, BuildingInstanceFurnitureExt, BuildingType,
};
use crate::{
    direction::{DirectionFlat, Rotating},
    export::ExportSettings,
    map::Map,
    shape,
    voxel::{
        voxels_from_dot_vox, voxels_from_uniform_shape, CollectVoxels, Voxel, WithDotVoxMaterials,
    },
    WithCoords,
};
use dfhack_remote::{BuildingDefinition, BuildingInstance, PlantRawList};
use dot_vox::Model;
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
            if let Some(cfg) = crate::models::MODELS.building(building_definition.id()) {
                let mut model = Model {
                    size: cfg.model.size,
                    voxels: cfg.model.voxels.clone(),
                };
                match cfg.orientation_mode {
                    crate::models::OrientationMode::FromDwarfFortress => {
                        if let Some(direction) = DirectionFlat::maybe_from_df(&self.direction()) {
                            model = model.looking_at(direction);
                        }
                    }
                    crate::models::OrientationMode::AgainstWall => {
                        model = model.facing_away(map.wall_direction(self.origin()));
                    }
                }
                return voxels_from_dot_vox(&model, self.origin(), &self.dot_vox_materials());
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
            BuildingType::Hatch => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_full(),
                shape::slice_empty(),
            ],
            BuildingType::BarsVertical
            | BuildingType::GrateWall
            | BuildingType::Support
            | BuildingType::AxleVertical => shape::box_from_fn(|x, y, _| x == 1 && y == 1),
            BuildingType::GearAssembly => [
                shape::slice_empty(),
                [
                    [false, false, false],
                    [false, true, false],
                    [false, false, false],
                ],
                [
                    [false, true, false],
                    [true, true, true],
                    [false, true, false],
                ],
                shape::slice_full(),
                shape::slice_empty(),
            ],
            BuildingType::Box => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [false, true, false],
                    [false, false, false],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ]
            .facing_away(map.wall_direction(coords)),
            BuildingType::AnimalTrap
            | BuildingType::Chair
            | BuildingType::Chain
            | BuildingType::OfferingPlace => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [false, false, false],
                    [false, true, false],
                    [false, false, false],
                ],
                shape::slice_empty(),
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
            BuildingType::Coffin => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [true, true, true],
                    [true, true, true],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ],
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
