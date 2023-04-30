use dfhack_remote::BuildingInstance;
use itertools::Itertools;
use std::ops::RangeInclusive;

use crate::{
    direction::{Direction, DirectionFlat},
    map::{Coords, Map},
    palette::{Material, Palette},
};

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

pub struct Building {
    pub building_type: BuildingType,
    pub material: Material,
    pub origin: Coords,
    pub bounding_box: BoundingBox,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BuildingType {
    Door,
    Floodgate,
    WindowGlass,
    WindowGem,
    Workshop { subtype: i32 },
    Bridge { direction: Option<DirectionFlat> },
    Support,
    Hatch,
    GrateWall,
    GrateFloor,
    BarsVertical,
    BarsFloor,
    AxleVertical,
    Slab,
    Bookcase,
    DisplayFurniture,
    OfferingPlace,
}

pub trait BuildingExtensions {
    fn get_type(&self) -> Option<BuildingType>;
}

impl BuildingExtensions for dfhack_remote::BuildingInstance {
    fn get_type(&self) -> Option<BuildingType> {
        let building_type = self.building_type.get_or_default();
        let t = match building_type.building_type() {
            8 => BuildingType::Door,
            9 => BuildingType::Floodgate,
            13 => BuildingType::Workshop {
                subtype: building_type.building_subtype(),
            },
            16 => BuildingType::WindowGlass,
            17 => BuildingType::WindowGem,
            19 => BuildingType::Bridge {
                direction: DirectionFlat::maybe_from_df(&self.direction()),
            },
            25 => BuildingType::Support,
            35 => BuildingType::Hatch,
            36 => BuildingType::GrateWall,
            37 => BuildingType::GrateFloor,
            38 => BuildingType::BarsVertical,
            39 => BuildingType::BarsFloor,
            42 => BuildingType::AxleVertical,
            46 => BuildingType::Slab,
            52 => BuildingType::Bookcase,
            53 => BuildingType::DisplayFurniture,
            54 => BuildingType::OfferingPlace,
            _ => return None,
        };
        Some(t)
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
            BuildingType::GrateFloor | BuildingType::BarsFloor => [
                [
                    [false, false, false],
                    [false, false, false],
                    [false, false, false],
                ],
                [
                    [false, false, false],
                    [false, false, false],
                    [false, false, false],
                ],
                [
                    [false, true, false],
                    [true, true, true],
                    [false, true, false],
                ],
            ],
            BuildingType::BarsVertical
            | BuildingType::GrateWall
            | BuildingType::Support
            | BuildingType::AxleVertical => [
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

    fn window_shape(&self, map: &Map) -> [[[bool; 3]; 3]; 3] {
        let n = map.connect_window_to_coords(self.origin + Direction::North);
        let s = map.connect_window_to_coords(self.origin + Direction::South);
        let e = map.connect_window_to_coords(self.origin + Direction::East);
        let w = map.connect_window_to_coords(self.origin + Direction::West);
        [
            [[false, n, false], [w, true, e], [false, s, false]],
            [[false, n, false], [w, true, e], [false, s, false]],
            [[false, n, false], [w, true, e], [false, s, false]],
        ]
    }

    fn door_shape(&self, map: &Map) -> [[[bool; 3]; 3]; 3] {
        let n = map.connect_door_to_coords(self.origin + Direction::North);
        let s = map.connect_door_to_coords(self.origin + Direction::South);
        let e = map.connect_door_to_coords(self.origin + Direction::East);
        let w = map.connect_door_to_coords(self.origin + Direction::West);
        [
            [[false, n, false], [w, true, e], [false, s, false]],
            [[false, n, false], [w, true, e], [false, s, false]],
            [[false, n, false], [w, true, e], [false, s, false]],
        ]
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
                    [
                        [false, false, false],
                        [false, false, false],
                        [false, false, false],
                    ],
                    [[w || n, n, e || n], [w, false, e], [w || s, s, e || s]],
                    [[true, true, true], [true, true, true], [true, true, true]],
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
    shape: [[[bool; 3]; 3]; 3],
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
