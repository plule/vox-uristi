use super::tree::{connectivity_from_direction_string, PlantPart};
use crate::{
    context::DFContext,
    direction::{Neighbouring8Flat, Rotating},
    map::Map,
    palette::{DefaultMaterials, EffectiveMaterial, Material, Palette},
    rfr::BlockTile,
    shape::{box_empty, box_from_levels, slice_empty, slice_from_fn, slice_full, Box3D},
    voxel::{voxels_from_shape, voxels_from_uniform_shape},
    DFMapCoords, IsSomeAnd, StableRng,
};
use dfhack_remote::{TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use easy_ext::ext;
use rand::Rng;

pub fn ramp_shape(map: &Map, coords: DFMapCoords) -> [[[bool; 3]; 3]; 5] {
    let c = map.neighbouring_8flat(coords, |o| {
        o.block_tile
            .as_ref()
            .map(|t| t.ramp_contact_height())
            .unwrap_or(1)
    });
    let nw = c.nw.max(c.n).max(c.w);
    let ne = c.ne.max(c.n).max(c.e);
    let sw = c.sw.max(c.s).max(c.w);
    let se = c.se.max(c.s).max(c.e);

    let c = Neighbouring8Flat {
        n: (nw + ne) / 2,
        ne,
        e: (ne + se) / 2,
        se,
        s: (sw + se) / 2,
        sw,
        w: (nw + sw) / 2,
        nw,
    };

    let max = nw.max(ne).max(sw).max(se);

    let levels = [[c.nw, c.n, c.ne], [c.w, max / 2, c.e], [c.sw, c.s, c.se]];

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

    fn ramp_contact_height(&self) -> usize {
        if self.is_wall() {
            6
        } else {
            1
        }
    }

    fn build_structure(
        &self,
        map: &Map,
        context: &DFContext,
        palette: &mut Palette,
    ) -> Vec<dot_vox::Voxel> {
        let mut rng = self.stable_rng();
        let coords = self.global_coords();
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
                let item_on_tile = map
                    .occupancy
                    .get(&coords)
                    .is_some_and(|t| !t.buildings.is_empty());
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
                let c = map.neighbouring_8flat(coords, |o| o.block_tile.some_and(|t| t.is_wall()));
                // Inside the wall is either the "hidden" material, or the material of the wall if
                // it's transparent. It could be worth avoiding building the whole effective mat here...
                let effective_material = EffectiveMaterial::from_material(&material, context);
                let inside = if effective_material.transparency.is_some() {
                    material.clone()
                } else {
                    Material::Default(DefaultMaterials::Hidden)
                };
                let slice = [
                    [c.n && c.w && c.nw, c.n, c.n && c.e && c.ne],
                    [c.w, true, c.e],
                    [c.s && c.w && c.sw, c.s, c.s && c.e && c.se],
                ]
                .map(|col| {
                    col.map(|b| {
                        Some(if b {
                            palette.get(&inside, context)
                        } else {
                            palette.get(&material, context)
                        })
                    })
                });
                let shape = [slice, slice, slice, slice, slice];
                return voxels_from_shape(shape, self.local_coords());
            }
            TiletypeShape::FORTIFICATION => {
                let conn =
                    map.neighbouring_flat(coords, |o| o.block_tile.some_and(|t| t.is_wall()));
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

        voxels_from_uniform_shape(shape, self.local_coords(), palette.get(&material, context))
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
