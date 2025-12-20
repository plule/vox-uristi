//! Template .vox file management

use anyhow::Context;
use dfhack_remote::MatPair;
use dot_vox::{Model, Voxel};
use glob_match::glob_match;
use include_dir::{include_dir, Dir};
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{collections::HashMap, iter::repeat};

use crate::{
    coords::WithBoundingBox,
    direction::{DirectionFlat, NeighbouringFlat, Rotating},
    export::building::BuildingInstanceExt,
    export::context::DFContext,
    export::tile::BlockTileExt,
    BASE,
};

use super::{DefaultMaterials, Map, Material, Palette};

static META_BYTES: &[u8] = include_bytes!("../../assets/prefabs.yaml");
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
}

#[derive(Default)]
pub struct Prefabs {
    buildings: HashMap<String, Prefab>,
}

impl Prefabs {
    pub fn building<'a>(&'a self, id: &str) -> Option<&'a Prefab> {
        self.buildings.get(id)
    }
}

#[derive(Debug)]
pub struct Prefab {
    pub model: Model,
    pub orientation: OrientationMode,
    pub content: ContentMode,
    pub connectivity: Connectivity,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub enum OrientationMode {
    #[default]
    FromDwarfFortress,
    AgainstWall,
    FacingChairOrAgainstWall,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub enum ContentMode {
    #[default]
    Unique,
    All,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
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
            },
        );
    }
    prefabs
}

lazy_static! {
    pub static ref MODELS: Prefabs = load_models();
}

pub trait FromPrefab: WithBoundingBox {
    fn build_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_>;
    fn content_materials(&self) -> Box<dyn Iterator<Item = MatPair> + '_>;
    fn df_orientation(&self) -> Option<DirectionFlat>;
    fn self_connectivity(&self, map: &Map, context: &DFContext) -> NeighbouringFlat<bool>;
}

