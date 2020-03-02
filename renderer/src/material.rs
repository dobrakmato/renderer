use crate::content::Result;
use crate::content::{Content, Load};
use crate::pod::MaterialData;
use cgmath::Vector3;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
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
    albedo_map: Option<String>,
    normal_map: Option<String>,
}

impl MaterialDesc {
    pub fn to_material(
        &self,
        content: &Content,
        pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        sampler: Arc<Sampler>,
    ) -> Arc<Material> {
        let albedo = content
            .load(self.albedo_map.as_ref().unwrap().as_str())
            .wait_for_then_unwrap();
        Material::new(
            pipeline,
            sampler,
            albedo,
            MaterialData {
                albedo_color: self.albedo_color,
                alpha_cutoff: 0.0,
            },
        )
    }
}

cache_storage_impl!(MaterialDesc);

impl Load for MaterialDesc {
    fn load(bytes: &[u8], _: Arc<Queue>) -> Result<Self> {
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
        albedo: Arc<ImmutableImage<Format>>,
        data: MaterialData,
    ) -> Arc<Material> {
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
            .add_sampled_image(albedo, sampler)
            .unwrap()
            .add_buffer(uniform_buffer.clone())
            .unwrap()
            .build()
            .expect("cannot build pds"),
        );

        Arc::new(Material {
            uniform_buffer,
            descriptor_set,
        })
    }
}
