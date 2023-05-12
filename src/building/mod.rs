mod bridge;
mod building_type;
mod collect;
mod workshop;

pub use self::building_type::BuildingType;
use crate::{palette::Material, Coords};
use dfhack_remote::BuildingInstance;
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

pub struct Building<'a>(pub &'a BuildingInstance);

impl Building<'_> {
    pub fn building_type(&self) -> BuildingType {
        BuildingType::from_df(self.0)
    }

    pub fn material(&self) -> Material {
        Material::Generic(self.0.material.get_or_default().to_owned())
    }

    pub fn origin(&self) -> Coords {
        Coords::new(self.0.pos_x_min(), self.0.pos_y_min(), self.0.pos_z_min())
    }

    pub fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(
            self.0.pos_x_min()..=self.0.pos_x_max(),
            self.0.pos_y_min()..=self.0.pos_y_max(),
            self.0.pos_z_min()..=self.0.pos_z_max(),
        )
    }
}
