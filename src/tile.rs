use crate::{
    map::{Coords, Direction, Map},
    palette::{Material, Palette},
    rfr::{DFTile, MatPairHash},
};
use dfhack_remote::{TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use itertools::Itertools;
use rand::Rng;

#[derive(Debug)]
pub struct Tile {
    pub shape: Shape,
    pub material: Material,
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
    fn height(&self) -> i8 {
        match self {
            RampContactKind::Wall => 3,
            RampContactKind::Ramp => 2,
            RampContactKind::Empty => 1,
        }
    }
}

fn ramp_status_at(map: &Map, coords: &Coords) -> RampContactKind {
    if let Some(tile) = map.tiles.get(coords) {
        return match tile.shape {
            Shape::Full | Shape::Fortification => RampContactKind::Wall,
            Shape::Ramp => RampContactKind::Ramp,
            _ => RampContactKind::Empty,
        };
    };
    RampContactKind::Empty
}

fn corner_ramp_level(c1: RampContactKind, c2: RampContactKind) -> i8 {
    match (c1, c2) {
        (RampContactKind::Ramp, RampContactKind::Ramp) => 2, // should be 1 for concave, 3 for convexe todo
        (RampContactKind::Ramp, c) | (c, RampContactKind::Ramp) => c.height(),
        (c1, c2) => (c1.height() + c2.height()) / 2
    }
}

impl Tile {
    pub fn new_water(coords: Coords, level: u8) -> Self {
        Self {
            shape: Shape::Fluid(level),
            material: Material::Water,
            coords,
        }
    }

    pub fn new_magma(coords: Coords, level: u8) -> Self {
        Self {
            shape: Shape::Fluid(level),
            material: Material::Magma,
            coords,
        }
    }

    pub fn new_tree(coords: Coords, mat_index: i32, origin: Coords, part: TreePart) -> Self {
        let shape = Shape::Tree { origin, part };
        let wood = MatPairHash::new(420, mat_index);
        let leaves = MatPairHash::new(421, mat_index);
        match part {
            TreePart::Trunk => Tile {
                shape,
                material: Material::Generic(vec![wood]),
                coords,
            },
            TreePart::Branch => Tile {
                shape,
                material: Material::Generic(vec![wood, leaves]),
                coords,
            },
            TreePart::Twig => Tile {
                shape,
                material: Material::Generic(vec![leaves]),
                coords,
            },
        }
    }

    fn get_shape(&self, map: &Map) -> [[[bool; 3]; 3]; 3] {
        let mut rng = rand::thread_rng();
        match &self.shape {
            Shape::Fluid(level) => {
                let lvl1 = *level >= 4;
                let lvl2 = *level >= 7;
                [
                    [[lvl2, lvl2, lvl2], [lvl2, lvl2, lvl2], [lvl2, lvl2, lvl2]],
                    [[lvl1, lvl1, lvl1], [lvl1, lvl1, lvl1], [lvl1, lvl1, lvl1]],
                    [[true, true, true], [true, true, true], [true, true, true]],
                ]
            }
            #[rustfmt::skip]
            Shape::Floor { smooth } => {
                let r = !smooth;
                [
                    [
                        [false, false, false],
                        [false, false, false],
                        [false, false, false],
                    ],
                    [
                        [r && rng.gen(), r && rng.gen(), r && rng.gen()],
                        [r && rng.gen(), r && rng.gen(), r && rng.gen()],
                        [r && rng.gen(), r && rng.gen(), r && rng.gen()],
                    ],
                    [
                        [true, true, true],
                        [true, true, true],
                        [true, true, true]
                    ],
                ]
            }
            #[rustfmt::skip]
            Shape::Stair(part) => {
                let up = *part == StairPart::Up || *part == StairPart::UpDown;
                let middle = *part == StairPart::Up || *part == StairPart::UpDown;
                let down = *part == StairPart::Down || *part == StairPart::UpDown;
                let floor = *part == StairPart::Up;
                
                [
                    [
                        [up, up, false],
                        [false, false, false],
                        [false, false, false],
                    ],
                    [
                        [false, false, false],
                        [middle, false, false],
                        [middle, false, false]
                    ],
                    [
                        [floor, floor, down | floor],
                        [floor, floor, down | floor],
                        [floor, down | floor, down | floor]
                    ],
                ]
            }

            Shape::Ramp => {
                let n = ramp_status_at(map, &(self.coords + Direction::North));
                let s = ramp_status_at(map, &(self.coords + Direction::South));
                let e = ramp_status_at(map, &(self.coords + Direction::East));
                let w = ramp_status_at(map, &(self.coords + Direction::West));

                let levels = [
                    [corner_ramp_level(n, w) , n.height(), corner_ramp_level(n, e)],
                    [w.height()      , 2                 , e.height()             ],
                    [corner_ramp_level(s, w) , s.height(), corner_ramp_level(s, e)],
                ];

                [2, 1, 0].map(|z| [0,1,2].map(|y| [0,1,2].map(|x| levels[y][x] > z)))
            }
            Shape::Tree { origin, part } => {
                let a = map.has_tree_at_coords(&(self.coords + Direction::Above), origin);
                let n = map.has_tree_at_coords(&(self.coords + Direction::North), origin);
                let w = map.has_tree_at_coords(&(self.coords + Direction::West), origin);
                let e = map.has_tree_at_coords(&(self.coords + Direction::East), origin);
                let s = map.has_tree_at_coords(&(self.coords + Direction::South), origin);
                let b = map.has_tree_at_coords(&(self.coords + Direction::Below), origin);
                match part {
                    TreePart::Trunk => [
                        [[true, true, true], [true, true, true], [true, true, true]],
                        [[true, true, true], [true, true, true], [true, true, true]],
                        [[true, true, true], [true, true, true], [true, true, true]],
                    ],
                    TreePart::Branch => [
                        [
                            [rng.gen(), rng.gen(), rng.gen()],
                            [rng.gen(), a, rng.gen()],
                            [rng.gen(), rng.gen(), rng.gen()],
                        ],
                        [
                            [rng.gen(), n, rng.gen()],
                            [w, true, e],
                            [rng.gen(), s, rng.gen()],
                        ],
                        [
                            [rng.gen(), rng.gen(), rng.gen()],
                            [rng.gen(), b, rng.gen()],
                            [rng.gen(), rng.gen(), rng.gen()],
                        ],
                    ],
                    TreePart::Twig => [
                        [
                            [rng.gen(), false, rng.gen()],
                            [false, a, false],
                            [rng.gen(), false, rng.gen()],
                        ],
                        [
                            [rng.gen(), n, rng.gen()],
                            [w, true, e],
                            [rng.gen(), s, rng.gen()],
                        ],
                        [
                            [rng.gen(), false, rng.gen()],
                            [false, b, false],
                            [rng.gen(), false, rng.gen()],
                        ],
                    ],
                }
            }
            #[rustfmt::skip]
            Shape::Fortification => [
                [
                    [true, false, true],
                    [false, false, false],
                    [true, false, true]
                ],
                [
                    [true, false, true],
                    [false, false, false],
                    [true, false, true]
                ],
                [
                    [true, true, true],
                    [true, true, true],
                    [true, true, true]
                ],
            ],
            Shape::Full => [
                [[true, true, true], [true, true, true], [true, true, true]],
                [[true, true, true], [true, true, true], [true, true, true]],
                [[true, true, true], [true, true, true], [true, true, true]],
            ],
        }
    }

    pub fn is_from_tree(&self, other_tree: &Coords) -> bool {
        match &self.shape {
            Shape::Tree { origin, part: _ } => origin == other_tree,
            _ => false,
        }
    }

    pub fn collect_voxels(&self, palette: &Palette, map: &Map) -> Vec<(Coords, u8)> {
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
            .map(|coords| (coords, self.material.pick_color(&palette.colors)))
            .collect_vec()
    }
}

impl<'a> From<&'a DFTile<'a>> for Option<Tile> {
    fn from(tile: &DFTile) -> Self {
        if tile.hidden {
            return Some(Tile {
                shape: Shape::Full,
                material: Material::Hidden,
                coords: tile.coords,
            });
        }
        // Check if it's a fluid, in that case, ignore what's below
        match tile.tile_type.material() {
            TiletypeMaterial::MAGMA => {
                return Some(Tile::new_magma(
                    tile.coords,
                    tile.magma.try_into().unwrap_or_default(),
                ))
            }
            TiletypeMaterial::POOL | TiletypeMaterial::BROOK | TiletypeMaterial::RIVER => {
                return Some(Tile::new_water(
                    tile.coords,
                    tile.water.try_into().unwrap_or_default(),
                ))
            }
            TiletypeMaterial::TREE_MATERIAL => {
                let mat_pair_index = tile.material.map(|mat| mat.mat_pair.mat_index());
                if let Some(mat_pair_index) = mat_pair_index {
                    match tile.tile_type.shape() {
                        TiletypeShape::WALL | TiletypeShape::TRUNK_BRANCH => {
                            return Some(Tile::new_tree(
                                tile.coords,
                                mat_pair_index,
                                tile.tree_origin,
                                TreePart::Trunk,
                            ));
                        }
                        TiletypeShape::BRANCH | TiletypeShape::RAMP => {
                            return Some(Tile::new_tree(
                                tile.coords,
                                mat_pair_index,
                                tile.tree_origin,
                                TreePart::Branch,
                            ));
                        }
                        TiletypeShape::TWIG => {
                            return Some(Tile::new_tree(
                                tile.coords,
                                mat_pair_index,
                                tile.tree_origin,
                                TreePart::Twig,
                            ));
                        }
                        _ => {}
                    }
                }
            }
            TiletypeMaterial::GRASS_DARK => {
                return Some(Tile {
                    coords: tile.coords,
                    shape: Shape::Floor { smooth: false },
                    material: Material::DarkGrass,
                })
            }
            TiletypeMaterial::GRASS_LIGHT => {
                return Some(Tile {
                    coords: tile.coords,
                    shape: Shape::Floor { smooth: false },
                    material: Material::LightGrass,
                })
            }
            _ => {}
        };

        // Some fluid tile just have the fluid amount indic
        if tile.water == 7 {
            return Some(Tile::new_water(
                tile.coords,
                tile.water.try_into().unwrap_or_default(),
            ));
        }

        if tile.magma == 7 {
            return Some(Tile::new_magma(
                tile.coords,
                tile.magma.try_into().unwrap_or_default(),
            ));
        }

        // Not a fluid, check if it has a solid shape and a material
        if let Some(material) = tile.material {
            if let Some(shape) = match tile.tile_type.shape() {
                TiletypeShape::FLOOR
                | TiletypeShape::BOULDER
                | TiletypeShape::PEBBLES
                | TiletypeShape::SHRUB
                | TiletypeShape::SAPLING => Some(Shape::Floor {
                    smooth: tile.tile_type.special() == TiletypeSpecial::SMOOTH,
                }),
                TiletypeShape::RAMP => Some(Shape::Ramp),
                TiletypeShape::STAIR_UPDOWN => Some(Shape::Stair(StairPart::UpDown)),
                TiletypeShape::STAIR_UP => Some(Shape::Stair(StairPart::Up)),
                TiletypeShape::STAIR_DOWN => Some(Shape::Stair(StairPart::Down)),
                TiletypeShape::FORTIFICATION => Some(Shape::Fortification),
                TiletypeShape::WALL => Some(Shape::Full),
                _ => None,
            } {
                let material: MatPairHash = material.mat_pair.clone().unwrap_or_default().into();
                return Some(Tile {
                    coords: tile.coords,
                    shape,
                    material: Material::Generic(vec![material]),
                });
            }
        }

        None
    }
}
