use crate::{
    building_type::BuildingType,
    direction::DirectionFlat,
    map::{Coords, IsSomeAnd, Map},
    palette::{Material, Palette},
    shape::{self, Box3D, Rotating},
    tile::Shape,
};
use dfhack_remote::BuildingInstance;
use itertools::Itertools;
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct BoundingBox {
    pub x: RangeInclusive<i32>,
    pub y: RangeInclusive<i32>,
    pub z: RangeInclusive<i32>,
}

impl BoundingBox {
    pub fn new(x: RangeInclusive<i32>, y: RangeInclusive<i32>, z: RangeInclusive<i32>) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug)]
pub struct Building {
    pub building_type: BuildingType,
    pub material: Material,
    pub origin: Coords,
    pub bounding_box: BoundingBox,
}

pub trait BuildingExtensions {
    fn get_type(&self) -> Option<BuildingType>;
}

impl BuildingExtensions for dfhack_remote::BuildingInstance {
    fn get_type(&self) -> Option<BuildingType> {
        BuildingType::maybe_from_df(self)
    }
}

impl Building {
    pub fn from_df_building(df_building: BuildingInstance) -> Option<Self> {
        df_building.get_type().map(|building_type| Self {
            building_type,
            material: Material::Generic(df_building.material.get_or_default().to_owned()),
            origin: Coords::new(
                df_building.pos_x_min(),
                df_building.pos_y_min(),
                df_building.pos_z_min(),
            ),
            bounding_box: BoundingBox::new(
                df_building.pos_x_min()..=df_building.pos_x_max(),
                df_building.pos_y_min()..=df_building.pos_y_max(),
                df_building.pos_z_min()..=df_building.pos_z_max(),
            ),
        })
    }

    pub fn collect_voxels(&self, palette: &Palette, map: &Map) -> Vec<(Coords, u8)> {
        let shape = match self.building_type {
            BuildingType::ArcheryTarget { direction } => self.archery_shape(direction),
            BuildingType::GrateFloor | BuildingType::BarsFloor => [
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [false, true, false],
                    [true, true, true],
                    [false, true, false],
                ],
            ],
            BuildingType::Hatch => [
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
                    [true, true, true],
                    [false, false, false],
                ],
                [
                    [true, true, true],
                    [true, true, true],
                    [false, false, false],
                ],
                [
                    [true, true, true],
                    [true, true, true],
                    [false, false, false],
                ],
            ]
            .looking_at(map.wall_direction(self.origin)),
            BuildingType::Statue | BuildingType::GearAssembly => [
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
                [
                    [false, false, false],
                    [false, true, false],
                    [false, false, false],
                ],
            ],
            BuildingType::Box => [
                shape::slice_empty(),
                [
                    [false, true, false],
                    [false, false, false],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ]
            .looking_at(map.wall_direction(self.origin)),
            BuildingType::AnimalTrap
            | BuildingType::Chair
            | BuildingType::Chain
            | BuildingType::DisplayFurniture
            | BuildingType::OfferingPlace => [
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
                shape::slice_full(),
                shape::slice_empty(),
            ],
            BuildingType::Bed => [
                shape::slice_empty(),
                [
                    [false, true, true],
                    [false, false, false],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ]
            .looking_at(map.wall_direction(self.origin)),
            BuildingType::Coffin => [
                shape::slice_empty(),
                [
                    [false, true, false],
                    [false, true, false],
                    [false, false, false],
                ],
                shape::slice_empty(),
            ],
            BuildingType::Well => [
                [
                    [false, false, false],
                    [true, true, true],
                    [false, false, false],
                ],
                [
                    [false, false, false],
                    [false, true, false],
                    [false, false, false],
                ],
                [
                    [false, false, false],
                    [false, true, false],
                    [false, false, false],
                ],
            ],
            BuildingType::WindowGem | BuildingType::WindowGlass => self.window_shape(map),
            BuildingType::Door => self.door_shape(map),
            BuildingType::Bridge { direction } => {
                return self.bridge_collect_voxels(palette, direction);
            }
            _ => return vec![],
        };
        collect_shape_voxels(&self.origin, &self.material, palette, shape)
    }

    fn window_shape(&self, map: &Map) -> Box3D<3, bool> {
        let conn = map.neighbouring_flat(self.origin, |tile, buildings| {
            buildings.iter().any(|b| {
                matches!(
                    b.building_type,
                    BuildingType::WindowGem | BuildingType::WindowGlass
                )
            }) || tile.some_and(|t| matches!(t.shape, Shape::Fortification | Shape::Full))
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
        ]
    }

    fn door_shape(&self, map: &Map) -> Box3D<3, bool> {
        let conn = map.neighbouring_flat(self.origin, |tile, buildings| {
            buildings
                .iter()
                .any(|b| matches!(b.building_type, BuildingType::Door))
                || tile.some_and(|t| matches!(t.shape, Shape::Fortification | Shape::Full))
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
        ]
    }

    fn archery_shape(&self, direction: DirectionFlat) -> Box3D<3, bool> {
        [
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
        ]
        .looking_at(direction)
    }

    fn bridge_collect_voxels(
        &self,
        palette: &Palette,
        direction: Option<DirectionFlat>,
    ) -> Vec<(Coords, u8)> {
        let mut voxels = Vec::new();
        let sn = matches!(direction, Some(DirectionFlat::North | DirectionFlat::South));
        let ew = matches!(direction, Some(DirectionFlat::East | DirectionFlat::West));
        for x in self.bounding_box.x.clone() {
            for y in self.bounding_box.y.clone() {
                let w = sn && x == *self.bounding_box.x.start();
                let e = sn && x == *self.bounding_box.x.end();
                let n = ew && y == *self.bounding_box.y.start();
                let s = ew && y == *self.bounding_box.y.end();
                let shape = [
                    shape::slice_empty(),
                    [[w || n, n, e || n], [w, false, e], [w || s, s, e || s]],
                    shape::slice_full(),
                ];
                let mut shape_voxels = collect_shape_voxels(
                    &Coords::new(x, y, self.origin.z),
                    &self.material,
                    palette,
                    shape,
                );
                voxels.append(&mut shape_voxels);
            }
        }
        voxels
    }
}

fn collect_shape_voxels(
    coords: &Coords,
    material: &Material,
    palette: &Palette,
    shape: Box3D<3, bool>,
) -> Vec<(Coords, u8)> {
    (0_usize..3_usize)
        .flat_map(move |x| {
            (0_usize..3_usize).flat_map(move |y| {
                (0_usize..3_usize).filter_map(move |z| {
                    if shape[2 - z][y][x] {
                        Some((x, y, z))
                    } else {
                        None
                    }
                })
            })
        })
        .map(|(local_x, local_y, local_z)| {
            Coords::new(
                coords.x * 3 + local_x as i32,
                coords.y * 3 + local_y as i32,
                coords.z * 3 + local_z as i32,
            )
        })
        .map(|coords| (coords, material.pick_color(&palette.colors)))
        .collect_vec()
}
