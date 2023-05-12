use super::{
    corner_ramp_level,
    plant::{connectivity_from_direction_string, PlantPart},
    RampContactKind,
};
use crate::{
    map::Map,
    rfr::BlockTile,
    shape::{
        box_empty, box_from_levels, box_full, slice_empty, slice_from_fn, slice_full, Box3D,
        Rotating,
    },
    IsSomeAnd,
};
use dfhack_remote::{TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use extend::ext;
use rand::Rng;

#[ext]
pub impl BlockTile<'_> {
    fn is_wall(&self) -> bool {
        matches!(
            self.tile_type().shape(),
            TiletypeShape::WALL | TiletypeShape::FORTIFICATION
        )
    }

    fn ramp_contact_kind(&self) -> RampContactKind {
        if self.is_wall() {
            RampContactKind::Wall
        } else if self.tile_type().shape() == TiletypeShape::RAMP {
            RampContactKind::Ramp
        } else {
            RampContactKind::Empty
        }
    }

    fn structure_shape(&self, map: &Map) -> Box3D<bool> {
        let coords = self.coords();
        let tile_type = self.tile_type();
        let mut rng = rand::thread_rng();
        match tile_type.shape() {
            TiletypeShape::FLOOR | TiletypeShape::BOULDER | TiletypeShape::PEBBLES => {
                let r = !matches!(
                    tile_type.special(),
                    TiletypeSpecial::SMOOTH | TiletypeSpecial::SMOOTH_DEAD
                );
                [
                    slice_empty(),
                    slice_empty(),
                    slice_empty(),
                    slice_from_fn(|_, _| r && rng.gen_bool(1.0 / 7.0)),
                    slice_full(),
                ]
            }
            TiletypeShape::WALL => box_full(),
            TiletypeShape::FORTIFICATION => {
                let conn = map.neighbouring_flat(coords, |tile, _| tile.some_and(|t| t.is_wall()));
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
                    slice_full()
                ];
                shape
            }
            TiletypeShape::STAIR_UP => stairs(true, true, false, true, coords.z),
            TiletypeShape::STAIR_DOWN => stairs(false, false, true, false, coords.z),
            TiletypeShape::STAIR_UPDOWN => stairs(true, true, true, false, coords.z),
            TiletypeShape::RAMP => {
                // review for perf
                let c = map.neighbouring_flat(coords, |tile, _| {
                    tile.map(|tile| tile.ramp_contact_kind())
                        .unwrap_or(RampContactKind::Empty)
                });

                #[rustfmt::skip]
                            let levels = [
                                [corner_ramp_level(c.n, c.w) , c.n.height(), corner_ramp_level(c.n, c.e)],
                                [c.w.height()                , 2           , c.e.height()               ],
                                [corner_ramp_level(c.s, c.w) , c.s.height(), corner_ramp_level(c.s, c.e)],
                            ];

                box_from_levels(levels)
            }
            TiletypeShape::TREE_SHAPE => box_empty(), // TODO
            TiletypeShape::SAPLING => box_empty(),    // TODO
            TiletypeShape::SHRUB => box_empty(),      // TODO
            TiletypeShape::BRANCH => box_empty(),     // TODO
            TiletypeShape::TRUNK_BRANCH => box_empty(), // TODO
            TiletypeShape::TWIG => box_empty(),       // TODO
            _ => box_empty(),
        }
    }

    fn plant_part(&self) -> PlantPart {
        let tile_type = self.tile_type();
        match (
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
        }
    }
}

fn stairs(up: bool, middle: bool, down: bool, floor: bool, z: i32) -> Box3D<bool> {
    #[rustfmt::skip]
    let shape = [
        [
            [false, false, false],
            [false, false, false],
            [up, up, up],
        ],
        [
            [false, false, middle],
            [false, false, middle],
            [false, false, middle],
        ],
        [
            [middle, middle, middle],
            [false, false, false],
            [false, false, false]
        ],
        [
            [middle, false, false],
            [middle, false, false],
            [middle, false, false]
        ],
        [
            [floor, floor, floor],
            [floor, floor, floor],
            [down || floor, down || floor, down || floor]
        ],
    ];
    shape.rotated_by((z % 4) as usize)
}
