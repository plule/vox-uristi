//! General helper to build dot_vox structures

use derive_more::Deref;
use dot_vox::{Dict, DotVoxData, Frame, Layer, Material, Model, SceneNode, ShapeModel, Size};
use easy_ext::ext;

use crate::coords::DotVoxModelCoords;

#[derive(Debug, Clone, Copy, Deref)]
pub struct LayerId(pub usize);

impl From<LayerId> for u32 {
    fn from(value: LayerId) -> Self {
        *value as u32
    }
}

#[derive(Debug, Clone, Copy, Deref)]
pub struct NodeId(pub usize);

impl From<NodeId> for u32 {
    fn from(value: NodeId) -> Self {
        *value as u32
    }
}

#[derive(Debug, Clone, Copy, Deref)]
pub struct ModelId(pub usize);

impl From<ModelId> for u32 {
    fn from(value: ModelId) -> Self {
        *value as u32
    }
}

pub struct DotVoxBuilder {
    // The .vox raw data
    pub data: DotVoxData,

    pub root_group: NodeId,
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

                layers: vec![
                    Layer {
                        attributes: Default::default(),
                    };
                    32
                ],
                index_map: vec![],
            },
            root_group: NodeId(1),
        }
    }
}

impl DotVoxBuilder {
    pub fn new_model(size: Size) -> Model {
        Model {
            size,
            voxels: vec![],
        }
    }

    fn insert_model(&mut self, model: Model) -> ModelId {
        let index = self.data.models.len();
        self.data.models.push(model);
        ModelId(index)
    }

    fn insert_node(&mut self, node: SceneNode) -> NodeId {
        let index = self.data.scenes.len();
        self.data.scenes.push(node);
        NodeId(index)
    }

    fn insert_child_to_group(&mut self, parent_group: NodeId, child: NodeId) {
        let parent_group = &mut self.data.scenes[*parent_group];
        match parent_group {
            SceneNode::Group {
                attributes: _,
                children,
            } => children.push(child.into()),
            _ => panic!("Parent node is not a group"),
        }
    }

    // Insert the transform/group pair, return the group index
    pub fn insert_group_node(
        &mut self,
        parent_group: NodeId,
        transform_attributes: Dict,
        frames: Vec<Frame>,
        layer_id: LayerId,
        group_attributes: Dict,
    ) -> NodeId {
        // Insert the transform and group pair
        let group_id = self.insert_node(SceneNode::Group {
            attributes: group_attributes,
            children: vec![],
        });
        let transform_index = self.insert_node(SceneNode::Transform {
            attributes: transform_attributes,
            frames,
            child: group_id.into(),
            layer_id: layer_id.into(),
        });

        // Add to the transform node to the parent group
        self.insert_child_to_group(parent_group, transform_index);
        group_id
    }

    pub fn insert_group_node_simple(
        &mut self,
        parent_group: NodeId,
        name: impl Into<String>,
        coordinates: Option<DotVoxModelCoords>,
        layer_id: LayerId,
    ) -> NodeId {
        let transform_attributes = Dict::from([("_name".to_string(), name.into())]);
        let mut frames = Vec::new();
        if let Some(coordinates) = coordinates {
            frames.push(Frame {
                attributes: Dict::from([(
                    "_t".to_string(),
                    format!("{} {} {}", coordinates.x, coordinates.y, coordinates.z),
                )]),
            });
        }
        self.insert_group_node(
            parent_group,
            transform_attributes,
            frames,
            layer_id,
            Default::default(),
        )
    }

    // Insert the transform/shape pair, return the shape index
    pub fn insert_shape_node(
        &mut self,
        parent_group: NodeId,
        transform_attributes: Dict,
        frames: Vec<Frame>,
        layer_id: LayerId,
        shape_attributes: Dict,
        models: Vec<ShapeModel>,
    ) -> NodeId {
        // Insert the transform and shape pair
        let shape_index = self.insert_node(SceneNode::Shape {
            attributes: shape_attributes,
            models,
        });
        let transform_index = self.insert_node(SceneNode::Transform {
            attributes: transform_attributes,
            frames,
            child: shape_index.into(),
            layer_id: layer_id.into(),
        });

        // Add to the transform node to the parent group
        self.insert_child_to_group(parent_group, transform_index);
        shape_index
    }

    pub fn insert_shape_node_simple(
        &mut self,
        parent_group: NodeId,
        name: impl Into<String>,
        coordinates: Option<DotVoxModelCoords>,
        layer_id: LayerId,
        model: ModelId,
    ) -> NodeId {
        let transform_attributes = Dict::from([("_name".to_string(), name.into())]);
        let mut frames = Vec::new();
        if let Some(coordinates) = coordinates {
            frames.push(Frame {
                attributes: Dict::from([(
                    "_t".to_string(),
                    format!("{} {} {}", coordinates.x, coordinates.y, coordinates.z),
                )]),
            });
        }
        self.insert_shape_node(
            parent_group,
            transform_attributes,
            frames,
            layer_id,
            Default::default(),
            vec![ShapeModel {
                model_id: model.into(),
                attributes: Default::default(),
            }],
        )
    }

