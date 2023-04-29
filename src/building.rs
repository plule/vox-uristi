use anyhow::bail;
use dfhack_remote::BuildingInstance;
use itertools::Itertools;

use crate::{
    map::{Coords, Direction, Map},
    palette::{Material, Palette},
};

pub struct Building {
    pub building_type: BuildingType,
    pub material: Material,
    pub coords: Coords,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BuildingType {
    Door,
    Floodgate,
    WindowGlass,
    WindowGem,
    Workshop { subtype: i32 },
    Bridge,
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

impl TryFrom<&dfhack_remote::BuildingType> for BuildingType {
    type Error = anyhow::Error;

    fn try_from(value: &dfhack_remote::BuildingType) -> Result<Self, Self::Error> {
        let t = match value.building_type() {
            8 => BuildingType::Door,
            9 => BuildingType::Floodgate,
            13 => BuildingType::Workshop {
                subtype: value.building_subtype(),
            },
            16 => BuildingType::WindowGlass,
            17 => BuildingType::WindowGem,
            19 => BuildingType::Bridge,
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
            other => bail!(anyhow::format_err!("Unsupported building_type {}", other)),
        };
        Ok(t)
    }
}

pub trait BuildingExtensions {
    fn get_type(&self) -> Option<BuildingType>;
}

impl BuildingExtensions for dfhack_remote::BuildingInstance {
    fn get_type(&self) -> Option<BuildingType> {
        self.building_type.get_or_default().try_into().ok()
    }
}

impl Building {
    pub fn from_df_building(df_building: BuildingInstance) -> Option<Self> {
        df_building.get_type().map(|building_type| Self {
            building_type,
            material: Material::Generic(df_building.material.get_or_default().to_owned()),
            coords: Coords::new(
                df_building.pos_x_min(),
                df_building.pos_y_min(),
                df_building.pos_z_min(),
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
            BuildingType::WindowGem | BuildingType::WindowGlass => {
                let n = map.connect_window_to_coords(self.coords + Direction::North);
                let s = map.connect_window_to_coords(self.coords + Direction::South);
                let e = map.connect_window_to_coords(self.coords + Direction::East);
                let w = map.connect_window_to_coords(self.coords + Direction::West);
                [
                    [[false, n, false], [w, true, e], [false, s, false]],
                    [[false, n, false], [w, true, e], [false, s, false]],
                    [[false, n, false], [w, true, e], [false, s, false]],
                ]
            }
            _ => return vec![],
        };
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
                    self.coords.x * 3 + local_x as i32,
                    self.coords.y * 3 + local_y as i32,
                    self.coords.z * 3 + local_z as i32,
                )
            })
            .map(|coords| (coords, self.material.pick_color(&palette.colors)))
            .collect_vec()
    }
}
