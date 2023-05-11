use super::Tile;
use crate::{
    direction::{DirectionFlat, NeighbouringFlat},
    export::ExportSettings,
    map::{IsSomeAnd, Map},
    palette::{DefaultMaterials, Material},
    rfr::{ConsoleColor, GetTiming},
    shape::{self, slice_empty, slice_from_fn, slice_full, Box3D},
    voxel::{voxels_from_shape, voxels_from_uniform_shape, Voxel},
};
use dfhack_remote::{MatPair, PlantRawList, TiletypeMaterial, TiletypeSpecial};
use rand::{seq::SliceRandom, Rng};

impl Tile<'_> {
    pub fn collect_plant_voxels(
        &self,
        map: &Map,
        settings: &ExportSettings,
        raws: &PlantRawList,
    ) -> Vec<Voxel> {
        let material = match self.0.tile_type().material() {
            TiletypeMaterial::GRASS_LIGHT => Material::Default(DefaultMaterials::LightGrass),
            TiletypeMaterial::GRASS_DARK => Material::Default(DefaultMaterials::DarkGrass),
            TiletypeMaterial::GRASS_DRY | TiletypeMaterial::GRASS_DEAD => {
                Material::Default(DefaultMaterials::DeadGrass)
            }
            _ => {
                return self.collect_tree_voxels(map, settings, raws);
            }
        };
        let mut rng = rand::thread_rng();
        let shape: Box3D<bool> = [
            slice_empty(),
            slice_empty(),
            slice_empty(),
            slice_from_fn(|_, _| rng.gen_bool(1.0 / 7.0)),
            slice_full(),
        ];

        voxels_from_uniform_shape(shape, self.0.coords(), material)
    }

    pub fn collect_tree_voxels(
        &self,
        map: &Map,
        settings: &ExportSettings,
        raws: &PlantRawList,
    ) -> Vec<Voxel> {
        let part = self.plant_part();
        let coords = self.0.coords();
        let mut rng = rand::thread_rng();
        let tile_type = self.0.tile_type();
        let plant_index = self.0.material().mat_index();
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
        let mut voxels = voxels_from_uniform_shape(
            self.plant_structure_shape(&part, map),
            coords,
            structure_material,
        );
        let growth_materials = self.growth_materials(&part, raws, settings.year_tick);
        if alive && !growth_materials.is_empty() {
            let growth = Tile::growth_shape(&part).map(|slice| {
                slice.map(|col| {
                    col.map(|t| {
                        if t {
                            growth_materials.choose(&mut rng).cloned()
                        } else {
                            None
                        }
                    })
                })
            });
            voxels.append(&mut voxels_from_shape(growth, coords));
        }
        voxels
    }

    pub fn plant_structure_shape(&self, part: &PlantPart, map: &Map) -> Box3D<bool> {
        let mut r = rand::thread_rng();
        let coords = self.0.coords();
        let origin = self.0.tree_origin();
        // The horror
        match part {
            PlantPart::Trunk | PlantPart::Root | PlantPart::Cap => {
                let on_floor = coords == origin;
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
                let to = map.neighbouring(coords, |tile, _| {
                    tile.some_and(|tile| {
                        tile.0.tree_origin() == origin
                            && tile.plant_part() == PlantPart::LightBranch
                    })
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
                let c = map.neighbouring(coords, |tile, _| {
                    tile.some_and(|tile| {
                        tile.0.tree_origin() == origin
                            && matches!(
                                tile.plant_part(),
                                PlantPart::HeavyBranch { .. } | PlantPart::Twig
                            )
                    })
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
                let c = map.neighbouring(coords, |tile, _| {
                    tile.some_and(|tile| {
                        tile.0.tree_origin() == origin
                            && tile.plant_part() == PlantPart::LightBranch
                    })
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

    pub fn growth_shape(part: &PlantPart) -> Box3D<bool> {
        let mut r = rand::thread_rng();
        match part {
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

    pub fn growth_materials(
        &self,
        part: &PlantPart,
        raws: &PlantRawList,
        year_tick: i32,
    ) -> Vec<Material> {
        let plant_index = self.0.material().mat_index();
        if let Some(plant_raw) = raws.plant_raws.get(plant_index as usize) {
            plant_raw
                .growths
                .iter()
                .filter(|growth| {
                    growth.timing().contains(&year_tick)
                        && match part {
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

pub fn connectivity_from_direction_string(direction_string: &str) -> NeighbouringFlat<bool> {
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
