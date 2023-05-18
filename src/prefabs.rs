use anyhow::Context;
use dot_vox::Model;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::collections::HashMap;

static META_BYTES: &[u8] = include_bytes!("prefabs/meta.yaml");
static BUILDING_BYTES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/prefabs/buildings");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrefabsConfig {
    pub buildings: HashMap<String, PrefabConfig>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PrefabConfig {
    #[serde(default)]
    pub model: String,

    #[serde(default)]
    pub orientation_mode: OrientationMode,
}

#[derive(Default)]
pub struct Prefabs {
    buildings: HashMap<String, Prefab>,
}

impl Prefabs {
    pub fn building<'a>(&'a self, id: &str) -> Option<&'a Prefab> {
        self.buildings.get(&id.to_string())
    }
}

pub struct Prefab {
    pub model: Model,
    pub orientation_mode: OrientationMode,
}

#[derive(Deserialize, Default)]
pub enum OrientationMode {
    #[default]
    FromDwarfFortress,
    AgainstWall,
    FacingChairOrAgainstWall,
}

fn load_model(bytes: &[u8]) -> Model {
    dot_vox::load_bytes(bytes)
        .expect("Invalid .vox")
        .models
        .pop()
        .expect("No model in .vox")
}

pub fn load_models() -> Prefabs {
    let mut prefab_configs: PrefabsConfig = serde_yaml::from_slice(META_BYTES).unwrap();

    for model in BUILDING_BYTES.find("**").unwrap() {
        if let Some(model) = model.as_file() {
            match model.path().extension().and_then(|ext| ext.to_str()) {
                Some("vox") => {
                    let path = model.path().to_string_lossy();
                    let prefab = prefab_configs
                        .buildings
                        .entry(path.replace(".vox", "").to_string())
                        .or_insert_with(PrefabConfig::default);
                    if prefab.model.is_empty() {
                        prefab.model = path.to_string();
                    }
                }
                _ => panic!("Unsupported file type"),
            }
        }
    }

    let mut prefabs = Prefabs::default();
    for (id, cfg) in prefab_configs.buildings.into_iter() {
        prefabs.buildings.insert(
            id.clone(),
            Prefab {
                model: load_model(
                    BUILDING_BYTES
                        .get_file(&cfg.model)
                        .with_context(|| format!("Missing file: {} for model {}", &cfg.model, &id))
                        .unwrap()
                        .contents(),
                ),
                orientation_mode: cfg.orientation_mode,
            },
        );
    }
    prefabs
}

lazy_static! {
    pub static ref MODELS: Prefabs = load_models();
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
                if let Some(cfg) = MODELS.buildings.get(def.id()) {
                    models_to_check.remove(def.id());
                    total_buildings_with_model += 1;
                    let (x, y) = building.dimension();
                    assert_eq!(
                        0,
                        (x * 3) % cfg.model.size.x as i32,
                        "{}. building dimension: {}, model size: {}",
                        def.id(),
                        x,
                        cfg.model.size.x
                    );
                    assert_eq!(
                        0,
                        (y * 3) % cfg.model.size.y as i32,
                        "{}. building dimension: {}, model size: {}",
                        def.id(),
                        y,
                        cfg.model.size.y
                    );
                    assert_eq!(5, cfg.model.size.z, "{}", def.id());
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
