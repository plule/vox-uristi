use dot_vox::{Dict, DotVoxData, Frame, Material, Model, SceneNode, ShapeModel, Size, Voxel};
use easy_ext::ext;
use num_integer::div_mod_floor;
use std::collections::HashMap;

const MODEL_EDGE: i32 = 256;
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
                palette: (0..256)
                    .map(|_| dot_vox::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    })
                    .collect(),
                materials: (0..256)
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
                    attributes: Dict::from([(
                        "_t".to_string(),
                        format!(
                            "{} {} {}",
                            x * MODEL_EDGE + MODEL_EDGE / 2,
                            y * MODEL_EDGE + MODEL_EDGE / 2,
                            z * MODEL_EDGE + MODEL_EDGE / 2
                        ),
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

#[ext(MaterialExt)]
pub impl Material {
    fn with_id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    fn diffuse(id: u32) -> Self {
        Self {
            id,
            properties: Dict::from([
                ("_rough".to_string(), "0.1".to_string()),
                ("_ior".to_string(), "0.3".to_string()),
                ("_d".to_string(), "0.05".to_string()),
            ]),
        }
    }

    fn set_type(&mut self, type_: &str) {
        self.set_str("_type", type_);
    }

    fn set_f32(&mut self, prop: &str, value: f32) {
        self.properties.insert(prop.to_string(), value.to_string());
    }

    fn set_str(&mut self, prop: &str, value: &str) {
        self.properties.insert(prop.to_string(), value.to_string());
    }

    fn set_diffuse(&mut self) {
        self.set_str("_type", "_diffuse");
    }

    fn set_metal(&mut self) {
        self.set_str("_type", "_metal");
    }

    fn set_roughness(&mut self, roughness: f32) {
        self.set_f32("_rough", roughness);
    }

    fn set_ior(&mut self, ior: f32) {
        self.set_f32("_ior", ior);
    }

    fn set_specular(&mut self, specular: f32) {
        self.set_f32("_sp", specular);
    }

    fn set_metalness(&mut self, metalness: f32) {
        self.set_f32("_metal", metalness);
    }

    fn set_emissive(&mut self) {
        self.set_str("_type", "_emit");
    }

    fn set_emit(&mut self, emit: f32) {
        self.set_f32("_emit", emit);
    }

    fn set_flux(&mut self, flux: f32) {
        self.set_f32("_flux", flux);
    }

    fn set_ldr(&mut self, ldr: f32) {
        self.set_f32("_ldr", ldr);
    }
    fn set_glass(&mut self) {
        self.set_str("_type", "_glass");
    }
    fn set_transparency(&mut self, trans: f32) {
        self.set_f32("_trans", trans);
        self.set_f32("_alpha", trans);
    }
    fn set_density(&mut self, density: f32) {
        self.set_f32("_d", density);
    }
}
