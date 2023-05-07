use crate::{
    map::{Coords, IsSomeAnd, Map},
    palette::{DefaultMaterials, Material, RandomMaterial},
    rfr::BlockTile,
    shape::{self, Box3D, Rotating},
    voxel::{CollectVoxels, Voxel},
};
use dfhack_remote::{MatPair, TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use itertools::Itertools;
use rand::Rng;

#[derive(Debug)]
pub struct Tile {
    pub shape: Shape,
    pub material: RandomMaterial,
    pub coords: Coords,
}

#[derive(Debug)]
pub enum Shape {
    Fluid(u8),
    Floor { smooth: bool },
    Stair(StairPart),
    Tree { origin: Coords, part: TreePart },
    Fortification,
    Full,
    Ramp,
}

#[derive(Debug, Clone, Copy)]
pub enum TreePart {
    Trunk,
    Branch,
    Twig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StairPart {
    UpDown,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RampContactKind {
    Wall,
    Ramp,
    Empty,
}

impl RampContactKind {
    fn height(&self) -> usize {
        match self {
            RampContactKind::Wall => 3,
            RampContactKind::Ramp => 2,
            RampContactKind::Empty => 1,
        }
    }
}

fn corner_ramp_level(c1: RampContactKind, c2: RampContactKind) -> usize {
    match (c1, c2) {
        (RampContactKind::Ramp, RampContactKind::Ramp) => 2, // should be 1 for concave, 3 for convexe todo
        (RampContactKind::Ramp, c) | (c, RampContactKind::Ramp) => c.height(),
        (c1, c2) => (c1.height() + c2.height()) / 2,
    }
}

impl Tile {
    pub fn new_water(coords: Coords, level: u8) -> Self {
        Self {
            shape: Shape::Fluid(level),
            material: Material::Default(DefaultMaterials::Water).into(),
            coords,
        }
    }

    pub fn new_magma(coords: Coords, level: u8) -> Self {
        Self {
            shape: Shape::Fluid(level),
            material: Material::Default(DefaultMaterials::Magma).into(),
            coords,
        }
    }

    pub fn new_tree(coords: Coords, mat_index: i32, origin: Coords, part: TreePart) -> Self {
        let shape = Shape::Tree { origin, part };
        let wood = MatPair {
            mat_type: Some(420),
            mat_index: Some(mat_index),
            ..Default::default()
        };
        let leaves = MatPair {
            mat_type: Some(421),
            mat_index: Some(mat_index),
            ..Default::default()
        };
        match part {
            TreePart::Trunk => Tile {
                shape,
                material: Material::Generic(wood).into(),
                coords,
            },
            TreePart::Branch => Tile {
                shape,
                material: RandomMaterial::new(vec![
                    Material::Generic(wood),
                    Material::Generic(leaves),
                ]),
                coords,
            },
            TreePart::Twig => Tile {
                shape,
                material: Material::Generic(leaves).into(),
                coords,
            },
        }
    }

    fn get_shape(&self, map: &Map) -> Box3D<3, bool> {
        let mut rng = rand::thread_rng();
        match &self.shape {
            Shape::Fluid(level) => [
                shape::slice_const(*level >= 7),
                shape::slice_const(*level >= 4),
                shape::slice_full(),
            ],
            #[rustfmt::skip]
            Shape::Floor { smooth } => {
                let r = !smooth;
                [
                    shape::slice_empty(),
                    shape::slice_from_fn(|_,_| r && rng.gen()),
                    shape::slice_full(),
                ]
            }

            Shape::Stair(part) => {
                let up = *part == StairPart::Up || *part == StairPart::UpDown;
                let middle = *part == StairPart::Up || *part == StairPart::UpDown;
                let down = *part == StairPart::Down || *part == StairPart::UpDown;
                let floor = *part == StairPart::Up;

                #[rustfmt::skip]
                let shape = [
                    [
                        [false, up, up],
                        [false, false, false],
                        [false, false, false],
                    ],
                    [
                        [false, false, false],
                        [false, false, middle],
                        [false, false, middle]
                    ],
                    [
                        [floor, floor, floor],
                        [floor, floor, floor],
                        [down || floor, down || floor, floor]
                    ],
                ];
                shape.rotated_by((self.coords.z % 4) as usize)
            }

            Shape::Ramp => {
                let c = map.neighbouring_flat(self.coords, |tile, _| match tile {
                    Some(Tile {
                        shape: Shape::Full | Shape::Fortification,
                        ..
                    }) => RampContactKind::Wall,
                    Some(Tile {
                        shape: Shape::Ramp, ..
                    }) => RampContactKind::Ramp,
                    _ => RampContactKind::Empty,
                });

                #[rustfmt::skip]
                let levels = [
                    [corner_ramp_level(c.n, c.w) , c.n.height(), corner_ramp_level(c.n, c.e)],
                    [c.w.height()                , 2           , c.e.height()               ],
                    [corner_ramp_level(c.s, c.w) , c.s.height(), corner_ramp_level(c.s, c.e)],
                ];

                shape::box_from_levels(levels)
            }
            Shape::Tree { origin, part } => {
                let connectivity = map.neighbouring(self.coords, |tile, _| {
                    tile.some_and(|t| match &t.shape {
                        Shape::Tree {
                            origin: other_origin,
                            part: _,
                        } => origin == other_origin,
                        _ => false,
                    })
                });
                match part {
                    TreePart::Trunk => shape::box_full(),
                    TreePart::Branch => [
                        [
                            [rng.gen(), rng.gen(), rng.gen()],
                            [rng.gen(), connectivity.a, rng.gen()],
                            [rng.gen(), rng.gen(), rng.gen()],
                        ],
                        [
                            [rng.gen(), connectivity.n, rng.gen()],
                            [connectivity.w, true, connectivity.e],
                            [rng.gen(), connectivity.s, rng.gen()],
                        ],
                        [
                            [rng.gen(), rng.gen(), rng.gen()],
                            [rng.gen(), connectivity.b, rng.gen()],
                            [rng.gen(), rng.gen(), rng.gen()],
                        ],
                    ],
                    TreePart::Twig => [
                        [
                            [rng.gen(), false, rng.gen()],
                            [false, connectivity.a, false],
                            [rng.gen(), false, rng.gen()],
                        ],
                        [
                            [rng.gen(), connectivity.n, rng.gen()],
                            [connectivity.w, true, connectivity.e],
                            [rng.gen(), connectivity.s, rng.gen()],
                        ],
                        [
                            [rng.gen(), false, rng.gen()],
                            [false, connectivity.b, false],
                            [rng.gen(), false, rng.gen()],
                        ],
                    ],
                }
            }
            Shape::Fortification => {
                let conn = map.neighbouring_flat(self.coords, |tile, _| {
                    matches!(
                        tile,
                        Some(Tile {
                            shape: Shape::Full | Shape::Fortification,
                            ..
                        })
                    )
                });
                #[rustfmt::skip]
                let shape = [
                    [
                        [true, conn.n, true],
                        [conn.w, false, conn.e],
                        [true, conn.s, true]
                    ],
                    [
                        [true, conn.n, true],
                        [conn.w, false, conn.e],
                        [true, conn.s, true]
                    ],
                    shape::slice_full()
                ];

                shape
            }
            Shape::Full => shape::box_full(),
        }
    }
}

impl CollectVoxels for Tile {
    fn collect_voxels(&self, map: &Map) -> Vec<crate::voxel::Voxel> {
        let shape = self.get_shape(map);
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
            .map(|coords| Voxel::new(coords, self.material.pick()))
            .collect_vec()
    }
}

impl<'a> From<&'a BlockTile<'a>> for Option<Tile> {
    fn from(tile: &BlockTile) -> Self {
        if tile.hidden() {
            return Some(Tile {
                shape: Shape::Full,
                material: Material::Default(DefaultMaterials::Hidden).into(),
                coords: tile.coords(),
            });
        }
        // Check if it's a fluid, in that case, ignore what's below
        match tile.tile_type().material() {
            TiletypeMaterial::MAGMA => {
                return Some(Tile::new_magma(
                    tile.coords(),
                    tile.magma().try_into().unwrap_or_default(),
                ))
            }
            TiletypeMaterial::POOL | TiletypeMaterial::BROOK | TiletypeMaterial::RIVER => {
                return Some(Tile::new_water(
                    tile.coords(),
                    tile.water().try_into().unwrap_or_default(),
                ))
            }
            TiletypeMaterial::TREE_MATERIAL => {
                let mat_pair_index = tile.material().mat_index();
                match tile.tile_type().shape() {
                    TiletypeShape::WALL | TiletypeShape::TRUNK_BRANCH => {
                        return Some(Tile::new_tree(
                            tile.coords(),
                            mat_pair_index,
                            tile.tree_origin(),
                            TreePart::Trunk,
                        ));
                    }
                    TiletypeShape::BRANCH | TiletypeShape::RAMP => {
                        return Some(Tile::new_tree(
                            tile.coords(),
                            mat_pair_index,
                            tile.tree_origin(),
                            TreePart::Branch,
                        ));
                    }
                    TiletypeShape::TWIG => {
                        return Some(Tile::new_tree(
                            tile.coords(),
                            mat_pair_index,
                            tile.tree_origin(),
                            TreePart::Twig,
                        ));
                    }
                    _ => {}
                }
            }
            TiletypeMaterial::GRASS_DARK => {
                return Some(Tile {
                    coords: tile.coords(),
                    shape: Shape::Floor { smooth: false },
                    material: Material::Default(DefaultMaterials::DarkGrass).into(),
                })
            }
            TiletypeMaterial::GRASS_LIGHT => {
                return Some(Tile {
                    coords: tile.coords(),
                    shape: Shape::Floor { smooth: false },
                    material: Material::Default(DefaultMaterials::LightGrass).into(),
                })
            }
            _ => {}
        };

        // Some fluid tile just have the fluid amount indic
        if tile.water() == 7 {
            return Some(Tile::new_water(
                tile.coords(),
                tile.water().try_into().unwrap_or_default(),
            ));
        }

        if tile.magma() == 7 {
            return Some(Tile::new_magma(
                tile.coords(),
                tile.magma().try_into().unwrap_or_default(),
            ));
        }

        // Not a fluid, check if it has a solid shape and a material
        if let Some(shape) = match tile.tile_type().shape() {
            TiletypeShape::FLOOR
            | TiletypeShape::BOULDER
            | TiletypeShape::PEBBLES
            | TiletypeShape::SHRUB
            | TiletypeShape::SAPLING => Some(Shape::Floor {
                smooth: tile.tile_type().special() == TiletypeSpecial::SMOOTH,
            }),
            TiletypeShape::RAMP => Some(Shape::Ramp),
            TiletypeShape::STAIR_UPDOWN => Some(Shape::Stair(StairPart::UpDown)),
            TiletypeShape::STAIR_UP => Some(Shape::Stair(StairPart::Up)),
            TiletypeShape::STAIR_DOWN => Some(Shape::Stair(StairPart::Down)),
            TiletypeShape::FORTIFICATION => Some(Shape::Fortification),
            TiletypeShape::WALL => Some(Shape::Full),
            _ => None,
        } {
            return Some(Tile {
                coords: tile.coords(),
                shape,
                material: Material::Generic(tile.material().clone()).into(),
            });
        }

        None
    }
}
