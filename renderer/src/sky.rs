use crate::hosek::make_hosek_wilkie_params;
use crate::pod::HosekWilkieParams;
use cgmath::{vec3, Vector3};

pub struct SkyParams {
    pub albedo: Vector3<f32>,
    pub sun_dir: Vector3<f32>,
    pub turbidity: f32,
}

impl Default for SkyParams {
    fn default() -> Self {
        Self {
            albedo: vec3(0.0, 0.0, 0.0),
            sun_dir: vec3(0.0, 1.0, 0.0),
            turbidity: 2.0,
        }
    }
}

impl From<&SkyParams> for HosekWilkieParams {
    fn from(sp: &SkyParams) -> Self {
        make_hosek_wilkie_params(sp.sun_dir, sp.turbidity, sp.albedo)
    }
}
