use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum VertexFormat {
    // vec3(pos), vec3(nor), vec2(uv), vec3(tangent) + 1 byte padding
    PositionNormalUvTangent,
}

impl VertexFormat {
    /// Returns the size in bytes of one vertex of this type.
    #[inline]
    pub fn size_of_one_vertex(self) -> usize {
        match self {
            VertexFormat::PositionNormalUvTangent => std::mem::size_of::<f32>() * 12,
        }
    }
}

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
pub struct Mesh<'a> {
    pub vertex_format: VertexFormat,
    pub vertex_data: &'a [u8],
    pub index_type: IndexType,
    pub index_data: &'a [u8],
}
