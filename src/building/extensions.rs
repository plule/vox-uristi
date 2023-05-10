use super::BoundingBox;
use crate::{
    building_type::BuildingType,
    direction::DirectionFlat,
    map::Coords,
    palette::Material,
    shape,
    voxel::{voxels_from_uniform_shape, Voxel},
};

pub trait BuildingExtensions {
    fn building_type(&self) -> BuildingType;
    fn material(&self) -> Material;
    fn origin(&self) -> Coords;
    fn bounding_box(&self) -> BoundingBox;
    fn bridge_collect_voxels(&self, direction: Option<DirectionFlat>) -> Vec<Voxel>;
}

impl BuildingExtensions for dfhack_remote::BuildingInstance {
    fn building_type(&self) -> BuildingType {
        BuildingType::from_df(self)
    }

    fn material(&self) -> Material {
        Material::Generic(self.material.get_or_default().to_owned())
    }

    fn origin(&self) -> Coords {
        Coords::new(self.pos_x_min(), self.pos_y_min(), self.pos_z_min())
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(
            self.pos_x_min()..=self.pos_x_max(),
            self.pos_y_min()..=self.pos_y_max(),
            self.pos_z_min()..=self.pos_z_max(),
        )
    }

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
