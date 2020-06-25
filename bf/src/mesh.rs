//! Indexed triangular meshes stored in specified vertex format.

use serde::{Deserialize, Serialize};

/// Represents the individual vertex attributes, their loading and
/// padding inside a single vertex in the vertex buffer.
#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum VertexFormat {
    // vec3(pos), vec3(nor), vec2(uv), vec3(tangent) + 1 byte padding
    PositionNormalUvTangent,
    // vec3(pos), vec3(nor), vec2(uv)
    PositionNormalUv,
    // vec3(pos) + 4 byte padding
    Position,
}

impl VertexFormat {
    /// Returns the size in bytes of one vertex of this type.
    #[inline]
    pub fn size_of_one_vertex(self) -> usize {
        match self {
            VertexFormat::PositionNormalUvTangent => std::mem::size_of::<f32>() * 12,
            VertexFormat::PositionNormalUv => std::mem::size_of::<f32>() * 8,
            VertexFormat::Position => std::mem::size_of::<f32>() * 4,
        }
    }
}

/// Represents a type that is used as index in the index buffer.
///
/// Only supported index formats are `u16` and `u32` according to *Vulkan* specification.
#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum IndexType {
    U16,
    U32,
}

impl IndexType {
    /// Returns the size in bytes of one index of this type.
    #[inline]
    pub fn size_of_one_index(self) -> usize {
        match self {
            IndexType::U16 => std::mem::size_of::<u16>(),
            IndexType::U32 => std::mem::size_of::<u32>(),
        }
    }
}

/// Asset type that is used to store indexed triangular geometry data. Each mesh has specified
/// format of vertex data and index type.
#[derive(Debug, Serialize, Deserialize)]
pub struct Mesh {
    pub vertex_format: VertexFormat,
    #[serde(with = "serde_bytes")]
    pub vertex_data: Vec<u8>,
    pub index_type: IndexType,
    #[serde(with = "serde_bytes")]
    pub index_data: Vec<u8>,
}
