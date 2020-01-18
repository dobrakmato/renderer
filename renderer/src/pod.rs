use cgmath::{Matrix4, Vector3};

#[repr(C)]
pub struct MaterialData {
    pub albedo_color: Vector3<f32>,
    pub alpha_cutoff: f32,
}

#[repr(C)]
pub struct DirectionalLight {
    pub direction: Vector3<f32>,
    pub intensity: f32,
    pub color: Vector3<f32>,
}

#[repr(C)]
pub struct PointLight {
    pub position: Vector3<f32>,
    pub color: Vector3<f32>,
    pub intensity: f32,
}

#[repr(C)]
pub struct SpotLight {
    pub position: Vector3<f32>,
    pub angle: f32,
    pub color: Vector3<f32>,
    pub intensity: f32,
}

#[repr(C)]
pub struct MatrixData {
    pub model: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub projection: Matrix4<f32>,
}
