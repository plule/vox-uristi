mod bridge;
mod building_type;
mod collect;
mod furniture;
mod workshop;

pub use bridge::BuildingInstanceBridgeExt;
pub use building_type::*;
pub use furniture::BuildingInstanceFurnitureExt;
pub use workshop::BuildingInstanceWorkshopExt;

pub use self::building_type::BuildingType;
use crate::{palette::Material, Coords};
use dfhack_remote::BuildingInstance;
use extend::ext;
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct BoundingBox {
    pub x: RangeInclusive<i32>,
    pub y: RangeInclusive<i32>,
    pub z: RangeInclusive<i32>,
}

impl BoundingBox {
    pub fn new(x: RangeInclusive<i32>, y: RangeInclusive<i32>, z: RangeInclusive<i32>) -> Self {
        Self { x, y, z }
    }
}

#[ext]
impl<T: Ord> Vec<T> {
    fn sorted(mut self) -> Self {
        self.sort();
        self
    }
}

#[ext]
pub impl &BuildingInstance {
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
}
