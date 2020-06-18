//! Meshes and functions used to created meshes.

use crate::render::vertex::PositionOnlyVertex;
use bf::mesh::IndexType;
use safe_transmute::{Error, TriviallyTransmutable};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, ImmutableBuffer};
use vulkano::device::Queue;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::input_assembly::Index;
use vulkano::pipeline::vertex::Vertex;
use vulkano::sync::GpuFuture;

/// Renderable indexed triangular geometry with specified vertex format
/// and index type.
pub struct IndexedMesh<V, I>
where
    V: Vertex,
    I: Index,
{
    /// Vertex buffer.
    vertex_buffer: Arc<ImmutableBuffer<[V]>>,
    /// Index buffer.
    index_buffer: Arc<ImmutableBuffer<[I]>>,
}

impl<V, I> IndexedMesh<V, I>
where
    V: Vertex,
    I: Index,
{
    /// Creates a new `Mesh` from provided buffers.
    pub fn new(
        vertex_buffer: Arc<ImmutableBuffer<[V]>>,
        index_buffer: Arc<ImmutableBuffer<[I]>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            vertex_buffer,
            index_buffer,
        })
    }

    /// Returns the `Arc` reference to vertex buffer of this mesh.
    #[inline]
    pub fn vertex_buffer(&self) -> &Arc<ImmutableBuffer<[V]>> {
        &self.vertex_buffer
    }

    /// Returns the `Arc` reference to index buffer of this mesh.
    #[inline]
    pub fn index_buffer(&self) -> &Arc<ImmutableBuffer<[I]>> {
        &self.index_buffer
    }
}

/// Possible errors that can happen when creating a buffer.
#[derive(Debug)]
pub enum CreateBufferError {
    /// Generic parameters representing a single element in the created buffer
    /// is of incorrect type.
    IncorrectElementType,
    /// The buffer couldn't be allocated.
    CannotAllocateBuffer(DeviceMemoryAllocError),
}

/// Helper function to create a GPU buffer from array elements of type `T` encoded
/// as array of bytes.
///
/// This function is internally used by [`create_mesh`](fn.create_mesh.html) fucntion.
fn create_buffer<T>(
    bytes: &[u8],
    queue: Arc<Queue>,
    usage: BufferUsage,
) -> Result<(Arc<ImmutableBuffer<[T]>>, impl GpuFuture), CreateBufferError>
where
    T: TriviallyTransmutable + Send + Sync + 'static,
{
    fn possible_non_zero_copy<'a, T: TriviallyTransmutable>(
        bytes: &'a [u8],
        possible_owner: &'a mut std::vec::Vec<T>,
    ) -> &'a [T] {
        match safe_transmute::transmute_many_pedantic::<T>(bytes) {
            Ok(safe) => safe,
            Err(Error::Unaligned(e)) => {
                log::error!(
                    "cannot zero-copy unaligned &[{:?}] data: {:?}",
                    std::any::type_name::<T>(),
                    e
                );
                *possible_owner = e.copy();
                possible_owner.as_slice()
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    // copy data to correctly aligned temporary array
    let mut index_vec = Vec::new();
    let items = possible_non_zero_copy::<T>(bytes, &mut index_vec);

    // copy data from temporary aligned array to staging buffer and
    // then issue gpu-copy between staging and final buffer
    let (buffer, future) = ImmutableBuffer::from_iter(items.iter().cloned(), usage, queue)
        .map_err(CreateBufferError::CannotAllocateBuffer)?;

    Ok((buffer, future))
}

/// This function creates a `Mesh` struct from provided `bf::mesh::Mesh` asset
/// without any conversion. This function returns the mesh and `GpuFuture` that
/// represents the time when both buffers (and thus the mesh) are ready to use.
pub fn create_mesh<V, I>(
    from: &bf::mesh::Mesh,
    queue: Arc<Queue>,
) -> Result<(Arc<IndexedMesh<V, I>>, impl GpuFuture), CreateBufferError>
where
    V: Vertex + TriviallyTransmutable + Send + Sync + 'static,
    I: Index + TriviallyTransmutable + Send + Sync + 'static,
{
    // verify that the method was invoked with correct index type
    if from.index_type.size_of_one_index() != std::mem::size_of::<I>() {
        return Err(CreateBufferError::IncorrectElementType);
    }

    // verify that the method was invoked with correct index type
    if from.vertex_format.size_of_one_vertex() != std::mem::size_of::<V>() {
        return Err(CreateBufferError::IncorrectElementType);
    }

    let (vertex, f1) = create_buffer(
        from.vertex_data.as_slice(),
        queue.clone(),
        BufferUsage::vertex_buffer(),
    )?;
    let (index, f2) = create_buffer(
        from.index_data.as_slice(),
        queue,
        BufferUsage::index_buffer(),
    )?;

    Ok((IndexedMesh::new(vertex, index), f1.join(f2)))
}

/// Generates a new `Mesh` instance that is a full-screen triangle that can be used
/// to perform full-screen passes. This function returns the mesh and `GpuFuture` that
/// represents the time when both buffers (and thus the mesh) are ready to use.
pub fn create_full_screen_triangle(
    queue: Arc<Queue>,
) -> Result<(Arc<IndexedMesh<PositionOnlyVertex, u16>>, impl GpuFuture), DeviceMemoryAllocError> {
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
        IndexedMesh::new(vertex_buffer, index_buffer),
        vbo_future.join(ibo_future),
    ))
}

/// Renderable indexed triangular geometry with specified vertex format
/// and **dynamic runtime chosen** index format.
///
/// You need to always match on variant before using the inner `IndexeMesh`.
pub enum DynamicIndexedMesh<V: Vertex> {
    U16(IndexedMesh<V, u16>),
    U32(IndexedMesh<V, u32>),
}

/// Result of [`create_mesh_dynamic`](fn.create_mesh_dynamic.html) function invocation.
pub type DynamicIndexedMeshResult<V> =
    Result<(Arc<DynamicIndexedMesh<V>>, Box<dyn GpuFuture>), CreateBufferError>;

/// Same as [`create_mesh`](fn.create_mesh.html) except the index type is chosen at
/// runtime.
///
/// This function creates a `DynamicMesh` enum from provided `bf::mesh::Mesh` asset
/// without any conversion. It automatically select the appropriate index type based
/// on the information in `mesh` parameters.
///
/// This function returns the mesh and `GpuFuture` that represents the time when both
/// buffers (and thus the mesh) are ready to use.
pub fn create_mesh_dynamic<V: Vertex + TriviallyTransmutable>(
    mesh: &bf::mesh::Mesh,
    queue: Arc<Queue>,
) -> DynamicIndexedMeshResult<V> {
    macro_rules! impl_for_types {
        ($($typ:ident),+) => {
            match mesh.index_type {
                $(IndexType::$typ => match create_mesh(&mesh, queue) {
                    Ok((t, f)) => return Ok((
                        Arc::new(DynamicIndexedMesh::$typ(match Arc::try_unwrap(t) {
                            Ok(t) => t,
                            Err(_) => unreachable!(),
                        })),
                        f.boxed(),
                    )),
                    Err(e) => {
                        return Err(e)
                    }
                }),+
            }
        };
    }

    impl_for_types!(U16, U32);
}
