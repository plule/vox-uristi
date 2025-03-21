use std::collections::HashMap;

use anyhow::Result;
use dfhack_remote::{
    BasicMaterialInfo, BasicMaterialInfoMask, BuildingDefinition, BuildingType, ListEnumsOut,
    ListMaterialsIn, MapInfo, MaterialList, PlantRawList, TiletypeList,
};
use protobuf::MessageField;

use crate::{block::BLOCK_SIZE, export::ExportSettings, rfr::create_building_def_map, BASE};

pub struct DFContext {
    pub settings: ExportSettings,
    pub tile_types: TiletypeList,
    pub materials: MaterialList,
    pub map_info: MapInfo,
    pub plant_raws: PlantRawList,
    pub enums: ListEnumsOut,
    pub building_map: HashMap<(i32, i32, i32), BuildingDefinition>,
    pub inorganic_materials_map: HashMap<(i32, i32), BasicMaterialInfo>,
}

impl DFContext {
    pub fn try_new(client: &mut dfhack_remote::Client, settings: ExportSettings) -> Result<Self> {
        let inorganics_materials = client.core().list_materials(ListMaterialsIn {
            mask: MessageField::some(BasicMaterialInfoMask {
                flags: Some(true),
                reaction: Some(true),
                ..Default::default()
            }),
            inorganic: Some(true),
            builtin: Some(true),
            ..Default::default()
        })?;
        let inorganic_materials_map: HashMap<(i32, i32), BasicMaterialInfo> = inorganics_materials
            .reply
            .value
            .into_iter()
            .map(|mat| ((mat.type_(), mat.index()), mat))
            .collect();
        Ok(Self {
            settings,
            tile_types: client.remote_fortress_reader().get_tiletype_list()?.reply,
            materials: client.remote_fortress_reader().get_material_list()?.reply,
            map_info: client.remote_fortress_reader().get_map_info()?.reply,
            plant_raws: client.remote_fortress_reader().get_plant_raws()?.reply,
            enums: client.core().list_enums()?.reply,
            building_map: create_building_def_map(
                client
                    .remote_fortress_reader()
                    .get_building_def_list()?
                    .reply,
            ),
            inorganic_materials_map,
        })
    }

    pub fn building_definition<'a>(
        &'a self,
        building_type: &BuildingType,
    ) -> Option<&'a BuildingDefinition> {
        self.building_map.get(&(
            building_type.building_type(),
            building_type.building_subtype(),
            building_type.building_custom(),
        ))
    }

    pub fn max_vox_x(&self) -> i32 {
        (self.map_info.block_size_x() * (BLOCK_SIZE * BASE) as i32) / 2
    }

    pub fn max_vox_y(&self) -> i32 {
        (self.map_info.block_size_y() * (BLOCK_SIZE * BASE) as i32) / 2
    }
}
