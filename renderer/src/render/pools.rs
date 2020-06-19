//! Pools for rendering primitives.

use std::sync::{Arc, Mutex};
use vulkano::buffer::{BufferUsage, CpuBufferPool};
use vulkano::descriptor::descriptor_set::{
    FixedSizeDescriptorSetsPool, PersistentDescriptorSetBuildError, PersistentDescriptorSetError,
    UnsafeDescriptorSetLayout,
};
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Device;
use vulkano::memory::DeviceMemoryAllocError;

/// Error that can happen while creating descriptor set using the `ObjectDataPool`.
#[derive(Debug)]
pub enum UniformBufferPoolError {
    /// Buffer for data for this frame couldn't be allocated.
    CannotAllocateBuffer(DeviceMemoryAllocError),
    /// Descriptor set could be created.
    CannotCreateDescriptorSet(PersistentDescriptorSetError),
    /// Descriptor set could be built.
    CannotBuildDescriptorSet(PersistentDescriptorSetBuildError),
}

/// Pool for descriptor sets that are used to render objects.
pub struct UniformBufferPool<T> {
    buffer_pool: CpuBufferPool<T>,
    descriptor_set_pool: Mutex<FixedSizeDescriptorSetsPool>,
}

impl<T> UniformBufferPool<T> {
    /// Creates a new `UniformBufferPool` that contains pool for buffers
    /// and pool for descriptor sets.
    pub fn new(device: Arc<Device>, layout: Arc<UnsafeDescriptorSetLayout>) -> Self {
        Self {
            buffer_pool: CpuBufferPool::new(device, BufferUsage::uniform_buffer()),
            // todo: FixedSizeDescriptorSetsPool needs &mut reference to work internally
            descriptor_set_pool: Mutex::new(FixedSizeDescriptorSetsPool::new(layout)),
        }
    }

    /// Creates a new descriptor set that can be used with specified data.
    pub fn next(&self, data: T) -> Result<impl DescriptorSet, UniformBufferPoolError> {
        let buffer = self
            .buffer_pool
            .next(data)
            .map_err(UniformBufferPoolError::CannotAllocateBuffer)?;

        Ok(self
            .descriptor_set_pool
            .lock()
            .unwrap()
            .next()
            .add_buffer(buffer)
            .map_err(UniformBufferPoolError::CannotCreateDescriptorSet)?
            .build()
            .map_err(UniformBufferPoolError::CannotBuildDescriptorSet)?)
    }
}
