use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlendMode {
    /// Suitable for normal solid objects with no transparent areas.
    Opaque,
    /// Allows you to create a transparent effect that has hard edges between the opaque and
    /// transparent areas. In this mode, there are no semi-transparent areas, the texture is
    /// either 100% opaque, or invisible.
    Masked,
}

/// Material is a descriptive asset that contains some properties and links to other assets (maps).
#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub blend_mode: BlendMode,

    pub albedo_color: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,

    // if using masked shading mode we need to store alpha_cutoff
    pub alpha_cutoff: f32,

    pub albedo_map: Option<Uuid>,
    pub normal_map: Option<Uuid>,
    pub displacement_map: Option<Uuid>,
    pub roughness_map: Option<Uuid>,
    pub ao_map: Option<Uuid>,
    pub metallic_map: Option<Uuid>,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            blend_mode: BlendMode::Opaque,
            albedo_color: [86.0 / 255.0, 93.0 / 255.0, 110.0 / 255.0],
            roughness: 0.5,
            metallic: 0.0,
            alpha_cutoff: 0.0,
            albedo_map: None,
            normal_map: None,
            displacement_map: None,
            roughness_map: None,
            ao_map: None,
            metallic_map: None,
        }
    }
}
