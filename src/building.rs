use crate::{
    context::DFContext, coords::WithBoundingBox, direction::DirectionFlat,
    dot_vox_builder::DotVoxBuilder, export::BUILDING_LAYER, map::Map, prefabs::FromPrefab,
    DFBoundingBox, DFMapCoords, WithDFCoords,
};
use dfhack_remote::{BuildingInstance, MatPair};
use easy_ext::ext;

impl WithDFCoords for BuildingInstance {
    fn coords(&self) -> DFMapCoords {
        DFMapCoords::new(self.pos_x_min(), self.pos_y_min(), self.pos_z_min())
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
        map.neighbouring_flat(coords, |o| {
            o.buildings
                .iter()
                .any(|b| def == context.building_definition(&b.building_type))
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
    fn build(
        &self,
        map: &Map,
        context: &DFContext,
        vox: &mut DotVoxBuilder,
        palette: &mut crate::palette::Palette,
        group: usize,
    ) {
        if let Some((name, model)) = self.do_build(map, context, palette) {
            let coords = self
                .bounding_box()
                .layer_dot_vox_coords()
                .into_layer_global_coords(context.max_vox_x(), context.max_vox_y());

            vox.insert_model_shape(group, Some(coords), model, BUILDING_LAYER, name);
        }
    }
    fn do_build(
        &self,
        map: &crate::map::Map,
        context: &DFContext,
        palette: &mut crate::palette::Palette,
    ) -> Option<(String, dot_vox::Model)> {
        let building_definition =
            context.building_definition(self.building_type.get_or_default())?;

        let name = building_definition.name();
        let prefab = crate::prefabs::MODELS.building(building_definition.id())?;
        let model = prefab.build(self, map, context, palette);
        Some((name.to_string(), model))
    }

    fn is_chair(&self, context: &DFContext) -> bool {
        if let Some(def) = context.building_definition(&self.building_type) {
            def.id() == "Chair"
        } else {
            false
        }
    }
}
