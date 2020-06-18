use crate::pod::MaterialData;
use std::sync::Arc;
use vulkano::format::Format;
use vulkano::image::ImmutableImage;

mod dynamic;
mod r#static;

pub use dynamic::DynamicMaterial;
pub use r#static::StaticMaterial;
use vulkano::descriptor::DescriptorSet;

/// Index of descriptor set that is used for material data.
pub const MATERIAL_UBO_DESCRIPTOR_SET: usize = 1;

/// Trait that represents an object that can be used as a material
/// in rendering process.
pub trait Material {
    /// Returns a descriptor set that will be used for rendering
    /// during this frame.
    fn descriptor_set(&self) -> Arc<dyn DescriptorSet + Send + Sync>;
}

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

macro_rules! fallback_fn {
    ($name: ident, $field: ident) => {
        #[inline]
        pub fn $name(
            &self,
            expected: &Option<Arc<ImmutableImage<Format>>>,
        ) -> Arc<ImmutableImage<Format>> {
            expected
                .as_ref()
                .cloned()
                .unwrap_or_else(|| self.$field.clone())
        }
    };
}

impl FallbackMaps {
    fallback_fn!(white, fallback_white);
    fallback_fn!(black, fallback_black);
    fallback_fn!(normal, fallback_normal);
}
