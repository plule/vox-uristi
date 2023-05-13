use ahash::{AHashMap, HashMap};
use dot_vox::{Dict, DotVoxData, Frame, Material, Model, SceneNode, ShapeModel, Size, Voxel};
use extend::ext;
use num_integer::div_mod_floor;

const MODEL_EDGE: i32 = 128;
pub struct DotVoxBuilder {
    data: DotVoxData,
    models: HashMap<(i32, i32, i32), usize>,
}

impl Default for DotVoxBuilder {
    fn default() -> Self {
        let root_scene_graph = vec![
            SceneNode::Transform {
                attributes: Default::default(),
                frames: vec![Frame {
                    attributes: Default::default(),
                }],
                child: 1,
                layer_id: 0,
            },
            SceneNode::Group {
                attributes: Default::default(),
                children: Default::default(),
            },
        ];
        Self {
            data: DotVoxData {
                version: 150,
                models: vec![],
                palette: dot_vox::DEFAULT_PALETTE.to_vec(),
                materials: (0..256)
                    .into_iter()
                    .map(|i| Material {
                        id: i,
                        properties: {
                            let mut map = Dict::new();
                            map.insert("_ior".to_owned(), "0.3".to_owned());
                            map.insert("_rough".to_owned(), "0.1".to_owned());
                            map.insert("_ior".to_owned(), "0.3".to_owned());
                            map.insert("_d".to_owned(), "0.05".to_owned());
                            map
                        },
                    })
                    .collect(),
                scenes: root_scene_graph,
                layers: vec![],
            },
            models: Default::default(),
        }
    }
}

impl DotVoxBuilder {
    pub fn get_or_insert_model(&mut self, (x, y, z): (i32, i32, i32)) -> &mut Model {
        let next_index = self.models.len();
        let index = self.models.entry((x, y, z)).or_insert_with(|| {
            // Create the model
            let model_index = next_index;
            self.data.models.push(Model {
                size: Size {
                    x: MODEL_EDGE as u32,
                    y: MODEL_EDGE as u32,
                    z: MODEL_EDGE as u32,
                },
                voxels: vec![],
            });

            // Insert the transform and shape nodes for this model in the scene graph
            let transform_node = self.data.scenes.len();
            let shape_node = transform_node + 1;
            self.data.scenes.push(SceneNode::Transform {
                attributes: Default::default(),
                frames: vec![Frame {
                    attributes: AHashMap::from([(
                        "_t".to_string(),
                        format!("{} {} {}", x * MODEL_EDGE, y * MODEL_EDGE, z * MODEL_EDGE),
                    )]),
                }],
                child: shape_node as u32,
                layer_id: 0,
            });
            self.data.scenes.push(SceneNode::Shape {
                attributes: Default::default(),
                models: vec![ShapeModel {
                    model_id: model_index as u32,
                    attributes: Default::default(),
                }],
            });

            // Add to the transform node to the root group
            let root_group = &mut self.data.scenes[1];
            match root_group {
                SceneNode::Group {
                    attributes: _,
                    children,
                } => children.push(transform_node as u32),
                _ => unreachable!(),
            }

            next_index
        });
        &mut self.data.models[*index]
    }

    pub fn add_voxel(&mut self, x: i32, y: i32, z: i32, i: u8) {
        let (model_coords, (subx, suby, subz)) = voxel_coords(x, y, z);
        let model = self.get_or_insert_model(model_coords);
        model.voxels.push(Voxel {
            x: subx,
            y: suby,
            z: subz,
            i,
        });
    }
}

impl From<DotVoxBuilder> for DotVoxData {
    fn from(value: DotVoxBuilder) -> Self {
        value.data
    }
}

fn voxel_coords(x: i32, y: i32, z: i32) -> ((i32, i32, i32), (u8, u8, u8)) {
    let (x, sub_x) = div_mod_floor(x, MODEL_EDGE);
    let (y, sub_y) = div_mod_floor(y, MODEL_EDGE);
    let (z, sub_z) = div_mod_floor(z, MODEL_EDGE);

    ((x, y, z), (sub_x as u8, sub_y as u8, sub_z as u8))
}

#[ext]
pub impl Material {
    fn diffuse(id: u32) -> Self {
        Self {
            id,
            properties: AHashMap::from([
                ("_rough".to_string(), "0.1".to_string()),
                ("_ior".to_string(), "0.3".to_string()),
                ("_d".to_string(), "0.05".to_string()),
            ]),
        }
    }
    fn metal(id: u32, metal: f32, rough: f32, ior: f32, sp: f32) -> Self {
        Self {
            id,
            properties: AHashMap::from([
                ("_type".to_string(), "_metal".to_string()),
                ("_metal".to_string(), metal.to_string()),
                ("_rough".to_string(), rough.to_string()),
                ("_ior".to_string(), ior.to_string()),
                ("_sp".to_string(), sp.to_string()),
            ]),
        }
    }
    fn emit(id: u32, emit: f32, flux: u8, ldr: f32) -> Self {
        Self {
            id,
            properties: AHashMap::from([
                ("_type".to_string(), "_emit".to_string()),
                ("_emit".to_string(), emit.to_string()),
                ("_flux".to_string(), flux.to_string()),
                ("_ldr".to_string(), ldr.to_string()),
            ]),
        }
    }
    fn glass(id: u32, rough: f32, ior: f32, trans: f32, density: f32) -> Self {
        Self {
            id,
            properties: AHashMap::from([
                ("_type".to_string(), "_glass".to_string()),
                ("_trans".to_string(), trans.to_string()),
                ("_alpha".to_string(), trans.to_string()),
                ("_rough".to_string(), rough.to_string()),
                ("_ior".to_string(), ior.to_string()),
                ("_d".to_string(), density.to_string()),
            ]),
        }
    }
}
