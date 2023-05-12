use super::BuildingInstanceExt;
use crate::{
    direction::DirectionFlat,
    shape,
    voxel::{voxels_from_uniform_shape, Voxel},
    Coords,
};
use dfhack_remote::BuildingInstance;
use extend::ext;

#[ext(name=BuildingInstanceBridgeExt)]
pub impl BuildingInstance {
    fn bridge_collect_voxels(&self, direction: Option<DirectionFlat>) -> Vec<Voxel> {
        let mut voxels = Vec::new();
        let sn = matches!(direction, Some(DirectionFlat::North | DirectionFlat::South));
        let ew = matches!(direction, Some(DirectionFlat::East | DirectionFlat::West));
        let bounding_box = self.bounding_box();
        for x in bounding_box.x.clone() {
            for y in bounding_box.y.clone() {
                let w = sn && x == *bounding_box.x.start();
                let e = sn && x == *bounding_box.x.end();
                let n = ew && y == *bounding_box.y.start();
                let s = ew && y == *bounding_box.y.end();
                let shape = [
                    shape::slice_empty(),
                    shape::slice_empty(),
                    shape::slice_empty(),
                    [[w || n, n, e || n], [w, false, e], [w || s, s, e || s]],
                    shape::slice_full(),
                ];
                let mut shape_voxels = voxels_from_uniform_shape(
                    shape,
                    Coords::new(x, y, self.origin().z),
                    self.material(),
                );
                voxels.append(&mut shape_voxels);
            }
        }
        voxels
    }
}