impl Prefab {
    pub fn build(
        &self,
        obj: &impl FromPrefab,
        map: &Map,
        context: &DFContext,
        palette: &mut Palette,
    ) -> Model {
        let mut model = Model {
            size: self.model.size,
            voxels: self.model.voxels.clone(),
        };

        let bounding_box = obj.bounding_box();
        let coords = bounding_box.origin();

        // Rotate the model based on the preference
        match self.orientation {
            OrientationMode::FromDwarfFortress => {
                if let Some(direction) = obj.df_orientation() {
                    model = model.looking_at(direction);
                }
            }
            OrientationMode::AgainstWall => {
                model = model.facing_away(map.wall_direction(coords));
            }
            OrientationMode::FacingChairOrAgainstWall => {
                let c = map
                    .neighbouring_flat(coords, |o| o.buildings.iter().any(|b| b.is_chair(context)));
                if let Some(chair_direction) = c.directions().first() {
                    model = model.looking_at(*chair_direction)
                } else {
                    model = model.facing_away(map.wall_direction(coords));
                }
            }
        }

        // Collect the material palette
        // First 8 materials of the palette are the build materials
        let build_materials = obj
            .build_materials()
            .map(|m| Some(Material::Generic(m)))
            .chain(repeat(None))
            .take(8);
        // Next 8 materials are the darker versions
        let dark_build_materials = obj
            .build_materials()
            .map(|m| Some(Material::DarkGeneric(m)))
            .chain(repeat(None))
            .take(8);
        // Next 8 are the content materials
        let content_materials = match self.content {
            ContentMode::Unique => obj
                .content_materials()
                .unique_by(|m| (m.mat_index(), m.mat_type()))
                .take(8)
                .collect_vec(),
            ContentMode::All => obj.content_materials().take(8).collect_vec(),
        }
        .into_iter()
        .map(|m| Some(Material::Generic(m)))
        .chain(repeat(None))
        .take(8);
        // Next are the default hard-coded materials
        let default_materials = [
            Some(Material::Default(DefaultMaterials::Fire)),
            Some(Material::Default(DefaultMaterials::Wood)),
            Some(Material::Default(DefaultMaterials::Light)),
        ];

        let materials: Vec<Option<Material>> = build_materials
            .chain(dark_build_materials)
            .chain(content_materials)
            .chain(default_materials)
            .collect();

        // Translate the material indexes, filter out the voxels without material
        model.voxels.retain_mut(|voxel| {
            let material = materials.get(voxel.i as usize).cloned().flatten();
            if let Some(material) = material {
                voxel.i = palette.get(&material, context);
                true
            } else {
                false
            }
        });

        // store the rotated prefab voxel by df coordinates (3x3xinf)
        let prefab_size = model.size;
        let (prefab_sx, prefab_sy) = (prefab_size.x as usize / BASE, prefab_size.y as usize / BASE);
        let mut prefab_voxel_tiles: Vec<Vec<Vec<Voxel>>> =
            vec![vec![Vec::new(); prefab_sy]; prefab_sx];
        for voxel in model.voxels.iter() {
            let x = voxel.x as usize / BASE;
            let y = voxel.y as usize / BASE;
            if let Some(voxels) = prefab_voxel_tiles.get_mut(x).and_then(|v| v.get_mut(y)) {
                voxels.push(Voxel {
                    x: voxel.x % BASE as u8,
                    y: voxel.y % BASE as u8,
                    ..*voxel
                });
            }
        }

        // Fill the voxels from the prefab voxel, repeating the
        // center tiles
        let dimension = bounding_box.dimension();
        let mut voxels = Vec::new();
        for x in 0..dimension.x {
            for y in 0..dimension.y {
                let x_tile = if prefab_sx >= 3 {
                    match x {
                        0 => 0,
                        x if x == dimension.x - 1 => prefab_sx - 1,
                        _ => (x as usize - 1) % (prefab_sx - 2) + 1,
                    }
                } else {
                    x as usize % prefab_sx
                };
                let y_tile = if prefab_sy >= 3 {
                    match y {
                        0 => 0,
                        y if y == dimension.y - 1 => prefab_sy - 1,
                        _ => (y as usize - 1) % (prefab_sy - 2) + 1,
                    }
                } else {
                    y as usize % prefab_sy
                };
                if let Some(prefab_voxel_tile) =
                    prefab_voxel_tiles.get(x_tile).and_then(|v| v.get(y_tile))
                {
                    for voxel in prefab_voxel_tile.iter() {
                        voxels.push(Voxel {
                            x: (x as u8 * BASE as u8 + voxel.x),
                            y: (y as u8 * BASE as u8 + voxel.y),
                            z: voxel.z,
                            i: voxel.i,
                        });
                    }
                }
            }
        }

        model.size = dot_vox::Size::from(dimension);
        model.voxels = voxels;

        // Apply connectivity rules
        match self.connectivity {
            Connectivity::None => {}
            Connectivity::SelfOrWall => {
                let wall_connectivity = map.neighbouring_flat(coords, |o| {
                    o.block_tile.as_ref().is_some_and(|t| t.is_wall())
                });
                let neighbour_connectivity = obj.self_connectivity(map, context);
                let c = wall_connectivity | neighbour_connectivity;
                let cx = (model.size.x / 2) as i32;
                let cy = (model.size.y / 2) as i32;
                model.voxels.retain(|voxel| {
                    let mut display = true;
                    let x = voxel.x as i32 - cx;
                    let y = voxel.y as i32 - cy;
                    if x < 0 {
                        display &= c.w;
                    }
                    if x > 0 {
                        display &= c.e;
                    }
                    if y < 0 {
                        display &= c.s;
                    }
                    if y > 0 {
                        display &= c.n;
                    }
                    display
                });
            }
            Connectivity::SelfRemovesLayer(layer) => {
                let neighbour_connectivity = obj.self_connectivity(map, context);
                let self_connectivity =
                    NeighbouringFlat::new(|dir| bounding_box.contains(coords + dir));
                let c = neighbour_connectivity | self_connectivity;
                let cx = (model.size.x / 2) as i32;
                let cy = (model.size.y / 2) as i32;
                model.voxels.retain(|voxel| {
                    let mut display = true;
                    let x = voxel.x as i32 - cx;
                    let y = voxel.y as i32 - cy;
                    let z = voxel.z;
                    if x < 0 && z == layer {
                        display &= !c.w;
                    }
                    if x > 0 && z == layer {
                        display &= !c.e;
                    }
                    if y < 0 && z == layer {
                        display &= !c.s;
                    }
                    if y > 0 && z == layer {
                        display &= !c.n;
                    }
                    display
                });
            }
        }

        model
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::Path};

    use dfhack_remote::{BlockList, BuildingList};
    use protobuf::Message;

    use crate::{
        coords::WithBoundingBox,
        direction::{DirectionFlat, Rotating},
        rfr::create_building_def_map,
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
                    let dimension = building.bounding_box().dimension();

                    if def.id() != "Bridge" {
                        // bridge is repeating
                        assert_eq!(
                            0,
                            (dimension.x * BASE as u32) % model.size.x,
                            "{}. building dimension: {}, model size: {}",
                            def.id(),
                            dimension.x,
                            model.size.x
                        );
                        assert_eq!(
                            0,
                            (dimension.y * BASE as u32) % model.size.y as u32,
                            "{}. building dimension: {}, model size: {}",
                            def.id(),
                            dimension.y,
                            model.size.y
                        );
                    }
                    assert_eq!(0, model.size.z % HEIGHT as u32, "{}", def.id());
                } else {
                    missing_models.push(def.id());
                }
            }
        }

        // todo
        let mut unchecked_models = HashSet::new();
        unchecked_models.insert("BarsFloor");
        unchecked_models.insert("SiegeEngine/BoltThrower");

        assert_eq!(unchecked_models, models_to_check);

        assert!(total_buildings > 0);
        assert!(total_buildings_with_model > 0);

        //assert_eq!(0, missing_models.len(), "{:#?}", missing_models);
    }
}
