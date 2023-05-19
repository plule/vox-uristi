mod collect;
use crate::{
    context::DFContext, direction::DirectionFlat, map::Map, palette::Material, prefabs,
    voxel::FromPrefab, Coords, WithCoords,
};
use dfhack_remote::{BuildingInstance, MatPair};
use easy_ext::ext;
use std::ops::{RangeInclusive, Sub};

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

    pub fn contains(&self, coords: Coords) -> bool {
        self.x.contains(&coords.x) && self.y.contains(&coords.y) && self.z.contains(&coords.z)
    }
}

impl WithCoords for BuildingInstance {
    fn coords(&self) -> Coords {
        Coords::new(self.pos_x_min(), self.pos_y_min(), self.pos_z_min())
    }
}

impl FromPrefab for BuildingInstance {
    fn build_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_> {
        Box::new(self.items.iter().cycle().filter_map(|item| {
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

    fn self_connectivity(
        &self,
        map: &Map,
        context: &DFContext,
    ) -> crate::direction::NeighbouringFlat<bool> {
        let def = context.building_definition(&self.building_type);
        let coords = self.coords();
        map.neighbouring_flat(coords, |_, buildings| {
            buildings
                .iter()
                .any(|building| def == context.building_definition(&building.building_type))
        })
    }
}

#[ext(BuildingInstanceExt)]
pub impl BuildingInstance {
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

    fn is_floor(&self, context: &DFContext) -> bool {
        if let Some(def) = context.building_definition(&self.building_type) {
            if let Some(prefab) = prefabs::MODELS.building(def.id()) {
                return prefab.is_floor;
            }
        }
        false
    }

    fn is_chair(&self, context: &DFContext) -> bool {
        if let Some(def) = context.building_definition(&self.building_type) {
            def.id() == "Chair"
        } else {
            false
        }
    }
}

impl Sub<Coords> for BoundingBox {
    type Output = BoundingBox;

    fn sub(self, rhs: Coords) -> Self::Output {
        Self::new(
            (self.x.start() - rhs.x)..=(self.x.end() - rhs.x),
            (self.y.start() - rhs.y)..=(self.y.end() - rhs.y),
            (self.z.start() - rhs.z)..=(self.z.end() - rhs.z),
        )
    }
}
