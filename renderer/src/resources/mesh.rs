//! Meshes and functions used to created meshes.

use crate::render::vertex::PositionOnlyVertex;
use bf::mesh::IndexType;
use safe_transmute::{Error, TriviallyTransmutable};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
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
    IncorrectElementType(&'static str),
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
        return Err(CreateBufferError::IncorrectElementType(
            "Index type is incorrect",
        ));
    }

    // verify that the method was invoked with correct index type
    if from.vertex_format.size_of_one_vertex() != std::mem::size_of::<V>() {
        return Err(CreateBufferError::IncorrectElementType(
            "Vertex type is incorrect",
        ));
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
            position: [-1.0, -1.0, 0.0, 0.0],
        },
        PositionOnlyVertex {
            position: [3.0, -1.0, 0.0, 0.0],
        },
        PositionOnlyVertex {
            position: [-1.0, 3.0, 0.0, 0.0],
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

/// Generates a new `Mesh` instance that is a icosphere. First the icosahedron is
/// generated, then more faces are added depending on the level of refinement.
///
/// Refinement level zero will cause this function to generate icosahedron. Each
/// level will cause to subdivide triangle into 4 triangles.
///
/// This function returns the mesh and `GpuFuture` that represents the time when
/// both buffers (and thus the mesh) are ready to use.
///
/// ![Icosahderon](https://upload.wikimedia.org/wikipedia/commons/thumb/e/e8/Zeroth_stellation_of_icosahedron.png/240px-Zeroth_stellation_of_icosahedron.png)
pub fn create_icosphere(
    queue: Arc<Queue>,
    refine_levels: u32,
) -> Result<(Arc<IndexedMesh<PositionOnlyVertex, u16>>, impl GpuFuture), DeviceMemoryAllocError> {
    // macro to create and normalize `PositionOnlyVertex` in less code
    macro_rules! v {
        ($($points:expr),+) => {
            {
                let length = ($($points * $points+)+ 0.0).sqrt();
                let normalized = [$($points / length),+, 0.0];
                PositionOnlyVertex { position: normalized }
            }
        };
    }

    let phi = (1.0 + (5.0_f32.sqrt())) / 2.0;
    let mut vertex_data = vec![
        v!(-1.0, phi, 0.0),
        v!(1.0, phi, 0.0),
        v!(-1.0, -phi, 0.0),
        v!(1.0, -phi, 0.0),
        v!(0.0, -1.0, phi),
        v!(0.0, 1.0, phi),
        v!(0.0, -1.0, -phi),
        v!(0.0, 1.0, -phi),
        v!(phi, 0.0, -1.0),
        v!(phi, 0.0, 1.0),
        v!(-phi, 0.0, -1.0),
        v!(-phi, 0.0, 1.0),
    ];
    let mut index_data = vec![
        0u16, 11, 5, //
        0, 5, 1, //
        0, 1, 7, //
        0, 7, 10, //
        0, 10, 11, //
        1, 5, 9, //
        5, 11, 4, //
        11, 10, 2, //
        10, 7, 6, //
        7, 1, 8, //
        3, 9, 4, //
        3, 4, 2, //
        3, 2, 6, //
        3, 6, 8, //
        3, 8, 9, //
        4, 9, 5, //
        2, 4, 11, //
        6, 2, 10, //
        8, 6, 7, //
        9, 8, 1, //
    ];

    // refinements with cache to merge same vertices
    let mut cache: HashMap<u32, u16> = HashMap::new();
    let mut middle_point = |p1: u16, p2: u16| {
        let small = p1.min(p2) as u32;
        let big = p1.max(p2) as u32;
        let key = (small << 16) + big;

        match cache.entry(key) {
            Entry::Occupied(t) => *t.get(),
            Entry::Vacant(t) => {
                let v1 = vertex_data[p1 as usize].position;
                let v2 = vertex_data[p2 as usize].position;

                // compute middle point
                let mx = (v1[0] + v2[0]) / 2.0;
                let my = (v1[1] + v2[1]) / 2.0;
                let mz = (v1[2] + v2[2]) / 2.0;

                let index = vertex_data.len();
                vertex_data.push(v!(mx, my, mz));
                assert!(index < std::u16::MAX as usize);
                *t.insert(index as u16)
            }
        }
    };

    for _ in 0..refine_levels {
        let mut new_index_data = vec![];

        for triangle in index_data.chunks(3) {
            let v1 = triangle[0];
            let v2 = triangle[1];
            let v3 = triangle[2];

            let a = middle_point(v1, v2);
            let b = middle_point(v2, v3);
            let c = middle_point(v3, v1);

            // replace this face with 4 faces
            new_index_data.extend_from_slice(&[
                v1, a, c, //
                v2, b, a, //
                v3, c, b, //
                a, b, c,
            ])
        }

        index_data = new_index_data;
    }

    let (vertex_buffer, vbo_future) = ImmutableBuffer::from_iter(
        vertex_data.into_iter(),
        BufferUsage::vertex_buffer(),
        queue.clone(),
    )?;
    let (index_buffer, ibo_future) =
        ImmutableBuffer::from_iter(index_data.into_iter(), BufferUsage::index_buffer(), queue)?;

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

impl<V> From<IndexedMesh<V, u16>> for DynamicIndexedMesh<V>
where
    V: Vertex,
{
    fn from(idx: IndexedMesh<V, u16>) -> Self {
        DynamicIndexedMesh::U16(idx)
    }
}

impl<V> From<IndexedMesh<V, u32>> for DynamicIndexedMesh<V>
where
    V: Vertex,
{
    fn from(idx: IndexedMesh<V, u32>) -> Self {
        DynamicIndexedMesh::U32(idx)
    }
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
