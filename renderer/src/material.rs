use crate::content::Result as ContentResult;
use crate::content::{Content, Load};
use crate::pod::MaterialData;
use bf::load_bf_from_bytes;
use bf::uuid::Uuid;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, PersistentDescriptorSetError};
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::ImmutableImage;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::sampler::Sampler;

pub trait MaterialExt {
    fn to_material(
        &self,
        content: &Content,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
        fallback: Arc<ImmutableImage<Format>>,
    ) -> Arc<Material>;
}

impl MaterialExt for Arc<bf::material::Material> {
    fn to_material(
        &self,
        content: &Content,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
        fallback: Arc<ImmutableImage<Format>>,
    ) -> Arc<Material> {
        // helper function to load Image asset from Option<Uuid>
        let load = |opt: Option<Uuid>| {
            opt.as_ref()
                .map(|x| format!("{}.bf", x.to_hyphenated().to_string().to_lowercase()))
                .map(|x| content.load(x.as_str()))
                .map(|x| x.wait_for_then_unwrap())
        };

        let albedo = load(self.albedo_map);
        let normal = load(self.normal_map);
        let displacement = load(self.displacement_map);
        let roughness = load(self.roughness_map);
        let ao = load(self.ao_map);
        let metallic = load(self.metallic_map);

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
                alpha_cutoff: self.alpha_cutoff,
                roughness: self.roughness,
                metallic: self.metallic,
            },
        )
        .expect("cannot create Material instance")
    }
}

cache_storage_impl!(bf::material::Material);

impl Load for bf::material::Material {
    fn load(bytes: &[u8], _: Arc<Queue>) -> ContentResult<Self> {
        (
            Arc::new(
                load_bf_from_bytes(bytes)
                    .expect("cannot read bytes as bf::material::Material")
                    .try_to_material()
                    .expect("file is not bf::material::Material"),
            ),
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
