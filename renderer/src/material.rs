use crate::content::Result as ContentResult;
use crate::content::{Content, Load};
use crate::pod::MaterialData;
use cgmath::Vector3;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, PersistentDescriptorSetError};
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::ImmutableImage;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::sampler::Sampler;

/// On disk representation of Material.
#[derive(Serialize, Deserialize, Debug)]
pub struct MaterialDesc {
    albedo_color: Vector3<f32>,
    roughness: f32,
    metallic: f32,
    normal_map_strength: f32,
    albedo_map: Option<String>,
    normal_map: Option<String>,
    displacement_map: Option<String>,
    roughness_map: Option<String>,
    ao_map: Option<String>,
    metallic_map: Option<String>,
}

impl MaterialDesc {
    pub fn to_material(
        &self,
        content: &Content,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
        fallback: Arc<ImmutableImage<Format>>,
    ) -> Arc<Material> {
        let albedo = self
            .albedo_map
            .as_ref()
            .map(|x| content.load(x.as_str()))
            .map(|x| x.wait_for_then_unwrap());
        let normal = self
            .normal_map
            .as_ref()
            .map(|x| content.load(x.as_str()))
            .map(|x| x.wait_for_then_unwrap());
        let displacement = self
            .displacement_map
            .as_ref()
            .map(|x| content.load(x.as_str()))
            .map(|x| x.wait_for_then_unwrap());
        let roughness = self
            .roughness_map
            .as_ref()
            .map(|x| content.load(x.as_str()))
            .map(|x| x.wait_for_then_unwrap());
        let ao = self
            .ao_map
            .as_ref()
            .map(|x| content.load(x.as_str()))
            .map(|x| x.wait_for_then_unwrap());
        let metallic = self
            .metallic_map
            .as_ref()
            .map(|x| content.load(x.as_str()))
            .map(|x| x.wait_for_then_unwrap());

        Material::new(
            pipeline,
            sampler,
            fallback,
            albedo,
            normal,
            displacement,
            roughness,
            ao,
            metallic,
            MaterialData {
                albedo_color: self.albedo_color,
                alpha_cutoff: 0.0,
                roughness: self.roughness,
                metallic: self.metallic,
                normal_map_strength: self.normal_map_strength,
            },
        )
        .expect("cannot create Material instance")
    }
}

cache_storage_impl!(MaterialDesc);

impl Load for MaterialDesc {
    fn load(bytes: &[u8], _: Arc<Queue>) -> ContentResult<Self> {
        (
            Arc::new(serde_json::from_slice(bytes).expect("cannot read bytes as MaterialDesc")),
            None,
        )
    }
}

/// Runtime material representation. This struct is immutable. It is however possible
/// to change material's properties. This should not be done as it is costly.
pub struct Material {
    uniform_buffer: Arc<CpuAccessibleBuffer<MaterialData>>,
    // descriptor set that contains uniform objects that are related to this material instance
    pub(crate) descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
}

impl Material {
    pub fn new(
        geometry_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
        fb: Arc<ImmutableImage<Format>>,
        albedo: Option<Arc<ImmutableImage<Format>>>,
        normal: Option<Arc<ImmutableImage<Format>>>,
        displacement: Option<Arc<ImmutableImage<Format>>>,
        roughness: Option<Arc<ImmutableImage<Format>>>,
        ao: Option<Arc<ImmutableImage<Format>>>,
        metallic: Option<Arc<ImmutableImage<Format>>>,
        data: MaterialData,
    ) -> Result<Arc<Material>, PersistentDescriptorSetError> {
        let uniform_buffer = CpuAccessibleBuffer::from_data(
            geometry_pipeline.device().clone(),
            BufferUsage::uniform_buffer(),
            false,
            data,
        )
        .unwrap();

        let descriptor_set = Arc::new(
            PersistentDescriptorSet::start(
                geometry_pipeline.descriptor_set_layout(0).unwrap().clone(),
            )
            .add_sampled_image(albedo.unwrap_or_else(|| fb.clone()), sampler.clone())?
            .add_sampled_image(normal.unwrap_or_else(|| fb.clone()), sampler.clone())?
            .add_sampled_image(displacement.unwrap_or_else(|| fb.clone()), sampler.clone())?
            .add_sampled_image(roughness.unwrap_or_else(|| fb.clone()), sampler.clone())?
            .add_sampled_image(ao.unwrap_or_else(|| fb.clone()), sampler.clone())?
            .add_sampled_image(metallic.unwrap_or_else(|| fb.clone()), sampler.clone())?
            .add_buffer(uniform_buffer.clone())?
            .build()
            .expect("cannot build pds"),
        );

        Ok(Arc::new(Material {
            uniform_buffer,
            descriptor_set,
        }))
    }
}
