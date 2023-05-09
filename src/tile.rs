use crate::{
    map::{Coords, Map},
    palette::{DefaultMaterials, Material},
    rfr::BlockTile,
    shape::{self, Box3D, Rotating},
    tile_plant::PlantTile,
    voxel::{voxels_from_uniform_shape, CollectVoxels},
};
use dfhack_remote::{PlantRawList, TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use rand::Rng;

#[derive(Debug)]
pub struct Tile {
    pub kind: TileKind,
    pub coords: Coords,
}

#[derive(Debug)]
pub enum Shape {
    Fluid(u8),
    Floor { smooth: bool },
    Stair(StairPart),
    Fortification,
    Full,
    Ramp,
}

#[derive(Debug)]
pub enum TileKind {
    Normal(NormalTile),
    Plant(PlantTile),
}

#[derive(Debug)]
pub struct NormalTile {
    pub shape: Shape,
    pub material: Material,
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

impl NormalTile {
    fn get_shape(&self, coords: &Coords, map: &Map) -> Box3D<bool> {
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
                    shape::slice_from_fn(|_,_| r && rng.gen_bool(1.0 / 7.0)),
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
                shape.rotated_by((coords.z % 4) as usize)
            }

            Shape::Ramp => {
                let c = map.neighbouring_flat(*coords, |tile, _| match tile {
                    Some(Tile {
                        kind:
                            TileKind::Normal(NormalTile {
                                shape: Shape::Full | Shape::Fortification,
                                ..
                            }),
                        ..
                    }) => RampContactKind::Wall,
                    Some(Tile {
                        kind:
                            TileKind::Normal(NormalTile {
                                shape: Shape::Ramp, ..
                            }),
                        ..
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
            Shape::Fortification => {
                let conn = map.neighbouring_flat(*coords, |tile, _| {
                    matches!(
                        tile,
                        Some(Tile {
                            kind: TileKind::Normal(NormalTile {
                                shape: Shape::Full | Shape::Fortification,
                                ..
                            }),
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

impl Tile {
    pub fn from_df(tile: &BlockTile, year_tick: i32, plant_raws: &PlantRawList) -> Option<Self> {
        if tile.hidden() {
            return Some(Tile::new_normal(
                tile.coords(),
                Shape::Full,
                Material::Default(DefaultMaterials::Hidden),
            ));
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
            // Grass should likely be handled the same way as plants?
            TiletypeMaterial::GRASS_DARK => {
                return Some(Tile::new_normal(
                    tile.coords(),
                    Shape::Floor { smooth: false },
                    Material::Default(DefaultMaterials::DarkGrass),
                ));
            }
            TiletypeMaterial::GRASS_LIGHT => {
                return Some(Tile::new_normal(
                    tile.coords(),
                    Shape::Floor { smooth: false },
                    Material::Default(DefaultMaterials::LightGrass),
                ));
            }
            TiletypeMaterial::GRASS_DRY | TiletypeMaterial::GRASS_DEAD => {
                return Some(Tile::new_normal(
                    tile.coords(),
                    Shape::Floor { smooth: false },
                    Material::Default(DefaultMaterials::DeadGrass),
                ));
            }
            _ => {}
        };

        // If it's a plant, build a plant material
        if tile.material().mat_type() == 419 {
            return Some(Tile::new_plant(tile.coords(), tile, year_tick, plant_raws));
        }

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
            return Some(Tile::new_normal(
                tile.coords(),
                shape,
                Material::Generic(tile.material().clone()),
            ));
        }

        None
    }
    pub fn new_normal(coords: Coords, shape: Shape, material: Material) -> Self {
        Self {
            coords,
            kind: TileKind::Normal(NormalTile { shape, material }),
        }
    }

    pub fn new_water(coords: Coords, level: u8) -> Self {
        Tile::new_normal(
            coords,
            Shape::Fluid(level),
            Material::Default(DefaultMaterials::Water),
        )
    }

    pub fn new_magma(coords: Coords, level: u8) -> Self {
        Tile::new_normal(
            coords,
            Shape::Fluid(level),
            Material::Default(DefaultMaterials::Magma),
        )
    }

    pub fn new_plant(
        coords: Coords,
        tile: &BlockTile,
        year_tick: i32,
        raws: &PlantRawList,
    ) -> Self {
        Self {
            coords,
            kind: TileKind::Plant(PlantTile::from_block_tile(tile, year_tick, raws)),
        }
    }
}

impl CollectVoxels for Tile {
    fn collect_voxels(&self, map: &Map) -> Vec<crate::voxel::Voxel> {
        match &self.kind {
            TileKind::Normal(tile) => voxels_from_uniform_shape(
                tile.get_shape(&self.coords, map),
                self.coords,
                &tile.material,
            ),
            TileKind::Plant(plant) => plant.collect_voxels(&self.coords, map),
        }
    }
}
