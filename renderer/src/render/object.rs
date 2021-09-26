//! Temporary helper struct to allow rendering of meshes with materials.

use crate::render::pools::{UniformBufferPool, UniformBufferPoolError};
use crate::render::transform::Transform;
use crate::render::ubo::ObjectMatrixData;
use crate::render::{descriptor_set_layout, OBJECT_DATA_UBO_DESCRIPTOR_SET};
use crate::resources::material::Material;
use crate::resources::mesh::DynamicIndexedMesh;
use std::sync::Arc;
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Device;
use vulkano::pipeline::vertex::Vertex;
use vulkano::pipeline::GraphicsPipelineAbstract;

/// Uniform buffer pool for object data.
pub type ObjectDataPool = UniformBufferPool<ObjectMatrixData>;

/// Struct that simplifies rendering of meshes with materials.
pub struct Object<V: Vertex> {
    pool: ObjectDataPool,

    /// Pipeline that is used for this object.
    pub pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
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
                descriptor_set_layout(pipeline.layout(), OBJECT_DATA_UBO_DESCRIPTOR_SET),
            ),
            transform,
            pipeline,
            mesh,
            material,
        }
    }

    /// Returns descriptor set that can be used for rendering in this frame. Returned
    /// `DescriptorSet` may or may not be cached from previous frame(s).
    pub fn object_matrix_data(
        &self,
    ) -> Result<impl DescriptorSet + Send + Sync, UniformBufferPoolError> {
        // todo: implement caching
        let data = self.transform.into();
        self.pool.next(data)
    }
}
