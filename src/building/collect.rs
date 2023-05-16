use super::{
    BuildingInstanceBridgeExt, BuildingInstanceExt, BuildingInstanceFurnitureExt, BuildingType,
};
use crate::{
    direction::Rotating,
    export::ExportSettings,
    map::Map,
    shape,
    voxel::{
        voxels_from_dot_vox, voxels_from_uniform_shape, CollectVoxels, Voxel, WithDotVoxMaterials,
    },
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
            if let Some(model) = crate::models::BUILDINGS.get(building_definition.id()) {
                return voxels_from_dot_vox(model, self.origin(), &self.dot_vox_materials());
            }
        }
        let coords = self.origin();
        let shape = match self.building_type() {
            BuildingType::ArcheryTarget { direction } => BuildingInstance::archery_shape(direction),
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
            BuildingType::Bookcase | BuildingType::Cabinet => [
                [
                    [true, true, true],
                    [false, false, false],
                    [false, false, false],
                ],
                [
                    [true, true, true],
                    [true, true, true],
                    [false, false, false],
                ],
                [
                    [true, true, true],
                    [false, false, false],
                    [false, false, false],
                ],
                [
                    [true, true, true],
                    [true, true, true],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ]
            .looking_at(map.wall_direction(coords)),
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
            .looking_at(map.wall_direction(coords)),
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
            BuildingType::Bed => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [true, true, false],
                    [true, true, false],
                    [true, true, false],
                ],
                shape::slice_empty(),
            ]
            .looking_at(map.wall_direction(coords)),
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
            BuildingType::ArmorStand => {
                #[rustfmt::skip]
                let shape = [
                    shape::slice_empty(),
                    [
                        [true, true, true],
                        [false, false, false],
                        [false, false, false],
                    ],
                    [
                        [false, true, false],
                        [false, false, false],
                        [false, false, false],
                    ],
                    [
                        [true, true, true],
                        [true, true, true],
                        [false, false, false],
                    ],
                    shape::slice_empty(),
                ];
                shape.looking_at(map.wall_direction(coords))
            }
            BuildingType::WeaponRack => {
                #[rustfmt::skip]
                    let shape = [
                        [
                            [true, false, true],
                            [false, false, false],
                            [false, false, false],
                        ],
                        [
                            [true, true, true],
                            [false, false, false],
                            [false, false, false],
                        ],
                        [
                            [true, false, true],
                            [false, false, false],
                            [false, false, false],
                        ],
                        [
                            [true, true, true],
                            [true, false, true],
                            [false, false, false],
                        ],
                        shape::slice_empty(),
                    ];
                shape.looking_at(map.wall_direction(coords))
            }
            _ => return vec![],
        };
        voxels_from_uniform_shape(shape, coords, self.material())
    }
}
