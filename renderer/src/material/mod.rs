use crate::pod::MaterialData;
use std::sync::Arc;
use vulkano::format::Format;
use vulkano::image::ImmutableImage;

mod dynamic;
mod r#static;

pub use dynamic::DynamicMaterial;
pub use r#static::StaticMaterial;
use vulkano::descriptor::DescriptorSet;

/// Trait that represents an object that can be used as a material
/// in rendering process.
pub trait Material {
    fn descriptor_set(&self) -> Arc<dyn DescriptorSet + Send + Sync>;
}

/// Index of descriptor set that is used for material data.
pub const MATERIAL_UBO_DESCRIPTOR_SET: usize = 0;

impl Into<MaterialData> for bf::material::Material {
    fn into(self) -> MaterialData {
        MaterialData {
            albedo_color: self.albedo_color,
            alpha_cutoff: self.alpha_cutoff,
            roughness: self.roughness,
            metallic: self.metallic,
        }
    }
}

/// Struct containing the default fallback maps (images) that will be
/// used when sampling a material that dont have some maps.
pub struct FallbackMaps {
    pub fallback_white: Arc<ImmutableImage<Format>>,
    pub fallback_black: Arc<ImmutableImage<Format>>,
    pub fallback_normal: Arc<ImmutableImage<Format>>,
}

impl FallbackMaps {
    #[inline]
    pub fn black(
        &self,
        expected: &Option<Arc<ImmutableImage<Format>>>,
    ) -> Arc<ImmutableImage<Format>> {
        expected
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.fallback_white.clone())
    }

    #[inline]
    pub fn white(
        &self,
        expected: &Option<Arc<ImmutableImage<Format>>>,
    ) -> Arc<ImmutableImage<Format>> {
        expected
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.fallback_white.clone())
    }

    #[inline]
    pub fn normal(
        &self,
        expected: &Option<Arc<ImmutableImage<Format>>>,
    ) -> Arc<ImmutableImage<Format>> {
        expected
            .as_ref()
            .cloned()
            .unwrap_or_else(|| self.fallback_normal.clone())
    }
}
