use dfhack_remote::BuildingInstance;

use crate::{
    building_type::BuildingType,
    direction::DirectionFlat,
    map::Map,
    shape::{self, Box3D, Rotating},
    tile::{NormalTile, Shape, Tile, TileKind},
    voxel::{voxels_from_uniform_shape, CollectVoxels, Voxel},
};

use super::BuildingExtensions;

impl CollectVoxels for BuildingInstance {
    fn collect_voxels(&self, map: &Map) -> Vec<Voxel> {
        let origin = self.origin();
        let shape = match self.building_type() {
            BuildingType::ArcheryTarget { direction } => archery_shape(direction),
            BuildingType::GrateFloor | BuildingType::BarsFloor => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_from_fn(|x, y| {
                    (origin.x + x as i32) % 2 == 0 || (origin.y + y as i32) % 2 == 0
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
            .looking_at(map.wall_direction(origin)),
            BuildingType::Statue | BuildingType::GearAssembly => [
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
            .looking_at(map.wall_direction(origin)),
            BuildingType::AnimalTrap
            | BuildingType::Chair
            | BuildingType::Chain
            | BuildingType::DisplayFurniture
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
            BuildingType::Table | BuildingType::TractionBench => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_full(),
                [
                    [false, false, false],
                    [false, true, false],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ],
            BuildingType::Bed => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [true, true, true],
                    [true, true, true],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ]
            .looking_at(map.wall_direction(origin)),
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
            BuildingType::Well => {
                #[rustfmt::skip]
                let shape = [
                    shape::slice_empty(),
                    [
                        [false, false, false],
                        [true, true, true],
                        [false, false, false],
                    ],
                    [
                        [false, false, false],
                        [true, false, true],
                        [false, false, false],
                    ],
                    [
                        [true, true, true],
                        [true, false, true],
                        [true, true, true],
                    ],
                    [
                        [true, true, true],
                        [true, false, true],
                        [true, true, true]
                    ],
                ];
                shape
            }
            BuildingType::WindowGem | BuildingType::WindowGlass => window_shape(self, map),
            BuildingType::Door => door_shape(self, map),
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
                shape.looking_at(map.wall_direction(origin))
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
                shape.looking_at(map.wall_direction(origin))
            }
            _ => return vec![],
        };
        voxels_from_uniform_shape(shape, origin, self.material())
    }
}

pub fn window_shape(building: &BuildingInstance, map: &Map) -> Box3D<bool> {
    let conn = map.neighbouring_flat(building.origin(), |tile, buildings| {
        buildings.iter().any(|b| {
            matches!(
                b.building_type(),
                BuildingType::WindowGem | BuildingType::WindowGlass
            )
        }) || matches!(
            tile,
            Some(Tile {
                kind: TileKind::Normal(NormalTile {
                    shape: Shape::Fortification | Shape::Full,
                    ..
                }),
                ..
            })
        )
    });
    [
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        shape::slice_empty(),
    ]
}

pub fn door_shape(building: &BuildingInstance, map: &Map) -> Box3D<bool> {
    let conn = map.neighbouring_flat(building.origin(), |tile, buildings| {
        buildings
            .iter()
            .any(|b| matches!(b.building_type(), BuildingType::Door))
            || matches!(
                tile,
                Some(Tile {
                    kind: TileKind::Normal(NormalTile {
                        shape: Shape::Fortification | Shape::Full,
                        ..
                    }),
                    ..
                })
            )
    });
    [
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        [
            [false, conn.n, false],
            [conn.w, true, conn.e],
            [false, conn.s, false],
        ],
        shape::slice_empty(),
    ]
}

pub fn archery_shape(direction: DirectionFlat) -> Box3D<bool> {
    [
        shape::slice_empty(),
        [
            [true, true, true],
            [false, true, false],
            [false, false, false],
        ],
        [
            [true, true, true],
            [false, true, false],
            [false, true, false],
        ],
        [
            [true, true, true],
            [false, true, false],
            [false, true, false],
        ],
        shape::slice_empty(),
    ]
    .looking_at(direction)
}
