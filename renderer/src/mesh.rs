use crate::render::PositionOnlyVertex;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, ImmutableBuffer};
use vulkano::device::Queue;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::sync::GpuFuture;

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

const VERTEX_DATA_FST: [PositionOnlyVertex; 3] = [
    PositionOnlyVertex {
        position: [-1.0, -1.0, 0.0],
    },
    PositionOnlyVertex {
        position: [3.0, -1.0, 0.0],
    },
    PositionOnlyVertex {
        position: [-1.0, 3.0, 0.0],
    },
];
const INDEX_DATA_FST: [u16; 3] = [0, 1, 2];

/// Generates a new Mesh instance that is a full-screen triangle that can be used
/// to perform full-screen passes. This f unction returns the mesh and future that
/// represents when both buffers (and thus the mesh) are ready to use.
pub fn create_full_screen_triangle(
    queue: Arc<Queue>,
) -> Result<(Mesh<PositionOnlyVertex, u16>, impl GpuFuture), DeviceMemoryAllocError> {
    let (vertex_buffer, vbo_future) = ImmutableBuffer::from_iter(
        (&VERTEX_DATA_FST).iter().cloned(),
        BufferUsage::vertex_buffer(),
        queue.clone(),
    )?;
    let (index_buffer, ibo_future) = ImmutableBuffer::from_iter(
        (&INDEX_DATA_FST).iter().cloned(),
        BufferUsage::index_buffer(),
        queue,
    )?;
    Ok((
        Mesh {
            vertex_buffer,
            index_buffer,
        },
        vbo_future.join(ibo_future),
    ))
}
