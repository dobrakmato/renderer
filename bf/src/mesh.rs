use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum VertexDataFormat {
    // vec3(pos), vec3(nor), vec2(uv), vec3(tangent) + 1 byte padding
    PositionNormalUvTangent,
}

impl VertexDataFormat {
    #[inline]
    pub fn size_of_one_vertex(self) -> usize {
        match self {
            VertexDataFormat::PositionNormalUvTangent => std::mem::size_of::<f32>() * 12,
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum IndexType {
    U16,
    U32,
}

impl IndexType {
    #[inline]
    pub fn size_of_one_index(self) -> usize {
        match self {
            IndexType::U16 => std::mem::size_of::<u16>(),
            IndexType::U32 => std::mem::size_of::<u32>(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mesh<'a> {
    pub vertex_format: VertexDataFormat,
    pub vertex_data: &'a [u8],
    pub index_type: IndexType,
    pub index_data: &'a [u8],
}
