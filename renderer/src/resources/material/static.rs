//! Static material whose properties are determined at creation time.

use crate::assets::Content;
use crate::render::ubo::MaterialData;
use crate::resources::image::create_image;
use crate::resources::material::{FallbackMaps, Material, MATERIAL_UBO_DESCRIPTOR_SET};
use bf::material::BlendMode;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, ImmutableBuffer};
use vulkano::descriptor_set::DescriptorSet;
use vulkano::descriptor_set::{
    PersistentDescriptorSet, PersistentDescriptorSetBuildError, PersistentDescriptorSetError,
};
use vulkano::device::Queue;
use vulkano::image::view::ImageView;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::sampler::Sampler;
use vulkano::sync::GpuFuture;

/// Errors that may happen when creating a dynamic material.
#[derive(Debug)]
pub enum StaticMaterialError {
    /// Uniform Buffer couldn't be created because of allocation error.
    CannotCreateUniformBuffer(DeviceMemoryAllocError),
    /// Descriptor set has invalid number.
    InvalidDescriptorSetNumber,
    /// Persistent descriptor set could be created.
    CannotCreateDescriptorSet(PersistentDescriptorSetError),
    /// Persistent descriptor set could be built.
    CannotBuildDescriptorSet(PersistentDescriptorSetBuildError),
}

/// Static materials are unable to change their properties or
/// textures at run-time. Static materials should be used when
/// possible as they might be faster and more performant then dynamic.
pub struct StaticMaterial {
    blend_mode: BlendMode,
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
    ) -> Result<(Arc<Self>, impl GpuFuture), StaticMaterialError> {
        macro_rules! load_image_sync {
            ($map: expr, $def: expr) => {
                match &$map {
                    None => (&$def).clone(),
                    Some(uuid) => {
                        let guard = content.request_load(*uuid);
                        let image = guard.wait();
                        let (image, f) = create_image(&image, content.transfer_queue.clone())
                            .expect(&format!("cannot create image for: {}", uuid));

                        f.then_signal_fence_and_flush().ok();

                        ImageView::new(image).expect("cannot create view from image")
                    }
                }
            };
        }

        // create a uniform buffer with material data
        let data: MaterialData = (*material).into();
        let (buffer, future) =
            ImmutableBuffer::from_data(data, BufferUsage::uniform_buffer(), queue)
                .map_err(StaticMaterialError::CannotCreateUniformBuffer)?;

        // create a descriptor set layout from pipeline
        let layout = pipeline
            .layout()
            .descriptor_set_layouts()
            .get(MATERIAL_UBO_DESCRIPTOR_SET)
            .ok_or(StaticMaterialError::InvalidDescriptorSetNumber)?;

        // use loaded textures or fallbacks
        let albedo = load_image_sync!(material.albedo_map, fallback.fallback_white);
        let normal = load_image_sync!(material.normal_map, fallback.fallback_normal);
        let displacement = load_image_sync!(material.displacement_map, fallback.fallback_black);
        let roughness = load_image_sync!(material.roughness_map, fallback.fallback_white);
        let ao = load_image_sync!(material.ao_map, fallback.fallback_white);
        let metallic = load_image_sync!(material.metallic_map, fallback.fallback_black);
        let opacity = load_image_sync!(material.opacity_map, fallback.fallback_white);

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
            .add_sampled_image(metallic, sampler.clone())
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_buffer(buffer)
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_sampled_image(opacity, sampler)
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .build()
            .map_err(StaticMaterialError::CannotBuildDescriptorSet)?;

        Ok((
            Arc::new(Self {
                descriptor_set: Arc::new(set),
                blend_mode: material.blend_mode,
            }),
            future,
        ))
    }

    pub fn from_material_data(
        blend_mode: BlendMode,
        parameters: MaterialData,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
        queue: Arc<Queue>,
        fallback: Arc<FallbackMaps>,
    ) -> Result<(Arc<Self>, impl GpuFuture), StaticMaterialError> {
        // create a uniform buffer with material data
        let (buffer, future) =
            ImmutableBuffer::from_data(parameters, BufferUsage::uniform_buffer(), queue)
                .map_err(StaticMaterialError::CannotCreateUniformBuffer)?;

        // create a descriptor set layout from pipeline
        let layout = pipeline
            .layout()
            .descriptor_set_layouts()
            .get(MATERIAL_UBO_DESCRIPTOR_SET)
            .ok_or(StaticMaterialError::InvalidDescriptorSetNumber)?;

        // use loaded textures or fallbacks
        let albedo = fallback.fallback_white.clone();
        let normal = fallback.fallback_normal.clone();
        let displacement = fallback.fallback_black.clone();
        let roughness = fallback.fallback_white.clone();
        let ao = fallback.fallback_white.clone();
        let metallic = fallback.fallback_white.clone();
        let opacity = fallback.fallback_white.clone();

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
            .add_sampled_image(metallic, sampler.clone())
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_buffer(buffer)
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .add_sampled_image(opacity, sampler)
            .map_err(StaticMaterialError::CannotCreateDescriptorSet)?
            .build()
            .map_err(StaticMaterialError::CannotBuildDescriptorSet)?;

        Ok((
            Arc::new(Self {
                descriptor_set: Arc::new(set),
                blend_mode,
            }),
            future,
        ))
    }
}

impl Material for StaticMaterial {
    fn descriptor_set(&self) -> Arc<dyn DescriptorSet + Send + Sync> {
        self.descriptor_set.clone()
    }

    fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }
}
