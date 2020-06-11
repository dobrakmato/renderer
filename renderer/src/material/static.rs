use crate::content::Content;
use crate::material::{FallbackMaps, Material, MATERIAL_UBO_DESCRIPTOR_SET};
use crate::pod::MaterialData;
use bf::uuid::Uuid;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, ImmutableBuffer};
use vulkano::descriptor::descriptor_set::{
    PersistentDescriptorSet, PersistentDescriptorSetBuildError, PersistentDescriptorSetError,
};
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Queue;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::sampler::Sampler;

#[derive(Debug)]
pub enum StaticMaterialError {
    CannotCreateUniformBuffer(DeviceMemoryAllocError),
    InvalidDescriptorSetNumber,
    CannotCreateDescriptorSet(PersistentDescriptorSetError),
    CannotBuildDescriptorSet(PersistentDescriptorSetBuildError),
}

/// Static materials are unable to change their properties or
/// textures at run-time. Static materials should be used when
/// possible as they might be faster and more performant then dynamic.
pub struct StaticMaterial {
    descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
}

impl StaticMaterial {
    pub fn from_material(
        material: &bf::material::Material,
        content: &Content,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
        queue: Arc<Queue>,
        fallback: Arc<FallbackMaps>,
    ) -> Result<Arc<Self>, StaticMaterialError> {
        // helper function to load Image asset from Option<Uuid>
        let load = |opt: Option<Uuid>| {
            opt.map(|x| content.load_uuid(x))
                .map(|x| x.wait_for_then_unwrap())
        };

        // request to load all maps
        let albedo = load(material.albedo_map);
        let normal = load(material.normal_map);
        let displacement = load(material.displacement_map);
        let roughness = load(material.roughness_map);
        let ao = load(material.ao_map);
        let metallic = load(material.metallic_map);

        // create a uniform buffer with material data
        let data: MaterialData = (*material).into();
        let (buffer, future) =
            ImmutableBuffer::from_data(data, BufferUsage::uniform_buffer(), queue)
                .map_err(StaticMaterialError::CannotCreateUniformBuffer)?;

        // create a descriptor set layout from pipeline
        let layout = pipeline
            .descriptor_set_layout(MATERIAL_UBO_DESCRIPTOR_SET)
            .ok_or(StaticMaterialError::InvalidDescriptorSetNumber)?;

        // use loaded textures or fallbacks
        let albedo = fallback.white(&albedo);
        let normal = fallback.normal(&normal);
        let displacement = fallback.black(&displacement);
        let roughness = fallback.white(&roughness);
        let ao = fallback.white(&ao);
        let metallic = fallback.black(&metallic);

        // create descriptor set
        let set = PersistentDescriptorSet::start(layout.clone())
            .add_sampled_image(albedo, sampler.clone())
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_sampled_image(normal, sampler.clone())
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_sampled_image(displacement, sampler.clone())
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_sampled_image(roughness, sampler.clone())
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_sampled_image(ao, sampler.clone())
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_sampled_image(metallic, sampler)
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_buffer(buffer)
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .build()
            .map_err(StaticMaterialError::CannotBuildDescriptorSet)?;

        Ok(Arc::new(Self {
            descriptor_set: Arc::new(set),
        }))
    }
}

impl Material for StaticMaterial {
    fn descriptor_set(&self) -> Arc<dyn DescriptorSet + Send + Sync> {
        self.descriptor_set.clone()
    }
}
