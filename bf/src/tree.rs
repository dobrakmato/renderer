use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Handle to a single `Node` element stored inside a `Tree`. Each instance of
/// handle represents a valid handle.
#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize, PartialOrd, Hash)]
pub struct Handle(usize);

/// Component (data stored in the node). Single `Node` may have multiple
/// components even of the same type. This however does not make sense
/// always. Some configuration are thus meaningless (for example having
/// multiple `Transform`s in a single `Node`).
#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Component {
    Transform {
        position: [f32; 3],
        rotation: [f32; 3],
        scale: [f32; 3],
    },
    MeshRenderer {
        mesh: Uuid,
        material: Uuid,
    },
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    name: Option<String>,
    children: Vec<Handle>,
    components: Vec<Component>,
}

impl Node {
    pub fn components(&self) -> impl Iterator<Item = &Component> {
        self.components.iter()
    }

    pub fn component_by_type(&self) -> &Component {
        self.components.iter().first(|x| 1)
    }
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Tree {
    nodes: Vec<Node>,
    root: Option<Handle>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            root: None,
        }
    }

    pub fn add_node(&mut self) -> &mut Node {
        let node = Node {
            name: Option::None,
            children: vec![],
            components: vec![],
        };

        self.nodes.push(node);

        return self.nodes.get_mut(self.nodes.len() - 1).unwrap();
    }

    pub fn root(&self) -> Option<&Node> {
        self.root.map(|x| self.nodes.get(x.0)).flatten()
    }

    pub fn node(&self, handle: Handle) -> &Node {
        self.nodes.get(handle.0).expect("invalid tree")
    }
}

#[cfg(test)]
mod tests {
    use crate::tree::{Component, Node, Tree};
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn can_construct() {
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
        let mut root = tree.add_node();
    }
}
