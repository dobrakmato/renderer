use bf::mesh::VertexFormat;

/// Extension trait to add utility methods to `VertexFormat` types.
pub trait VertexFormatExt {
    /// Returns whether this format contains vertex positions.
    fn has_position(&self) -> bool;

    /// Returns whether this format contains vertex normals.
    fn has_normals(&self) -> bool;

    /// Returns whether this format contains UVs.
    fn has_uvs(&self) -> bool;

    /// Returns whether this format contains tangents.
    fn has_tangents(&self) -> bool;

    /// Returns the length of padding at the end specified in number of bytes.
    fn padding_length(&self) -> usize;
}

impl VertexFormatExt for VertexFormat {
    fn has_position(&self) -> bool {
        true
    }

    fn has_normals(&self) -> bool {
        match self {
            VertexFormat::PositionNormalUvTangent => true,
            VertexFormat::PositionNormalUv => true,
            VertexFormat::Position => false,
        }
    }

    fn has_uvs(&self) -> bool {
        match self {
            VertexFormat::PositionNormalUvTangent => true,
            VertexFormat::PositionNormalUv => true,
            VertexFormat::Position => false,
        }
    }

    fn has_tangents(&self) -> bool {
        match self {
            VertexFormat::PositionNormalUvTangent => true,
            VertexFormat::PositionNormalUv => false,
            VertexFormat::Position => false,
        }
    }

    fn padding_length(&self) -> usize {
        match self {
            VertexFormat::PositionNormalUvTangent => 4,
            VertexFormat::PositionNormalUv => 0,
            VertexFormat::Position => 4,
        }
    }
}
