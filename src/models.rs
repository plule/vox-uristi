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
    for model in BUILDING_BYTES.entries() {
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
    use super::*;

    #[test]
    fn has_models_that_can_be_loaded() {
        assert!(load_buildings().len() > 0)
    }
}
