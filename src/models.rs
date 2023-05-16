use anyhow::Context;
use dot_vox::Model;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::collections::HashMap;

static META_BYTES: &[u8] = include_bytes!("../models/meta.yaml");
static BUILDING_BYTES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/models/buildings");

#[derive(Deserialize)]
pub struct ModelMeta {
    pub buildings: HashMap<String, String>,
}

#[derive(Default)]
pub struct Models {
    buildings: HashMap<String, Model>,
}

impl Models {
    pub fn building<'a>(&'a self, id: &str) -> Option<&'a Model> {
        self.buildings.get(&id.to_string())
    }
}

fn load_model(bytes: &[u8]) -> Model {
    dot_vox::load_bytes(bytes)
        .expect("Invalid .vox")
        .models
        .pop()
        .expect("No model in .vox")
}

pub fn load_models() -> Models {
    let mut meta: ModelMeta = serde_yaml::from_slice(META_BYTES).unwrap();

    for model in BUILDING_BYTES.find("**").unwrap() {
        if let Some(model) = model.as_file() {
            match model.path().extension().and_then(|ext| ext.to_str()) {
                Some("vox") => {
                    let path = model.path().to_string_lossy();
                    meta.buildings
                        .insert(path.replace(".vox", "").to_string(), path.to_string());
                }
                _ => panic!("Unsupported file type"),
            }
        }
    }

    let mut models = Models::default();
    for (id, path) in meta.buildings.into_iter() {
        models.buildings.insert(
            id.clone(),
            load_model(
                BUILDING_BYTES
                    .get_file(&path)
                    .with_context(|| format!("Missing file: {} for model {}", &path, &id))
                    .unwrap()
                    .contents(),
            ),
        );
    }
    models
}

lazy_static! {
    pub static ref MODELS: Models = load_models();
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
        assert!(MODELS.buildings.len() > 0)
    }

    #[test]
    fn check_models() {
        let mut models_to_check: HashSet<&str> =
            MODELS.buildings.keys().map(|s| s.as_str()).collect();
        let mut missing_models = Vec::new();
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
                if let Some(model) = MODELS.buildings.get(def.id()) {
                    models_to_check.remove(def.id());
                    total_buildings_with_model += 1;
                    let (x, y) = building.dimension();
                    assert_eq!(
                        0,
                        (x * 3) % model.size.x as i32,
                        "{}. building dimension: {}, model size: {}",
                        def.id(),
                        x,
                        model.size.x
                    );
                    assert_eq!(
                        0,
                        (y * 3) % model.size.y as i32,
                        "{}. building dimension: {}, model size: {}",
                        def.id(),
                        y,
                        model.size.y
                    );
                    assert_eq!(5, model.size.z, "{}", def.id());
                } else {
                    missing_models.push(def.id());
                }
            }
        }

        assert_eq!(0, models_to_check.len(), "{:#?}", models_to_check);

        assert!(total_buildings > 0);
        assert!(total_buildings_with_model > 0);

        //assert_eq!(0, missing_models.len(), "{:#?}", missing_models);
    }
}
