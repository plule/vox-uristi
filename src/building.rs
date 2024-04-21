use crate::{
    context::DFContext,
    coords::WithBoundingBox,
    direction::DirectionFlat,
    map::Map,
    palette::Palette,
    prefabs::{self, FromPrefab},
    voxel::CollectObjectVoxels,
    DFBoundingBox, DFCoords, WithDFCoords,
};
use dfhack_remote::{BuildingInstance, MatPair};
use easy_ext::ext;

impl CollectObjectVoxels for BuildingInstance {
    fn build(
        &self,
        map: &Map,
        context: &DFContext,
        palette: &mut Palette,
    ) -> Option<dot_vox::Model> {
        let building_definition =
            context.building_definition(self.building_type.get_or_default())?;
        let prefab = crate::prefabs::MODELS.building(building_definition.id())?;
        Some(self.apply_prefab(prefab, map, context, palette))
    }
}

impl WithDFCoords for BuildingInstance {
    fn coords(&self) -> DFCoords {
        DFCoords::new(self.pos_x_min(), self.pos_y_min(), self.pos_z_min())
    }
}

impl FromPrefab for BuildingInstance {
    fn build_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_> {
        Box::new(
            self.items
                .iter()
                .filter_map(|item| {
                    if item.mode() == 2 {
                        Some(item.item.material.get_or_default().to_owned())
                    } else {
                        None
                    }
                })
                .cycle(),
        )
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

    fn self_connectivity(
        &self,
        map: &Map,
        context: &DFContext,
    ) -> crate::direction::NeighbouringFlat<bool> {
        let def = context.building_definition(&self.building_type);
        let coords = self.coords();
        map.neighbouring_flat(coords, |tile| {
            tile.buildings
                .iter()
                .any(|building| def == context.building_definition(&building.building_type))
        })
    }
}

impl WithBoundingBox for BuildingInstance {
    fn bounding_box(&self) -> DFBoundingBox {
        DFBoundingBox::new(
            self.pos_x_min()..=self.pos_x_max(),
            self.pos_y_min()..=self.pos_y_max(),
            self.pos_z_min()..=self.pos_z_max(),
        )
    }
}

#[ext(BuildingInstanceExt)]
pub impl BuildingInstance {
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
