mod building_type;
mod collect;

pub use self::building_type::BuildingType;
use crate::{
    direction::DirectionFlat, map::Map, palette::Material, voxel::FromPrefab, Coords, WithCoords,
};
pub use building_type::*;
use dfhack_remote::{BuildingInstance, MatPair};
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

    pub fn origin(&self) -> Coords {
        Coords::new(*self.x.start(), *self.y.start(), *self.z.start())
    }
}

impl WithCoords for BuildingInstance {
    fn coords(&self) -> Coords {
        Coords::new(self.pos_x_min(), self.pos_y_min(), self.pos_z_min())
    }
}

impl FromPrefab for BuildingInstance {
    fn build_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_> {
        Box::new(self.items.iter().filter_map(|item| {
            if item.mode() == 2 {
                Some(item.item.material.get_or_default().to_owned())
            } else {
                None
            }
        }))
    }

    fn content_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_> {
        Box::new(self.items.iter().filter_map(|item| {
            if item.mode() != 2 {
                Some(item.item.material.get_or_default().to_owned())
            } else {
                None
            }
        }))
    }

    fn df_orientation(&self) -> Option<DirectionFlat> {
        self.direction
            .and_then(|dir| dir.enum_value().ok())
            .and_then(|dir| DirectionFlat::maybe_from_df(&dir))
    }

    fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(
            self.pos_x_min()..=self.pos_x_max(),
            self.pos_y_min()..=self.pos_y_max(),
            self.pos_z_min()..=self.pos_z_max(),
        )
    }

    fn self_connectivity(&self, map: &Map) -> crate::direction::NeighbouringFlat<bool> {
        map.neighbouring_flat(self.coords(), |_, buildings| {
            buildings
                .iter()
                .any(|building| self.building_type() == building.building_type())
        })
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
                | BuildingType::ScrewPump
                | BuildingType::Windmill
        )
    }

    fn is_chair(&self) -> bool {
        matches!(BuildingType::from_df(self), BuildingType::Chair)
    }
}
