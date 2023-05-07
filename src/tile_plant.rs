use crate::{
    direction::{DirectionFlat, Neighbouring, NeighbouringFlat},
    map::{Coords, Map},
    palette::{ConsoleColor, Material},
    rfr::{BlockTile, GetTiming},
    shape::{self, Box3D},
    tile::TileKind,
    voxel::{voxels_from_shape, voxels_from_uniform_shape, Voxel},
};
use dfhack_remote::{MatPair, PlantRawList, TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use rand::{seq::SliceRandom, Rng};

#[derive(Debug)]
pub struct PlantTile {
    pub plant_index: i32,
    pub part: PlantPart,
    pub alive: bool,
    pub wood_material: Material,
    pub growth_materials: Vec<Material>,
    pub origin: Coords,
}

#[derive(Debug, PartialEq)]
pub enum PlantPart {
    Root,
    Sapling,
    Trunk,
    HeavyBranch {
        connectivity: NeighbouringFlat<bool>,
    },
    LightBranch,
    Twig,
    Cap,
}

fn connectivity_from_direction_string(direction_string: &str) -> NeighbouringFlat<bool> {
    let directions: Vec<DirectionFlat> = direction_string
        .chars()
        .filter_map(|c| match c {
            'N' => Some(DirectionFlat::North),
            'E' => Some(DirectionFlat::East),
            'S' => Some(DirectionFlat::South),
            'W' => Some(DirectionFlat::West),
            _ => None,
        })
        .collect();
    if directions.len() <= 1 {
        // When there is a single direction, the direction indicates
        // where the branch is heading, so the connectivity is opposite
        NeighbouringFlat {
            n: directions.contains(&DirectionFlat::South),
            e: directions.contains(&DirectionFlat::West),
            s: directions.contains(&DirectionFlat::North),
            w: directions.contains(&DirectionFlat::East),
        }
    } else {
        // When there are multiple direction, it indicates the connectivity
        NeighbouringFlat {
            n: directions.contains(&DirectionFlat::North),
            e: directions.contains(&DirectionFlat::East),
            s: directions.contains(&DirectionFlat::South),
            w: directions.contains(&DirectionFlat::West),
        }
    }
}

impl PlantTile {
    pub fn from_block_tile(tile: &BlockTile, year_tick: i32, raws: &PlantRawList) -> Self {
        let plant_index = tile.material().mat_index();
        let tile_type = tile.tile_type();
        let part = match (
            tile_type.material(),
            tile_type.shape(),
            tile_type.direction(),
        ) {
            // these are probably actually somewhere...
            (TiletypeMaterial::ROOT, _, _) => PlantPart::Root,
            (TiletypeMaterial::MUSHROOM, _, _) => PlantPart::Cap,
            (_, TiletypeShape::SAPLING, _) => PlantPart::Sapling,
            (_, TiletypeShape::TWIG, _) => PlantPart::Twig,
            (_, TiletypeShape::BRANCH, "--------") => PlantPart::LightBranch,
            (_, TiletypeShape::BRANCH, direction) => PlantPart::HeavyBranch {
                connectivity: connectivity_from_direction_string(direction),
            },
            _ => PlantPart::Trunk,
        };
        let alive = !matches!(
            tile_type.special(),
            TiletypeSpecial::DEAD | TiletypeSpecial::SMOOTH_DEAD
        );
        let wood_material = Material::Generic(MatPair {
            mat_type: Some(420),
            mat_index: Some(plant_index),
            ..Default::default()
        });
        let origin = tile.tree_origin();
        let mut ret = Self {
            plant_index,
            wood_material,
            part,
            alive,
            origin,
            growth_materials: vec![],
        };
        ret.growth_materials = ret
            .growth_colors(raws, year_tick)
            .into_iter()
            .map(Material::Console)
            .collect();
        ret
    }

    pub fn growth_colors(&self, raws: &PlantRawList, year_tick: i32) -> Vec<ConsoleColor> {
        if let Some(plant_raw) = raws.plant_raws.get(self.plant_index as usize) {
            plant_raw
                .growths
                .iter()
                .filter(|growth| {
                    growth.timing().contains(&year_tick)
                        && match self.part {
                            PlantPart::Cap => growth.cap(),
                            PlantPart::Root => growth.roots(),
                            PlantPart::Sapling => growth.sapling(),
                            PlantPart::Trunk => growth.trunk(),
                            PlantPart::HeavyBranch { .. } => growth.heavy_branches(),
                            PlantPart::LightBranch => growth.light_branches(),
                            PlantPart::Twig => growth.twigs(),
                        }
                })
                .flat_map(|growth| {
                    growth
                        .prints
                        .iter()
                        .filter(|print| print.timing().contains(&year_tick))
                        .map(|print| print.color().into())
                })
                .collect()
        } else {
            vec![]
        }
    }

    pub fn collect_voxels(&self, coords: &Coords, map: &Map) -> Vec<Voxel> {
        let mut rng = rand::thread_rng();
        let mut voxels = voxels_from_uniform_shape(
            self.structure_shape(coords, map),
            *coords,
            &self.wood_material,
        );
        if !self.growth_materials.is_empty() {
            let growth = self.growth_shape().map(|slice| {
                slice.map(|col| {
                    col.map(|t| {
                        if t {
                            self.growth_materials.choose(&mut rng)
                        } else {
                            None
                        }
                    })
                })
            });
            voxels.append(&mut voxels_from_shape(growth, *coords));
        }

        voxels
    }

    pub fn growth_shape(&self) -> Box3D<3, bool> {
        let mut r = rand::thread_rng();
        match &self.part {
            PlantPart::Root | PlantPart::Trunk | PlantPart::Cap | PlantPart::HeavyBranch { .. } => {
                [
                    [
                        [r.gen_ratio(1, 5), false, r.gen_ratio(1, 5)],
                        [false, false, false],
                        [r.gen_ratio(1, 5), false, r.gen_ratio(1, 5)],
                    ],
                    [
                        [r.gen_ratio(1, 5), false, r.gen_ratio(1, 5)],
                        [false, false, false],
                        [r.gen_ratio(1, 5), false, r.gen_ratio(1, 5)],
                    ],
                    [
                        [r.gen_ratio(1, 5), false, r.gen_ratio(1, 5)],
                        [false, false, false],
                        [r.gen_ratio(1, 5), false, r.gen_ratio(1, 5)],
                    ],
                ]
            }
            PlantPart::Twig | PlantPart::LightBranch => [
                [
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                ],
                [
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), true, r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                ],
                [
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                ],
            ],
            PlantPart::Sapling => [
                [
                    [false, false, false],
                    [false, r.gen_ratio(1, 5), false],
                    [false, false, false],
                ],
                [
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                ],
                shape::slice_empty(),
            ],
        }
    }

    pub fn structure_shape(&self, coords: &Coords, map: &Map) -> Box3D<3, bool> {
        let mut r = rand::thread_rng();
        // The horror
        match &self.part {
            PlantPart::Root | PlantPart::Trunk | PlantPart::Cap => [
                [
                    [r.gen_ratio(1, 3), true, r.gen_ratio(1, 3)],
                    [true, true, true],
                    [r.gen_ratio(1, 3), true, r.gen_ratio(1, 3)],
                ],
                [
                    [r.gen_ratio(1, 3), true, r.gen_ratio(1, 3)],
                    [true, true, true],
                    [r.gen_ratio(1, 3), true, r.gen_ratio(1, 3)],
                ],
                [
                    [r.gen_ratio(1, 3), true, r.gen_ratio(1, 3)],
                    [true, true, true],
                    [r.gen_ratio(1, 3), true, r.gen_ratio(1, 3)],
                ],
            ],
            PlantPart::Sapling => shape::box_from_levels([[1, 1, 1], [1, 2, 1], [1, 1, 1]]),
            PlantPart::HeavyBranch { connectivity: d } => {
                let c = map.neighbouring(*coords, |tile, _| {
                    if let Some(tile) = tile {
                        if let TileKind::Plant(plant) = &tile.kind {
                            if matches!(
                                plant,
                                PlantTile {
                                    part: PlantPart::LightBranch,
                                    ..
                                }
                            ) {
                                return plant.origin == self.origin;
                            }
                        }
                    }
                    false
                });
                let c = Neighbouring {
                    a: c.a,
                    b: c.b,
                    n: c.n || d.n,
                    e: c.e || d.e,
                    s: c.s || d.s,
                    w: c.w || d.w,
                };

                let a = rand::thread_rng().gen_range(0..=4);
                let w = rand::thread_rng().gen_range(0..=2);
                let e = rand::thread_rng().gen_range(0..=2);
                let s = rand::thread_rng().gen_range(0..=2);
                let n = rand::thread_rng().gen_range(0..=2);

                #[rustfmt::skip]
                let shape = [
                    [
                        [false, c.a && a == 0, false],
                        [c.a && a == 1, c.a && a == 2, c.a && a == 3],
                        [false, c.a && a == 4, false],
                    ],
                    [
                        [c.w && w == 0 || c.n && n == 0, c.n && n == 1, c.e && e == 0 || c.n && n == 2],
                        [c.w && w == 1, true, c.e && e == 1],
                        [c.w && w == 2 || c.s && s == 0, c.s && s == 1, c.e && e == 2 || c.s && s == 2],
                    ],
                    [
                        [false, false, false],
                        [false, false, false],
                        [false, false, false]
                    ],
                ];

                shape
            }
            PlantPart::LightBranch => {
                let c = map.neighbouring(*coords, |tile, _| {
                    if let Some(tile) = tile {
                        if let TileKind::Plant(plant) = &tile.kind {
                            if matches!(
                                plant,
                                PlantTile {
                                    part: PlantPart::HeavyBranch { .. },
                                    ..
                                }
                            ) {
                                return plant.origin == self.origin;
                            }
                        }
                    }
                    false
                });

                let a = rand::thread_rng().gen_range(0..=8);
                let w = rand::thread_rng().gen_range(0..=4);
                let e = rand::thread_rng().gen_range(0..=4);
                let s = rand::thread_rng().gen_range(0..=4);
                let n = rand::thread_rng().gen_range(0..=4);

                #[rustfmt::skip]
                let shape = [
                    [
                        [false, c.a && a == 0, false],
                        [c.a && a == 1, c.a && a == 2, c.a && a == 3],
                        [false, c.a && a == 4, false],
                    ],
                    [
                        [c.w && w == 0 || c.n && n == 0, c.n && n == 1, c.e && e == 0 || c.n && n == 2],
                        [c.w && w == 1, false, c.e && e == 1],
                        [c.w && w == 2 || c.s && s == 0, c.s && s == 1, c.e && e == 2 || c.s && s == 2],
                    ],
                    [
                        [false, false, false],
                        [false, false, false],
                        [false, false, false],
                    ],
                ];
                shape
            }
            PlantPart::Twig => {
                shape::box_const(false)
                /*let c = map.neighbouring(*coords, |tile, _| {
                    if let Some(tile) = tile {
                        if let TileKind::Plant(plant) = &tile.kind {
                            return plant.part == PlantPart::LightBranch
                                && plant.origin == self.origin;
                        }
                    }
                    false
                });

                #[rustfmt::skip]
                let shape = [
                    [
                        [false, false, false],
                        [false, false, false],
                        [false, false, false],
                    ],
                    [
                        [false, c.n && r.gen_ratio(1, 2), false],
                        [c.w && r.gen_ratio(1, 2), false, c.e && r.gen_ratio(1, 2)],
                        [false, c.s && r.gen_ratio(1, 2), false],
                    ],
                    [
                        [false, false, false],
                        [false, c.b && r.gen_ratio(1, 2), false],
                        [false, false, false],
                    ],
                ];
                shape*/
            }
        }
    }
}
