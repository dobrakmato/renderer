//! Static & dynamic materials.

use crate::render::ubo::MaterialData;
use std::sync::Arc;
use vulkano::format::Format;
use vulkano::image::ImmutableImage;

mod dynamic;
mod r#static;

use crate::resources::image::create_single_pixel_image;
pub use dynamic::DynamicMaterial;
pub use r#static::StaticMaterial;
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Queue;
use vulkano::image::view::ImageView;
use vulkano::sync::GpuFuture;

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

/// Struct containing the default fallback maps (images) that should be
/// used when shading a material that doesn't have some maps.
///
/// This struct has several methods. Each of them works the same way.
/// The function accepts an `Option` of `Arc<ImmutableImage>`.
/// - If the option is `Some`, this function returns cloned `Arc` of the passed in reference.
/// - If the option is `None`, this function returns cloned `Arc` of fallback texture.
pub struct FallbackMaps {
    /// Fallback texture that is white (255, 255, 255).
    pub fallback_white: Arc<ImageView<Arc<ImmutableImage<Format>>>>,
    /// Fallback texture that is black (0, 0, 0).
    pub fallback_black: Arc<ImageView<Arc<ImmutableImage<Format>>>>,
    /// Fallback texture that is flat tangent space normal map (128, 128, 255).
    pub fallback_normal: Arc<ImageView<Arc<ImmutableImage<Format>>>>,
}

macro_rules! fallback_fn {
    ($name: ident, $field: ident) => {
        /// See [`FallbackMaps`](struct.FallbackMaps.html) docs for more information on usage.
        #[inline]
        pub fn $name(
            &self,
            expected: &Option<Arc<ImageView<Arc<ImmutableImage<Format>>>>>,
        ) -> Arc<ImageView<Arc<ImmutableImage<Format>>>> {
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

/// Creates a [`FallbackMaps`](struct.FallbackMaps.html) struct with default
/// maps. Returns the struct and future that represents the moment maps are
/// ready to be used.
pub fn create_default_fallback_maps(queue: Arc<Queue>) -> (Arc<FallbackMaps>, impl GpuFuture) {
    let (white, f1) = create_single_pixel_image(queue.clone(), [255; 4]).unwrap();
    let (black, f2) = create_single_pixel_image(queue.clone(), [0; 4]).unwrap();
    let (normal, f3) = create_single_pixel_image(queue, [0, 128, 0, 128]).unwrap(); // normal map is in packed representation

    (
        Arc::new(FallbackMaps {
            fallback_white: ImageView::new(white).ok().unwrap(),
            fallback_black: ImageView::new(black).ok().unwrap(),
            fallback_normal: ImageView::new(normal).ok().unwrap(),
        }),
        f1.join(f2).join(f3),
    )
}
