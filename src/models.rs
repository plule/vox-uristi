use dot_vox::Model;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use std::collections::HashMap;

static BUILDING_BYTES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/models/buildings");

fn load_model(bytes: &[u8]) -> Model {
    dot_vox::load_bytes(bytes)
        .expect("Invalid .vox")
        .models
        .pop()
        .expect("No model in .vox")
}

pub fn load_buildings() -> HashMap<String, Model> {
    let mut ret = HashMap::new();
    for model in BUILDING_BYTES.find("**").unwrap() {
        if let Some(model) = model.as_file() {
            match model.path().extension().map(|ext| ext.to_str()) {
                Some(Some("vox")) => {
                    ret.insert(
                        model.path().to_string_lossy().replace(".vox", ""),
                        load_model(model.contents()),
                    );
                }
                _ => panic!("Unsupported file type"),
            }
        }
    }
    ret
}

lazy_static! {
    pub static ref BUILDINGS: HashMap<String, Model> = load_buildings();
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::Path};

    use dfhack_remote::{BlockList, BuildingList};
    use protobuf::Message;

    use crate::{building::BuildingInstanceExt, rfr::create_building_def_map};

    use super::*;

    #[test]
    fn has_models_that_can_be_loaded() {
        assert!(BUILDINGS.len() > 0)
    }

    #[test]
    fn size_check() {
        let mut models_to_check: HashSet<&str> = BUILDINGS.keys().map(|s| s.as_str()).collect();
        let building_defs = BuildingList::parse_from_bytes(
            &std::fs::read(Path::new("testdata/building_defs.dat")).unwrap(),
        )
        .unwrap();
        let block_list =
            BlockList::parse_from_bytes(&std::fs::read(Path::new("testdata/block_0.dat")).unwrap())
                .unwrap();
        let building_defs = create_building_def_map(building_defs);
        assert!(!block_list.map_blocks.is_empty());
        let mut total_buildings = 0;
        let mut total_buildings_with_model = 0;
        for block in block_list.map_blocks {
            for building in block.buildings {
                total_buildings += 1;
                let building_type = building.building_type.clone();
                let def = building_defs
                    .get(&(
                        building_type.building_type(),
                        building_type.building_subtype(),
                        building_type.building_custom(),
                    ))
                    .unwrap();
                if let Some(model) = BUILDINGS.get(def.id()) {
                    models_to_check.remove(def.id());
                    total_buildings_with_model += 1;
                    let (x, y) = building.dimension();
                    assert_eq!(x * 3, model.size.x as i32, "{}", def.id());
                    assert_eq!(y * 3, model.size.y as i32, "{}", def.id());
                    assert_eq!(5, model.size.z, "{}", def.id());
                }
            }
        }

        assert_eq!(0, models_to_check.len(), "{:#?}", models_to_check);

        assert!(total_buildings > 0);
        assert!(total_buildings_with_model > 0);
    }
}
