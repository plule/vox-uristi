use super::tree::{connectivity_from_direction_string, PlantPart};
use crate::{
    direction::Rotating,
    export::{
        tile::ramp_levels, DFContext, DefaultMaterials, EffectiveMaterial, Map, Material, Palette,
    },
    rfr::BlockTile,
    shape::{
        box_const, box_empty, box_from_fn, box_from_levels, box_from_shape_fn, box_map,
        slice_const, slice_from_fn, slice_full, Box3D,
    },
    voxel::voxels_from_shape,
    DFMapCoords, StableRng, WithDFCoords,
};
use dfhack_remote::{TiletypeMaterial, TiletypeShape, TiletypeSpecial};
use easy_ext::ext;
use rand::seq::IndexedRandom;

pub fn ramp_shape(map: &Map, coords: DFMapCoords) -> [[[bool; 3]; 3]; 5] {
    let levels = ramp_levels(map, coords);

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

    // Returns a tuple with the terrain voxels
    fn build_terrain(
        &self,
        map: &Map,
        context: &DFContext,
        palette: &mut Palette,
    ) -> Vec<dot_vox::Voxel> {
        let mut rng = self.stable_rng();
        let coords = self.global_coords();
        let tile_type = self.tile_type();
        let material = Material::TileGeneric(self.material().clone(), self.tile_type().material());
        let material_dark =
            Material::DarkTileGeneric(self.material().clone(), self.tile_type().material());

        let rough = !matches!(
            tile_type.special(),
            TiletypeSpecial::SMOOTH | TiletypeSpecial::SMOOTH_DEAD // smoothed surface
        ) && !matches!(
            tile_type.material(),
            TiletypeMaterial::CONSTRUCTION /* constructed surface or ramp */ | TiletypeMaterial::FROZEN_LIQUID /* exclude ice it looks bad */
        );

        let engraved = map
            .occupancy
            .get(&self.coords())
            .is_some_and(|o| o.engraving.is_some());

        let mat1 = palette.get(&material, context);
        let mat2 = if engraved || rough {
            palette.get(&material_dark, context)
        } else {
            0
        };
        let mut rand_mat = || *[mat1, mat2].choose(&mut rng).unwrap();

        let shape: Box3D<bool> = match tile_type.shape() {
            TiletypeShape::FLOOR | TiletypeShape::BOULDER | TiletypeShape::PEBBLES => {
                let floor_slice = if engraved {
                    slice_from_fn(|x, y| Some(if (x + y) % 2 == 1 { mat1 } else { mat2 }))
                } else if rough {
                    slice_from_fn(|_, _| Some(rand_mat()))
                } else {
                    slice_const(Some(mat1))
                };

                let shape: Box3D<Option<u8>> = [
                    slice_const(None),
                    slice_const(None),
                    slice_const(None),
                    slice_const(None),
                    floor_slice,
                ];

                return voxels_from_shape(shape, self.local_coords());
            }
            TiletypeShape::WALL => {
                // Build the wall shape
                let mut wall_shape: Box3D<u8> = if rough {
                    box_from_fn(|_, _, _| rand_mat())
                } else if engraved {
                    [
                        slice_from_fn(|x, y| if (x + y) % 2 == 0 { mat1 } else { mat2 }),
                        slice_from_fn(|x, y| if (x + y) % 2 == 1 { mat1 } else { mat2 }),
                        slice_from_fn(|x, y| if (x + y) % 2 == 0 { mat1 } else { mat2 }),
                        slice_from_fn(|x, y| if (x + y) % 2 == 0 { mat1 } else { mat2 }),
                        slice_const(mat1),
                    ]
                } else {
                    box_const(mat1)
                };

                // Replace the inside with the "void" material unless it's a transparent material
                // improvement: not build the whole material here
                let effective_material = EffectiveMaterial::from_material(&material, context);
                if effective_material.transparency.is_none() {
                    let c = map.neighbouring_8flat(coords, |o| {
                        o.block_tile.as_ref().is_some_and(|t| t.is_wall())
                    });
                    let inside_slice = [
                        [c.n && c.w && c.nw, c.n, c.n && c.e && c.ne],
                        [c.w, true, c.e],
                        [c.s && c.w && c.sw, c.s, c.s && c.e && c.se],
                    ];
                    let inside_shape = [
                        inside_slice,
                        inside_slice,
                        inside_slice,
                        inside_slice,
                        inside_slice,
                    ];
                    let inside_mat =
                        palette.get(&Material::Default(DefaultMaterials::Hidden), context);
                    wall_shape = box_from_fn(|x, y, z| {
                        if inside_shape[z][y][x] {
                            inside_mat
                        } else {
                            wall_shape[z][y][x]
                        }
                    });
                }

                let shape: Box3D<Option<u8>> = box_map(wall_shape, Some);
                return voxels_from_shape(shape, self.local_coords());
            }
            TiletypeShape::FORTIFICATION => {
                let conn = map.neighbouring_flat(coords, |o| {
                    o.block_tile.as_ref().is_some_and(|t| t.is_wall())
                });
                let slice_fortification = [
                    [true, conn.n, true],
                    [conn.w, false, conn.e],
                    [true, conn.s, true],
                ];
                [
                    slice_fortification,
                    slice_fortification,
                    slice_full(),
                    slice_full(),
                    slice_full(),
                ]
            }
            TiletypeShape::STAIR_UP => stairs(true, true, false, true, coords.z),
            TiletypeShape::STAIR_DOWN => stairs(false, false, true, false, coords.z),
            TiletypeShape::STAIR_UPDOWN => stairs(true, true, true, false, coords.z),
            TiletypeShape::RAMP => ramp_shape(map, coords),
            _ => box_empty(),
        };

        let mat1 = palette.get(&material, context);
        let textured_shape: Box3D<Option<u8>> = if rough {
            let mat2 = palette.get(&material_dark, context);
            box_from_shape_fn(shape, || *[mat1, mat2].choose(&mut rng).unwrap())
        } else {
            box_from_shape_fn(shape, || mat1)
        };
        voxels_from_shape(textured_shape, self.local_coords())
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