    /// Insert a model in the .vox data, return its index
    pub fn insert_model_and_shape_node(
        &mut self,
        parent_group: NodeId,
        coordinates: Option<DotVoxModelCoords>,
        model: Model,
        layer_id: LayerId,
        name: impl Into<String>,
    ) -> ModelId {
        let index = self.insert_model(model);

        // Insert the transform and shape nodes for this model in the scene graph
        let transform_attributes = Dict::from([("_name".to_string(), name.into())]);
        let mut frames = Vec::new();
        if let Some(coordinates) = coordinates {
            frames.push(Frame {
                attributes: Dict::from([(
                    "_t".to_string(),
                    format!("{} {} {}", coordinates.x, coordinates.y, coordinates.z),
                )]),
            });
        }
        self.insert_shape_node(
            parent_group,
            transform_attributes,
            frames,
            layer_id,
            Default::default(),
            vec![ShapeModel {
                model_id: index.into(),
                attributes: Default::default(),
            }],
        );
        index
    }

    pub fn insert_model_and_group(
        &mut self,
        parent_group: NodeId,
        name: impl Into<String>,
        model: Model,
        layer_id: LayerId,
    ) {
        let name: String = name.into();
        let group = self.insert_group_node_simple(parent_group, name.clone(), None, layer_id);
        self.insert_model_and_shape_node(group, None, model, layer_id, name);
    }
}

impl From<DotVoxBuilder> for DotVoxData {
    fn from(value: DotVoxBuilder) -> Self {
        value.data
    }
}

#[ext(MaterialExt)]
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_nodes() {
        let mut builder = DotVoxBuilder::default();
        let attributes = Dict::from([("_name".to_string(), "group".to_string())]);
        let group = builder.insert_node(SceneNode::Group {
            attributes: attributes.clone(),
            children: vec![],
        });
        assert!(matches!(
            builder.data.scenes[*group],
            SceneNode::Group { .. }
        ));
        let transform = builder.insert_node(SceneNode::Transform {
            attributes: Default::default(),
            frames: vec![Frame {
                attributes: Default::default(),
            }],
            child: group.into(),
            layer_id: 0,
        });
        assert!(matches!(
            builder.data.scenes[*transform],
            SceneNode::Transform { .. }
        ));

        builder.insert_child_to_group(group, transform);
        assert_eq!(
            builder.data.scenes[*transform],
            SceneNode::Transform {
                attributes: Default::default(),
                frames: vec![Frame {
                    attributes: Default::default(),
                }],
                child: group.into(),
                layer_id: 0,
            }
        );
    }

    #[test]
    fn insert_group_node() {
        let mut builder = DotVoxBuilder::default();
        let group = builder.insert_group_node(
            builder.root_group,
            Default::default(),
            vec![Frame {
                attributes: Default::default(),
            }],
            LayerId(0),
            Default::default(),
        );
        assert!(matches!(
            builder.data.scenes[*group],
            SceneNode::Group { .. }
        ));
    }

    #[test]
    fn insert_shape_node() {
        let mut builder = DotVoxBuilder::default();
        let model = Model {
            size: Size { x: 1, y: 1, z: 1 },
            voxels: vec![],
        };
        let index = builder.insert_model(model);
        let shape = builder.insert_shape_node(
            builder.root_group,
            Default::default(),
            vec![Frame {
                attributes: Default::default(),
            }],
            LayerId(0),
            Default::default(),
            vec![ShapeModel {
                model_id: index.into(),
                attributes: Default::default(),
            }],
        );
        match &builder.data.scenes[*shape] {
            SceneNode::Shape { models, .. } => {
                assert_eq!(1, models.len());
                assert_eq!(u32::from(index), models[0].model_id);
            }
            _ => panic!("Expected a shape node"),
        }
    }

    #[test]
    fn insert_model() {
        let mut builder = DotVoxBuilder::default();
        let model = Model {
            size: Size { x: 1, y: 1, z: 1 },
            voxels: vec![],
        };
        let index = builder.insert_model(model);
        assert_eq!(
            builder.data.models[*index],
            Model {
                size: Size { x: 1, y: 1, z: 1 },
                voxels: vec![],
            }
        );
    }

    #[test]
    fn insert_model_shape() {
        let mut builder = DotVoxBuilder::default();
        let model = Model {
            size: Size { x: 1, y: 1, z: 1 },
            voxels: vec![],
        };
        let index = builder.insert_model_and_shape_node(
            builder.root_group,
            None,
            model,
            LayerId(0),
            "test",
        );
        match &builder.data.scenes[*builder.root_group] {
            SceneNode::Group { children, .. } => {
                assert_eq!(1, children.len());
            }
            _ => panic!("Expected a group node"),
        }
        let inserted_model = &builder.data.models[*index];
        assert_eq!(
            inserted_model,
            &Model {
                size: Size { x: 1, y: 1, z: 1 },
                voxels: vec![],
            }
        );
    }
}
