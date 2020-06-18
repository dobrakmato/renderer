//! Temporary helper struct to allow rendering of meshes with materials.

use crate::render::transform::Transform;
use crate::render::ubo::ObjectMatrixData;
use crate::render::OBJECT_DATA_UBO_DESCRIPTOR_SET;
use crate::resources::material::Material;
use crate::resources::mesh::DynamicIndexedMesh;
use std::sync::{Arc, Mutex};
use vulkano::buffer::{BufferUsage, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::{
    FixedSizeDescriptorSetsPool, PersistentDescriptorSetBuildError, PersistentDescriptorSetError,
    UnsafeDescriptorSetLayout,
};
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Device;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::vertex::Vertex;
use vulkano::pipeline::GraphicsPipelineAbstract;

/// Struct that simplifies rendering of meshes with materials.
pub struct Object<V: Vertex> {
    pool: ObjectDataPool,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,

    /// Transform of this object.
    pub transform: Transform,
    /// Mesh that is currently being rendered.
    pub mesh: Arc<DynamicIndexedMesh<V>>,
    /// Material that is currently used for rendering.
    pub material: Arc<dyn Material>,
}

impl<V: Vertex> Object<V> {
    /// Creates a new `Object` from specified mesh, material. The device and pipeline
    /// parameters are needed to initialize internal object data pool.
    ///
    /// Once created, this object can only be used with the pipeline it was created with.
    pub fn new(
        mesh: Arc<DynamicIndexedMesh<V>>,
        material: Arc<dyn Material>,
        device: Arc<Device>,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        transform: Transform,
    ) -> Self {
        Self {
            pool: ObjectDataPool::new(
                device,
                pipeline
                    .descriptor_set_layout(OBJECT_DATA_UBO_DESCRIPTOR_SET)
                    .expect("cannot find descriptor set for ObjectData")
                    .clone(),
            ),
            transform,
            pipeline,
            mesh,
            material,
        }
    }

    /// Returns descriptor set that can be used for rendering in this frame. Returned
    /// `DescriptorSet` may or may not be cached from previous frame(s).
    fn object_matrix_data(&self) -> Result<impl DescriptorSet + Send + Sync, ObjectDataPoolError> {
        // todo: implement caching
        let data = self.transform.into();
        self.pool.next(data)
    }

    /// Records the draw command for rendering this object into the specified *CommandBufferBuilder*
    /// with specified dynamic state and frame matrix data descriptor set.
    ///
    /// Parameter `frame_matrix_data` should contain descriptor set that contains frame data
    /// for this rendering.
    pub fn draw_indexed(
        &self,
        dynamic_state: &DynamicState,
        frame_matrix_data: Arc<dyn DescriptorSet + Send + Sync>,
        cmd: &mut AutoCommandBufferBuilder,
    ) {
        let object_matrix_data = self
            .object_matrix_data()
            .expect("cannot create ObjectMatrixData for this frame");

        macro_rules! impl_dynamic_dispatch {
            ($($typ:ident),+) => {
                match self.mesh.as_ref() {
                    $(DynamicIndexedMesh::$typ(t) => {
                        cmd.draw_indexed(
                            self.pipeline.clone(),
                            dynamic_state,
                            vec![t.vertex_buffer().clone()],
                            t.index_buffer().clone(),
                            (
                                frame_matrix_data,
                                self.material.descriptor_set(),
                                object_matrix_data,
                            ),
                            (),
                        )
                        .expect("cannot DrawIndexed this mesh");
                    }),+
                }
            };
        }

        // dynamic dispatch based on dynamic mesh's index type
        impl_dynamic_dispatch!(U16, U32);
    }
}

/// Error that can happen while creating descriptor set using the `ObjectDataPool`.
#[derive(Debug)]
pub enum ObjectDataPoolError {
    /// Buffer for data for this frame couldn't be allocated.
    CannotAllocateBuffer(DeviceMemoryAllocError),
    /// Descriptor set could be created.
    CannotCreateDescriptorSet(PersistentDescriptorSetError),
    /// Descriptor set could be built.
    CannotBuildDescriptorSet(PersistentDescriptorSetBuildError),
}

/// Pool for descriptor sets that are used to render objects.
pub struct ObjectDataPool {
    buffer_pool: CpuBufferPool<ObjectMatrixData>,
    descriptor_set_pool: Mutex<FixedSizeDescriptorSetsPool>,
}

impl ObjectDataPool {
    /// Creates a new `ObjectDataPool` that contains pool for buffers and pool for descriptor sets.
    pub fn new(device: Arc<Device>, layout: Arc<UnsafeDescriptorSetLayout>) -> Self {
        Self {
            buffer_pool: CpuBufferPool::new(device, BufferUsage::uniform_buffer()),
            // todo: FixedSizeDescriptorSetsPool needs &mut reference to work internally
            descriptor_set_pool: Mutex::new(FixedSizeDescriptorSetsPool::new(layout)),
        }
    }

    /// Creates a new descriptor set that can be used with specified object data.
    pub fn next(
        &self,
        object_data: ObjectMatrixData,
    ) -> Result<impl DescriptorSet, ObjectDataPoolError> {
        let buffer = self
            .buffer_pool
            .next(object_data)
            .map_err(ObjectDataPoolError::CannotAllocateBuffer)?;

        Ok(self
            .descriptor_set_pool
            .lock()
            .unwrap()
            .next()
            .add_buffer(buffer)
            .map_err(ObjectDataPoolError::CannotCreateDescriptorSet)?
            .build()
            .map_err(ObjectDataPoolError::CannotBuildDescriptorSet)?)
    }
}
