use ahash::{AHashMap, HashMap};
use dot_vox::{DotVoxData, Frame, Model, SceneNode, ShapeModel, Size, Voxel};
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
                palette: vec![],
                materials: vec![],
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
