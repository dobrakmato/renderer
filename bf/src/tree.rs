//! Scene tree structures & scene and prefab serialization.
//!
//! # Programmatically creating a scene
//!
//! The simplest way to create a single level deep scene tree is to
//! use the `scene_tree!` and `scene_node!` macros.
//!
//! ```ignore
//! use bf::tree::Component;
//! use bf::scene_tree;
//! use bf::scene_node;
//!
//! let scene = scene_tree!(
//!     scene_node!(Component::Name("test".into())),
//!     scene_node!(
//!         Component::Name("sky".into()),
//!         Component::Sky {
//!             turbidity: 2.0,
//!             ground_albedo: [0.0, 0.0, 1.0]
//!         }
//!     )
//! );
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque struct representing a "pointer" to a single `Node` element stored
/// inside a `Tree`. Each instance of this struct represents a valid handle.
#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize, PartialOrd, Hash)]
pub struct Handle(usize);

impl Handle {
    /// Determines whether this instance is valid when used in specified
    /// tree instance.
    fn is_valid(&self, tree: &Tree) -> bool {
        self.0 < tree.nodes.len()
    }
}

/// Component (data stored in the node). Single `Node` may have multiple
/// components even of the same type. This however does not make sense
/// always. Some configuration are thus meaningless (for example having
/// multiple `Transform`s in a single `Node`).
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Component {
    /// Name of the node.
    Name(String),
    /// Sky renderer to this node.
    Sky {
        turbidity: f32,
        ground_albedo: [f32; 3],
    },
    /// 3D transform (position, rotation, scale) to this node.
    Transform {
        position: [f32; 3],
        rotation: [f32; 3],
        scale: [f32; 3],
    },
    /// Mesh renderer with specified material.
    MeshRenderer { mesh: Uuid, material: Uuid },
    /// Directional light.
    DirectionalLight {
        direction: [f32; 3],
        intensity: f32,
        color: [f32; 3],
    },
}

/// Single entry in the `Tree`. Each node can have multiple (or zero)
/// children nodes. It also contains a `Vec` of `Component`s attached
/// to this node.
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    children: Vec<Handle>,
    components: Vec<Component>,
}

impl Node {
    /// Adds a specified component to the vector of components attached
    /// to this `Node`.
    pub fn add_component(&mut self, component: Component) {
        self.components.push(component)
    }

    /// Returns iterator over shared references to `Component`s of this `Node`.
    pub fn components(&self) -> impl Iterator<Item = &Component> {
        self.components.iter()
    }

    /// Returns iterator over unique references to `Component`s of this `Node`.
    pub fn components_mut(&mut self) -> impl Iterator<Item = &mut Component> {
        self.components.iter_mut()
    }

    /// Returns iterator over `Handle`s of children nodes of this `Node`.
    pub fn children(&self) -> impl Iterator<Item = &Handle> {
        self.children.iter()
    }

    /// Adds the node specified by the `Handle` to the list of
    /// children nodes of this node.
    pub fn add_child(&mut self, handle: Handle) {
        self.children.push(handle)
    }
}

/// Creates a new `Node` struct. Accepts variable number of `Component` arguments.
#[macro_export]
macro_rules! scene_node {
    ($($component: expr),+) => {
        crate::tree::Node {
            children: vec![],
            components: vec![$($component),+],
        }
    };
}

/// Creates a new `Tree` struct. Accept variable number of `Node` arguments
/// which will be inserted as children of the tree's root node.
#[macro_export]
macro_rules! scene_tree {
    ($($node: expr),+) => {{
        let mut t = crate::tree::Tree::new();

        $(
            {
                let handle = t.add_node($node);
                t.root_mut().add_child(handle);
            }
        )+

        t
    }};
}

/// Represents a tree-like structure that owns the arena-allocated
/// memory backing the storage for the individual tree nodes.
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Tree {
    nodes: Vec<Node>,
    root: Handle,
}

impl Tree {
    /// Creates a new `Tree` with a root node.
    pub fn new() -> Self {
        let root = Node {
            children: vec![],
            components: vec![Component::Name("root".into())],
        };

        Self {
            nodes: vec![root],
            root: Handle(0),
        }
    }

    /// Performs the validation of Handles in all child nodes.
    pub(crate) fn validate_handles(self) -> Result<Self, TreeError> {
        if !self.root.is_valid(&self) {
            return Err(TreeError::InvalidRoot { handle: self.root });
        }

        // validate all handle in all nodes
        for (idx, node) in self.nodes.iter().enumerate() {
            for handle in node.children.iter() {
                if !handle.is_valid(&self) {
                    return Err(TreeError::InvalidHandle {
                        handle: *handle,
                        at: Handle(idx),
                    });
                }
            }
        }

        return Ok(self);
    }

    /// Allocates a space and adds the specified `Node` struct to
    /// this `Tree`. Returns `Handle` to the specified `Node` that
    /// can be used to access it.
    pub fn add_node(&mut self, node: Node) -> Handle {
        self.nodes.push(node);

        Handle(self.nodes.len() - 1)
    }

    /// Returns shared reference to root node of this tree.
    pub fn root(&self) -> &Node {
        self.nodes.get(self.root.0).expect("invalid tree")
    }

    /// Returns unique reference to root node of this tree.
    pub fn root_mut(&mut self) -> &mut Node {
        self.nodes.get_mut(self.root.0).expect("invalid tree")
    }

    /// Returns shared reference to node specified by the handle.
    pub fn node(&self, handle: &Handle) -> &Node {
        self.nodes.get(handle.0).expect("invalid tree")
    }

    /// Returns unique reference to node specified by the handle.
    pub fn node_mut(&mut self, handle: &Handle) -> &mut Node {
        self.nodes.get_mut(handle.0).expect("invalid tree")
    }
}

/// Possible errors that may happen when loading a `Tree`.
pub enum TreeError {
    NotATree,
    InvalidRoot { handle: Handle },
    InvalidHandle { handle: Handle, at: Handle },
}

#[cfg(test)]
mod tests {
    use crate::tree::{Component, Tree};
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn can_construct() {
        let name = Component::Name("Model".into());

        let transform = Component::Transform {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        };

        let mesh = Component::MeshRenderer {
            material: Uuid::from_str("4e8a9c8a-ed09-4f9b-8616-5508e1042213").unwrap(),
            mesh: Uuid::from_str("625dc4fc-9274-4b8d-97f2-d3a466f4501c").unwrap(),
        };

        let mut tree = Tree::new();
        let node = tree.add_node(scene_node!(name, transform, mesh));

        tree.root_mut().add_child(node);
    }

    #[test]
    fn can_construct_using_tree_macro() {
        let _ = scene_tree!(
            scene_node!(Component::Name("test".into())),
            scene_node!(
                Component::Name("sky".into()),
                Component::Sky {
                    turbidity: 2.0,
                    ground_albedo: [0.0, 0.0, 1.0]
                }
            )
        );
    }
}
