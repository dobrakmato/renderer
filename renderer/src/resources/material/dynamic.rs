//! Dynamic material that can change its properties in each frame.

use crate::render::ubo::MaterialData;
use bf::uuid::Uuid;
use std::sync::{Arc, Mutex};
use vulkano::buffer::{BufferUsage, CpuBufferPool};
use vulkano::descriptor::descriptor_set::{
    FixedSizeDescriptorSetsPool, PersistentDescriptorSetBuildError, PersistentDescriptorSetError,
};
use vulkano::descriptor::DescriptorSet;

use crate::assets::Storage;
use crate::resources::image::create_image;
use crate::resources::material::{FallbackMaps, Material, MATERIAL_UBO_DESCRIPTOR_SET};
use vulkano::format::Format;
use vulkano::image::ImmutableImage;
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::sampler::Sampler;

/// Errors that may happen when creating a dynamic material.
#[derive(Debug)]
pub enum DynamicMaterialError {
    /// Uniform Buffer couldn't be created because of allocation error.
    CannotCreateUniformBuffer(DeviceMemoryAllocError),
    /// Descriptor set has invalid number.
    InvalidDescriptorSetNumber,
    /// Persistent descriptor set could be created.
    CannotCreateDescriptorSet(PersistentDescriptorSetError),
    /// Persistent descriptor set could be built.
    CannotBuildDescriptorSet(PersistentDescriptorSetBuildError),
}

/// Dynamic materials can change their properties and textures
/// at run-time. Static materials should be used when
/// possible as they might be faster and more performant then dynamic.
///
/// You can change properties of this material at any time. However
/// the changes will be reflected in the next frame as `DescriptorSet`
/// for dynamic materials is rebuild on each frame.
pub struct DynamicMaterial {
    uniform_buffer_pool: CpuBufferPool<MaterialData>,
    descriptor_set_pool: Mutex<FixedSizeDescriptorSetsPool>,
    // todo: needs &mut reference to work internally
    pub fallback: Arc<FallbackMaps>,
    pub sampler: Arc<Sampler>,
    pub data: MaterialData,
    pub albedo_map: Option<Arc<ImmutableImage<Format>>>,
    pub normal_map: Option<Arc<ImmutableImage<Format>>>,
    pub displacement_map: Option<Arc<ImmutableImage<Format>>>,
    pub roughness_map: Option<Arc<ImmutableImage<Format>>>,
    pub ao_map: Option<Arc<ImmutableImage<Format>>>,
    pub metallic_map: Option<Arc<ImmutableImage<Format>>>,
    pub opacity_map: Option<Arc<ImmutableImage<Format>>>,
}

impl DynamicMaterial {
    pub fn from_material(
        material: &bf::material::Material,
        assets: &Storage,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
        fallback: Arc<FallbackMaps>,
    ) -> Result<Arc<Self>, DynamicMaterialError> {
        // helper function to load Image asset from Option<Uuid>
        let load = |opt: Option<Uuid>| opt.map(|x| assets.request_load(x));

        // request to load all maps
        let albedo_map = load(material.albedo_map);
        let normal_map = load(material.normal_map);
        let displacement_map = load(material.displacement_map);
        let roughness_map = load(material.roughness_map);
        let ao_map = load(material.ao_map);
        let metallic_map = load(material.metallic_map);
        let opacity_map = load(material.opacity_map);

        let create = |opt: Option<Arc<bf::image::Image>>| {
            opt.map(|x| create_image(&x, assets.transfer_queue.clone()).unwrap().0)
        };

        let albedo_map = create(albedo_map.map(|x| x.wait()));
        let normal_map = create(normal_map.map(|x| x.wait()));
        let displacement_map = create(displacement_map.map(|x| x.wait()));
        let roughness_map = create(roughness_map.map(|x| x.wait()));
        let ao_map = create(ao_map.map(|x| x.wait()));
        let metallic_map = create(metallic_map.map(|x| x.wait()));
        let opacity_map = create(opacity_map.map(|x| x.wait()));

        // create a descriptor set layout from pipeline
        let layout = pipeline
            .descriptor_set_layout(MATERIAL_UBO_DESCRIPTOR_SET)
            .ok_or(DynamicMaterialError::InvalidDescriptorSetNumber)?;

        Ok(Arc::new(DynamicMaterial {
            albedo_map,
            normal_map,
            displacement_map,
            roughness_map,
            ao_map,
            metallic_map,
            opacity_map,
            sampler,
            fallback,
            data: (*material).into(),
            uniform_buffer_pool: CpuBufferPool::new(
                pipeline.device().clone(),
                BufferUsage::uniform_buffer(),
            ),
            descriptor_set_pool: Mutex::new(FixedSizeDescriptorSetsPool::new(layout.clone())),
        }))
    }
}

impl Material for DynamicMaterial {
    /// This function panics when the descriptor set for this
    /// dynamic material cloud not be created.
    fn descriptor_set(&self) -> Arc<dyn DescriptorSet + Send + Sync> {
        fn internal(
            mat: &DynamicMaterial,
        ) -> Result<Arc<dyn DescriptorSet + Send + Sync>, DynamicMaterialError> {
            // use loaded textures or fallbacks
            let albedo = mat.fallback.white(&mat.albedo_map);
            let normal = mat.fallback.normal(&mat.normal_map);
            let displacement = mat.fallback.black(&mat.roughness_map);
            let roughness = mat.fallback.white(&mat.roughness_map);
            let ao = mat.fallback.white(&mat.ao_map);
            let metallic = mat.fallback.black(&mat.metallic_map);
            let opacity = mat.fallback.white(&mat.opacity_map);

            // create a uniform buffer for this frame
            let buffer = mat
                .uniform_buffer_pool
                .next(mat.data)
                .map_err(DynamicMaterialError::CannotCreateUniformBuffer)?;

            // create a descriptor set for this frame
            let descriptor_set = mat
                .descriptor_set_pool
                .lock()
                .unwrap()
                .next()
                .add_sampled_image(albedo, mat.sampler.clone())
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .add_sampled_image(normal, mat.sampler.clone())
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .add_sampled_image(displacement, mat.sampler.clone())
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .add_sampled_image(roughness, mat.sampler.clone())
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .add_sampled_image(ao, mat.sampler.clone())
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .add_sampled_image(metallic, mat.sampler.clone())
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .add_buffer(buffer)
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .add_sampled_image(opacity, mat.sampler.clone())
                .map_err(DynamicMaterialError::CannotCreateDescriptorSet)?
                .build()
                .map_err(DynamicMaterialError::CannotBuildDescriptorSet)?;

            Ok(Arc::new(descriptor_set))
        };

        internal(&self)
            .map_err(|e| {
                panic!(
                    "creating descriptor set for dynamic material failed: {:?}",
                    e
                )
            })
            .unwrap()
    }
}
