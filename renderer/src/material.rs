use crate::pod::MaterialData;
use cgmath::Vector3;
use std::sync::Arc;
use vulkano::buffer::ImmutableBuffer;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::format::Format;
use vulkano::image::ImmutableImage;

// definition
pub struct Material {
    albedo_map: Arc<ImmutableImage<Format>>, // todo: resource<ImmutableImage>
    albedo_color: Vector3<f32>,
    alpha_cutoff: f32,
}

impl Material {
    pub fn to_material_data(&self) -> MaterialData {
        MaterialData {
            albedo_color: self.albedo_color,
            alpha_cutoff: self.alpha_cutoff,
        }
    }
}

pub struct MaterialInstance<L, R> {
    descriptor_set: PersistentDescriptorSet<L, R>,
    material_data: ImmutableBuffer<MaterialData>,
}
