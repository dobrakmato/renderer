use std::sync::Arc;
use vulkano::buffer::ImmutableBuffer;

/// Marker trait for types that can be used as indices in index
/// buffer.
pub trait IndexType {}

impl IndexType for u8 {}

impl IndexType for u16 {}

impl IndexType for u32 {}

/// Defines a renderable geometry with geometry data already
/// loaded in GPU.
pub struct Mesh<VDef, I>
where
    I: IndexType,
{
    pub vertex_buffer: Arc<ImmutableBuffer<[VDef]>>,
    pub index_buffer: Arc<ImmutableBuffer<[I]>>,
}
