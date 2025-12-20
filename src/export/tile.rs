//! Tile export functions (smallest dwarf fortress map unit)
mod generic;
mod track;
mod tree;

use std::collections::HashSet;

use super::{DFContext, DefaultMaterials, Layers, Map, Material, Palette};
use crate::{
    direction::Neighbouring8Flat,
    export::{block::BlockModels, tile::track::BlockTileTrackExt},
    rfr::BlockTile,
    shape::{box_from_fn, box_from_levels, box_full, slice_const, Box3D},
    voxel::voxels_from_uniform_shape,
    DFMapCoords, GenBoolSafe, StableRng, WithDFCoords,
};
use dfhack_remote::{MatterState, TiletypeMaterial, TiletypeShape, TiletypeSpecial};
pub use generic::BlockTileExt;
use rand::Rng;
pub use tree::BlockTilePlantExt;

pub fn ramp_levels(map: &Map<'_>, coords: DFMapCoords) -> [[usize; 3]; 3] {
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

    [[c.nw, c.n, c.ne], [c.w, max / 2, c.e], [c.sw, c.s, c.se]]
}

impl WithDFCoords for BlockTile<'_> {
    fn coords(&self) -> crate::DFMapCoords {
        self.global_coords()
    }
}

impl BlockTile<'_> {
    pub fn build(
        &self,
        models: &mut BlockModels,
        map: &Map,
        context: &DFContext,
        palette: &mut Palette,
    ) {
        let mut rng = self.stable_rng();

        // Voxels that spatters can sit on top
        let mut occupied_for_spatters: HashSet<(u8, u8, u8)> = HashSet::new();

        if self.hidden() {
            let shape: Box3D<bool> = box_full();

            models.extend(
                Layers::Hidden,
                voxels_from_uniform_shape(
                    shape,
                    self.local_coords(),
                    palette.get(&Material::Default(DefaultMaterials::Hidden), context),
                ),
            );
            return;
        }

        match (
            self.tile_type().material(),
            self.tile_type().shape(),
            self.tile_type().special(),
        ) {
            (
                TiletypeMaterial::ROOT
                | TiletypeMaterial::MUSHROOM
                | TiletypeMaterial::PLANT
                | TiletypeMaterial::TREE_MATERIAL,
                _,
                _,
            )
            | (
                _,
                TiletypeShape::SAPLING
                | TiletypeShape::TWIG
                | TiletypeShape::SHRUB
                | TiletypeShape::BRANCH,
                _,
            ) => {
                // plant, trees
                let trees = self.build_trees(map, context, palette);
                occupied_for_spatters.extend(trees.iter().map(|v| (v.x, v.y, v.z)));
                models.extend(Layers::Vegetation, trees);
            }
            (_, _, TiletypeSpecial::TRACK) => {
                // tracks or frozen tracks
                let track = self.build_track(map, context, palette);
                models.extend(Layers::Terrain, track);
            }
            _ => {
                // classic tile structure
                let (terrain, roughness) = self.build_terrain(map, context, palette);
                occupied_for_spatters.extend(terrain.iter().map(|v| (v.x, v.y, v.z)));
                models.extend(Layers::Terrain, terrain);
                models.extend(Layers::Roughness, roughness);
            }
        }

        // liquids
        if self.water() > 0 {
            let water_shape: Box3D<bool> =
                box_from_levels(slice_const(self.water().clamp(2, 7) as usize));
            models.extend(
                Layers::Liquid,
                voxels_from_uniform_shape(
                    water_shape,
                    self.local_coords(),
                    palette.get(&Material::Default(DefaultMaterials::Water), context),
                ),
            );
        }

        if self.magma() > 0 {
            let magma_shape: Box3D<bool> =
                box_from_levels(slice_const(self.magma().clamp(2, 7) as usize));
            models.extend(
                Layers::Liquid,
                voxels_from_uniform_shape(
                    magma_shape,
                    self.local_coords(),
                    palette.get(&Material::Default(DefaultMaterials::Magma), context),
                ),
            );
        }

        // spatters
        for spatter in self.spatters() {
            // spatters sit on top of existing voxels, when there is some space
            let material = Material::Generic(spatter.material.get_or_default().clone());

            for (x, y, z) in &occupied_for_spatters {
                let coords = (*x, *y, *z + 1);
                if !occupied_for_spatters.contains(&coords) {
                    let gen = match spatter.state() {
                        // solid spatter is stuff like fruits and leaves, from zero to 10 000.
                        // there are a lot of them, so step down the probability
                        MatterState::Solid => rng.gen_bool_safe(spatter.amount() as f64 / 50_000.0),
                        // liquid spatter is blood etc, from 0 to 255.
                        // completely covered is a bit weird, half the probability
                        MatterState::Liquid => rng.gen_bool_safe(spatter.amount() as f64 / 512.0),
                        // powder spatter is likely snow, going from 0 to 100. We want 100% snow to covere the ground
                        MatterState::Powder => rng.gen_bool_safe(spatter.amount() as f64 / 100.0),
                        // gas, paste and other, I don't know how the can occur
                        _ => false,
                    };
                    if gen {
                        models.extend(
                            Layers::Spatter,
                            Some(dot_vox::Voxel {
                                x: coords.0,
                                y: coords.1,
                                z: coords.2,
                                i: palette.get(&material, context),
                            }),
                        );
                    }
                }
            }
        }

        // Fire is identified as a special tiletype material
        if self.tile_type().material() == TiletypeMaterial::FIRE {
            let shape: Box3D<bool> = box_from_fn(|_, _, _| rng.random_bool(0.1));
            let material = palette.get(&Material::Default(DefaultMaterials::Fire), context);
            models.extend(
                Layers::Fire,
                voxels_from_uniform_shape(shape, self.local_coords(), material),
            );
        }
    }
}
