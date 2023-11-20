use anyhow::Context;
use dot_vox::Model;
use glob_match::glob_match;
use include_dir::{include_dir, Dir};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::collections::HashMap;

static META_BYTES: &[u8] = include_bytes!("../assets/prefabs.yaml");
static BUILDING_BYTES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/buildings");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrefabsConfig {
    pub buildings: HashMap<String, PrefabConfig>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct PrefabConfig {
    pub model: Option<String>,
    pub orientation: Option<OrientationMode>,
    pub content: Option<ContentMode>,
    pub connectivity: Option<Connectivity>,
    pub is_floor: Option<bool>,
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
    pub orientation: OrientationMode,
    pub content: ContentMode,
    pub connectivity: Connectivity,
    pub is_floor: bool,
}

#[derive(Deserialize, Default, Clone, Copy)]
pub enum OrientationMode {
    #[default]
    FromDwarfFortress,
    AgainstWall,
    FacingChairOrAgainstWall,
}

#[derive(Deserialize, Default, Clone, Copy)]
pub enum ContentMode {
    #[default]
    Unique,
    All,
}

#[derive(Deserialize, Default, Clone, Copy)]
pub enum Connectivity {
    #[default]
    None,
    SelfOrWall,
    SelfRemovesLayer(u8),
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
                        .or_default();
                    if prefab.model.is_none() {
                        prefab.model = Some(path.to_string());
                    }
                }
                _ => panic!("Unsupported file type"),
            }
        }
    }

    // separate the glob patterns from the static patterns
    let mut globs = HashMap::new();
    let mut statics = HashMap::new();
    for (id, cfg) in prefab_configs.buildings.into_iter() {
        if id.contains('*') {
            globs.insert(id, cfg);
        } else {
            statics.insert(id, cfg);
        }
    }

    // create the concrete configuration
    let mut prefabs = Prefabs::default();
    for (id, mut cfg) in statics.into_iter() {
        for (glob, glob_cfg) in globs.iter() {
            if glob_match(glob, &id) {
                cfg.model = cfg.model.or(glob_cfg.model.clone());
                cfg.orientation = cfg.orientation.or(glob_cfg.orientation);
                cfg.connectivity = cfg.connectivity.or(glob_cfg.connectivity);
                cfg.content = cfg.content.or(glob_cfg.content);
                cfg.is_floor = cfg.is_floor.or(glob_cfg.is_floor);
            }
        }

        let model_path = cfg
            .model
            .with_context(|| format!("No model for building {}", &id))
            .unwrap();

        prefabs.buildings.insert(
            id.clone(),
            Prefab {
                model: load_model(
                    BUILDING_BYTES
                        .get_file(&model_path)
                        .with_context(|| {
                            format!("Missing file: {} for building {}", &model_path, &id)
                        })
                        .unwrap()
                        .contents(),
                ),
                orientation: cfg.orientation.unwrap_or_default(),
                content: cfg.content.unwrap_or_default(),
                connectivity: cfg.connectivity.unwrap_or_default(),
                is_floor: cfg.is_floor.unwrap_or(false),
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

    use crate::{
        building::BuildingInstanceExt,
        direction::{DirectionFlat, Rotating},
        rfr::create_building_def_map,
        voxel::FromPrefab,
        BASE, HEIGHT,
    };

    use super::*;

    #[test]
    fn has_models_that_can_be_loaded() {
        assert!(!MODELS.buildings.is_empty())
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
                if let Some(prefab) = MODELS.buildings.get(def.id()) {
                    let model = Model {
                        size: prefab.model.size,
                        voxels: prefab.model.voxels.clone(),
                    };
                    let model =
                        model.looking_at(building.df_orientation().unwrap_or(DirectionFlat::South));
                    models_to_check.remove(def.id());
                    total_buildings_with_model += 1;
                    let (x, y) = building.dimension();
                    assert_eq!(
                        0,
                        (x * BASE as i32) % model.size.x as i32,
                        "{}. building dimension: {}, model size: {}",
                        def.id(),
                        x,
                        model.size.x
                    );
                    assert_eq!(
                        0,
                        (y * BASE as i32) % model.size.y as i32,
                        "{}. building dimension: {}, model size: {}",
                        def.id(),
                        y,
                        model.size.y
                    );
                    assert_eq!(0, model.size.z % HEIGHT as u32, "{}", def.id());
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
