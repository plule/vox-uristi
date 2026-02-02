use super::BlockTileExt;
use crate::{
    direction::{DirectionFlat, NeighbouringFlat},
    export::{DFContext, DefaultMaterials, Map, Material, Palette},
    rfr::{BlockTile, ConsoleColor, GetTiming},
    shape::{self, Box3D},
    voxel::{voxels_from_shape, voxels_from_uniform_shape},
    StableRng,
};
use dfhack_remote::{MatPair, TiletypeSpecial};
use easy_ext::ext;
use itertools::Itertools;
use rand::{rngs::StdRng, seq::IndexedRandom, Rng};

#[ext(BlockTilePlantExt)]
pub impl BlockTile<'_> {
    fn build_trees(
        &self,
        map: &Map,
        context: &DFContext,
        palette: &mut Palette,
    ) -> Vec<dot_vox::Voxel> {
        let mut rng = self.stable_rng();
        let part = self.plant_part();
        let tile_type = self.tile_type();
        let plant_index = self.material().mat_index();
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
                DefaultMaterials::Plant
            } else {
                DefaultMaterials::DeadPlant
            }),
        };
        let mut voxels = voxels_from_uniform_shape(
            self.plant_structure_shape(&part, map),
            self.local_coords(),
            palette.get(&structure_material, context),
        );
        let growth_materials = self
            .growth_materials(&part, context)
            .into_iter()
            .map(|m| palette.get(&m, context))
            .collect_vec();
        if alive && !growth_materials.is_empty() {
            let growth = BlockTile::growth_shape(&part, &mut rng).map(|slice| {
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
            voxels.append(&mut voxels_from_shape(growth, self.local_coords()));
        }
        voxels
    }

    fn plant_structure_shape(&self, part: &PlantPart, map: &Map) -> Box3D<bool> {
        let mut r = self.stable_rng();
        let coords = self.global_coords();
        let origin = self.tree_origin();
        // The horror
        match part {
            PlantPart::Root => shape::box_full(),
            PlantPart::Trunk | PlantPart::Cap => {
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
                    [
                        r.random_ratio(1, 7),
                        r.random_ratio(1, 7),
                        r.random_ratio(1, 7),
                    ],
                    [
                        r.random_ratio(1, 7),
                        r.random_ratio(1, 7),
                        r.random_ratio(1, 7),
                    ],
                    [
                        r.random_ratio(1, 7),
                        r.random_ratio(1, 7),
                        r.random_ratio(1, 7),
                    ],
                ],
                shape::slice_full(),
            ],
            PlantPart::HeavyBranch { connectivity: from } => {
                // light branch connections
                let to = map.neighbouring(coords, |o| {
                    o.block_tile.as_ref().is_some_and(|t| {
                        t.tree_origin() == origin && t.plant_part() == PlantPart::LightBranch
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
                let c = map.neighbouring(coords, |o| {
                    o.block_tile.as_ref().is_some_and(|t| {
                        t.tree_origin() == origin
                            && matches!(
                                t.plant_part(),
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
                let c = map.neighbouring(coords, |o| {
                    o.block_tile.as_ref().is_some_and(|t| {
                        t.tree_origin() == origin && t.plant_part() == PlantPart::LightBranch
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

    fn growth_shape(part: &PlantPart, r: &mut StdRng) -> Box3D<bool> {
        match part {
            PlantPart::Root | PlantPart::Trunk | PlantPart::Cap | PlantPart::HeavyBranch { .. } => {
                [
                    shape::slice_empty(),
                    [
                        [r.random_ratio(1, 5), false, r.random_ratio(1, 5)],
                        [false, false, false],
                        [r.random_ratio(1, 5), false, r.random_ratio(1, 5)],
                    ],
                    [
                        [r.random_ratio(1, 5), false, r.random_ratio(1, 5)],
                        [false, false, false],
                        [r.random_ratio(1, 5), false, r.random_ratio(1, 5)],
                    ],
                    [
                        [r.random_ratio(1, 5), false, r.random_ratio(1, 5)],
                        [false, false, false],
                        [r.random_ratio(1, 5), false, r.random_ratio(1, 5)],
                    ],
                    shape::slice_empty(),
                ]
            }
            PlantPart::Twig | PlantPart::LightBranch => [
                shape::slice_empty(),
                [
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                ],
                [
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                    [r.random_ratio(1, 5), true, r.random_ratio(1, 5)],
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                ],
                [
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                ],
                shape::slice_empty(),
            ],
            PlantPart::Sapling | PlantPart::Shrub => [
                shape::slice_empty(),
                shape::slice_empty(),
                shape::slice_empty(),
                [
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                    [
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                        r.random_ratio(1, 5),
                    ],
                ],
                shape::slice_empty(),
            ],
        }
    }

    fn growth_materials(&self, part: &PlantPart, context: &DFContext) -> Vec<Material> {
        let plant_index = self.material().mat_index();
        if let Some(plant_raw) = context.plant_raws.plant_raws.get(plant_index as usize) {
            plant_raw
                .growths
                .iter()
                .filter(|growth| {
                    growth.timing().contains(&context.settings.year_tick)
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
                        .find(|print| print.timing().contains(&context.settings.year_tick));
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
