use super::{
    corner_ramp_level,
    tree::{connectivity_from_direction_string, PlantPart},
    RampContactKind,
};
use crate::{
    direction::Rotating,
    map::Map,
    palette::{DefaultMaterials, Material},
    rfr::BlockTile,
    shape::{box_empty, box_from_levels, slice_empty, slice_from_fn, slice_full, Box3D},
    voxel::{voxels_from_shape, voxels_from_uniform_shape, Voxel},
    DFCoords, IsSomeAnd, StableRng,
};
use dfhack_remote::{TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use easy_ext::ext;
use rand::Rng;

pub fn ramp_shape(map: &Map, coords: DFCoords) -> [[[bool; 3]; 3]; 5] {
    let c = map.neighbouring_flat(coords, |tile| {
        tile.block_tile
            .as_ref()
            .map(|tile| tile.ramp_contact_kind())
            .unwrap_or(RampContactKind::Empty)
    });

    #[rustfmt::skip]
                let levels = [
                    [corner_ramp_level(c.n, c.w) , c.n.height(), corner_ramp_level(c.n, c.e)],
                    [c.w.height()                , 3           , c.e.height()               ],
                    [corner_ramp_level(c.s, c.w) , c.s.height(), corner_ramp_level(c.s, c.e)],
                ];

    box_from_levels(levels)
}

#[ext(BlockTileExt)]
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

    fn collect_structure_voxels(&self, map: &Map) -> Vec<Voxel> {
        let mut rng = self.stable_rng();
        let coords = self.coords();
        let tile_type = self.tile_type();
        let material = match self.tile_type().material() {
            // Grass don't have proper materials in the raw
            TiletypeMaterial::GRASS_LIGHT => Material::Default(DefaultMaterials::LightGrass),
            TiletypeMaterial::GRASS_DARK => Material::Default(DefaultMaterials::DarkGrass),
            TiletypeMaterial::GRASS_DRY | TiletypeMaterial::GRASS_DEAD => {
                Material::Default(DefaultMaterials::DeadGrass)
            }
            // Generic material from raw
            mat => Material::TileGeneric(self.material().clone(), mat),
        };
        let shape = match tile_type.shape() {
            TiletypeShape::FLOOR | TiletypeShape::BOULDER | TiletypeShape::PEBBLES => {
                let item_on_tile = map.with_building.contains(&coords);
                let rough = !item_on_tile // no roughness if there is a rendered item
                    && tile_type.material() != TiletypeMaterial::FROZEN_LIQUID // no roughness for ice, it looks bad
                    && !matches!(
                        tile_type.special(),
                        TiletypeSpecial::SMOOTH | TiletypeSpecial::SMOOTH_DEAD
                    );
                [
                    slice_empty(),
                    slice_empty(),
                    slice_empty(),
                    slice_from_fn(|_, _| rough && rng.gen_bool(1.0 / 7.0)),
                    slice_full(),
                ]
            }
            TiletypeShape::WALL => {
                let c = map
                    .neighbouring_8flat(coords, |tile| tile.block_tile.some_and(|t| t.is_wall()));
                let void = Material::Default(DefaultMaterials::Hidden);
                let slice = [
                    [c.n && c.w && c.nw, c.n, c.n && c.e && c.ne],
                    [c.w, true, c.e],
                    [c.s && c.w && c.sw, c.s, c.s && c.e && c.se],
                ]
                .map(|col| col.map(|b| Some(if b { void.clone() } else { material.clone() })));
                let shape = [
                    slice.clone(),
                    slice.clone(),
                    slice.clone(),
                    slice.clone(),
                    slice,
                ];
                return voxels_from_shape(shape, self.coords());
            }
            TiletypeShape::FORTIFICATION => {
                let conn =
                    map.neighbouring_flat(coords, |tile| tile.block_tile.some_and(|t| t.is_wall()));
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
            TiletypeShape::RAMP => ramp_shape(map, coords),
            TiletypeShape::TREE_SHAPE => box_empty(), // TODO
            TiletypeShape::SAPLING => box_empty(),    // TODO
            TiletypeShape::SHRUB => box_empty(),      // TODO
            TiletypeShape::BRANCH => box_empty(),     // TODO
            TiletypeShape::TRUNK_BRANCH => box_empty(), // TODO
            TiletypeShape::TWIG => box_empty(),       // TODO
            _ => box_empty(),
        };

        voxels_from_uniform_shape(shape, self.coords(), material)
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
