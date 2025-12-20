use dfhack_remote::TiletypeShape;
use easy_ext::ext;

use crate::{
    direction::NeighbouringFlat,
    export::{tile::ramp_levels, DFContext, DefaultMaterials, Map, Material, Palette},
    rfr::BlockTile,
    shape::{box_from_levels_with_content, Box3D, Slice2D},
    voxel::voxels_from_shape,
    WithDFCoords, BASE, HEIGHT,
};

#[ext(BlockTileTrackExt)]
pub impl BlockTile<'_> {
    fn build_track(
        &self,
        map: &Map,
        context: &DFContext,
        palette: &mut Palette,
    ) -> Vec<dot_vox::Voxel> {
        // Build the material. We don't have the material below the track, use generic rock
        let track_material = palette.get(&Material::Generic(self.material().clone()), context);
        let ground_material = palette.get(&Material::Default(DefaultMaterials::Rock), context);

        // connectivity where the rails are
        let d = NeighbouringFlat::<bool>::from_direction(self.tile_type().direction());
        #[rustfmt::skip]
        let rails: Slice2D<bool> =
            [
                [true, !d.n, true],
                [!d.w, false, !d.e],
                [true, !d.s, true]
            ];
        let material_slice: Slice2D<u8> =
            rails.map(|c| c.map(|b| if b { track_material } else { ground_material }));

        // base elevation, ramp or flat
        let mut level_slice: Slice2D<usize> = match self.tile_type().shape() {
            TiletypeShape::RAMP => ramp_levels(map, self.coords()),
            _ => [[1, 1, 1], [1, 1, 1], [1, 1, 1]],
        };

        // add one level on tracks (clamped for ramps)
        for x in 0..BASE {
            for y in 0..BASE {
                if rails[x][y] {
                    level_slice[x][y] = (level_slice[x][y] + 1).clamp(0, HEIGHT);
                }
            }
        }

        let shape: Box3D<Option<u8>> = box_from_levels_with_content(level_slice, material_slice);
        voxels_from_shape(shape, self.local_coords())
    }
}
