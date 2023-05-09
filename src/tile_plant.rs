use crate::{
    direction::{DirectionFlat, NeighbouringFlat},
    map::{Coords, Map},
    palette::{DefaultMaterials, Material},
    rfr::{BlockTile, ConsoleColor, GetTiming},
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
    pub structure_material: Material,
    pub growth_materials: Vec<Material>,
    pub origin: Coords,
}

#[derive(Debug, PartialEq)]
pub enum PlantPart {
    Root,
    Sapling,
    Shrub,
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
            (_, TiletypeShape::SHRUB, _) => PlantPart::Shrub,
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
        // The "structure material" for plants looks like it's always an ugly default brown.
        // For tree, in mat_type 420 is generally the wood, which is nicer.
        // For other plants, use the hard-coded grass one.
        let structure_material = match part {
            PlantPart::Root
            | PlantPart::HeavyBranch { .. }
            | PlantPart::LightBranch
            | PlantPart::Trunk => Material::Generic(MatPair {
                mat_type: Some(420),
                mat_index: Some(plant_index),
                ..Default::default()
            }),
            _ => Material::Default(if alive {
                DefaultMaterials::LightGrass
            } else {
                DefaultMaterials::DeadGrass
            }),
        };

        let origin = tile.tree_origin();
        let mut ret = Self {
            plant_index,
            structure_material,
            part,
            alive,
            origin,
            growth_materials: vec![],
        };
        ret.growth_materials = ret.growth_materials(raws, year_tick).into_iter().collect();
        ret
    }

    pub fn growth_materials(&self, raws: &PlantRawList, year_tick: i32) -> Vec<Material> {
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
                            PlantPart::Shrub => true,
                            PlantPart::Trunk => growth.trunk(),
                            PlantPart::HeavyBranch { .. } => growth.heavy_branches(),
                            PlantPart::LightBranch => growth.light_branches(),
                            PlantPart::Twig => growth.twigs(),
                        }
                })
                .map(|growth| {
                    let material = growth.mat.clone().unwrap_or_default();
                    let current_print = growth
                        .prints
                        .iter()
                        .find(|print| print.timing().contains(&year_tick));
                    let fresh_print = growth
                        .prints
                        .iter()
                        .min_by_key(|print| print.timing_start());
                    match (current_print, fresh_print) {
                        (Some(current_print), Some(fresh_print)) => Material::Plant {
                            material,
                            source_color: fresh_print.get_console_color(),
                            dest_color: current_print.get_console_color(),
                        },
                        _ => Material::Generic(material),
                    }
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
            &self.structure_material,
        );
        if self.alive && !self.growth_materials.is_empty() {
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

    pub fn growth_shape(&self) -> Box3D<bool> {
        let mut r = rand::thread_rng();
        match &self.part {
            PlantPart::Root | PlantPart::Trunk | PlantPart::Cap | PlantPart::HeavyBranch { .. } => {
                [
                    shape::slice_empty(),
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
                    shape::slice_empty(),
                ]
            }
            PlantPart::Twig | PlantPart::LightBranch => [
                shape::slice_empty(),
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
                shape::slice_empty(),
            ],
            PlantPart::Sapling | PlantPart::Shrub => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                    [r.gen_ratio(1, 5), r.gen_ratio(1, 5), r.gen_ratio(1, 5)],
                ],
                shape::slice_empty(),
            ],
        }
    }

    pub fn structure_shape(&self, coords: &Coords, map: &Map) -> Box3D<bool> {
        let mut r = rand::thread_rng();
        // The horror
        match &self.part {
            PlantPart::Trunk | PlantPart::Root | PlantPart::Cap => {
                let on_floor = *coords == self.origin;
                [
                    [
                        [false, true, false],
                        [true, true, true],
                        [false, true, false],
                    ],
                    [
                        [false, true, false],
                        [true, true, true],
                        [false, true, false],
                    ],
                    [
                        [false, true, false],
                        [true, true, true],
                        [false, true, false],
                    ],
                    [
                        [false, true, false],
                        [true, true, true],
                        [false, true, false],
                    ],
                    [
                        [on_floor, true, on_floor],
                        [true, true, true],
                        [on_floor, true, on_floor],
                    ],
                ]
            }
            PlantPart::Sapling | PlantPart::Shrub => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [r.gen_ratio(1, 7), r.gen_ratio(1, 7), r.gen_ratio(1, 7)],
                    [r.gen_ratio(1, 7), r.gen_ratio(1, 7), r.gen_ratio(1, 7)],
                    [r.gen_ratio(1, 7), r.gen_ratio(1, 7), r.gen_ratio(1, 7)],
                ],
                shape::slice_full(),
            ],
            PlantPart::HeavyBranch { connectivity: from } => {
                // light branch connections
                let to = map.neighbouring(*coords, |tile, _| {
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

                #[rustfmt::skip]
                let shape = [
                    [
                        [false, false, false],
                        [false, to.a, false],
                        [false, false, false],
                    ],
                    [
                        [false, false, false],
                        [false, to.a, false],
                        [false, false, false],
                    ],
                    [
                        [false, to.n | from.n, false],
                        [to.w | from.w, true, to.e | from.e],
                        [false, to.s | from.s, false],
                    ],
                    [
                        [false, from.n, false],
                        [from.w, false, from.e],
                        [false, from.s, false],
                    ],
                    shape::slice_empty(),
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
                                    part: PlantPart::HeavyBranch { .. } | PlantPart::Twig,
                                    ..
                                }
                            ) {
                                return plant.origin == self.origin;
                            }
                        }
                    }
                    false
                });

                #[rustfmt::skip]
                let shape = [
                    [
                        [false, false, false],
                        [false, c.a, false],
                        [false, false, false],
                    ],
                    [
                        [false, c.n, false],
                        [c.w, true, c.e],
                        [false, c.s, false],
                    ],
                    [
                        [false, false, false],
                        [false, c.b, false],
                        [false, false, false],
                    ],
                    [
                        [false, false, false],
                        [false, c.b, false],
                        [false, false, false],
                    ],
                    [
                        [false, false, false],
                        [false, c.b, false],
                        [false, false, false],
                    ],
                ];
                shape
            }
            PlantPart::Twig => {
                let c = map.neighbouring(*coords, |tile, _| {
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
                        [false, c.n, false],
                        [c.w, false, c.e],
                        [false, c.s, false],
                    ],
                    shape::slice_empty(),
                    shape::slice_empty(),
                    shape::slice_empty(),
                    [
                        [false, false, false],
                        [false, c.b, false],
                        [false, false, false],
                    ],
                ];
                shape
            }
        }
    }
}
