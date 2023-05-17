mod bridge;
mod building_type;
mod collect;
mod furniture;

pub use bridge::BuildingInstanceBridgeExt;
pub use building_type::*;
pub use furniture::BuildingInstanceFurnitureExt;

pub use self::building_type::BuildingType;
use crate::{direction::DirectionFlat, palette::Material, voxel::FromDotVox2, Coords, WithCoords};
use dfhack_remote::BuildingInstance;
use easy_ext::ext;
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

impl WithCoords for BuildingInstance {
    fn coords(&self) -> Coords {
        Coords::new(self.pos_x_min(), self.pos_y_min(), self.pos_z_min())
    }
}

impl FromDotVox2 for BuildingInstance {
    fn build_materials(&self) -> [Option<dfhack_remote::MatPair>; 8] {
        let mut iter = self.items.iter().filter_map(|item| {
            if item.mode() == 2 {
                Some(item.item.material.get_or_default().to_owned())
            } else {
                None
            }
        });
        std::array::from_fn(|_| iter.next())
    }

    fn content_materials(&self) -> [Option<dfhack_remote::MatPair>; 8] {
        let mut iter = self.items.iter().filter_map(|item| {
            if item.mode() != 2 {
                Some(item.item.material.get_or_default().to_owned())
            } else {
                None
            }
        });
        std::array::from_fn(|_| iter.next())
    }

    fn df_orientation(&self) -> Option<DirectionFlat> {
        self.direction
            .and_then(|dir| dir.enum_value().ok())
            .and_then(|dir| DirectionFlat::maybe_from_df(&dir))
    }
}

#[ext(BuildingInstanceExt)]
pub impl BuildingInstance {
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

    fn dimension(&self) -> (i32, i32) {
        let bounding_box = self.bounding_box();
        (
            1 + bounding_box.x.end() - bounding_box.x.start(),
            1 + bounding_box.y.end() - bounding_box.y.start(),
        )
    }

    fn is_floor(&self) -> bool {
        matches!(
            BuildingType::from_df(self),
            BuildingType::TradeDepot
                | BuildingType::Furnace(_)
                | BuildingType::Statue
                | BuildingType::Workshop(_)
        )
    }
}
