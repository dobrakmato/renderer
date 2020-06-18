use cgmath::{vec3, InnerSpace, Matrix4, Vector3};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct MaterialData {
    pub albedo_color: [f32; 3],
    pub alpha_cutoff: f32,
    pub roughness: f32,
    pub metallic: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DirectionalLight {
    pub direction: Vector3<f32>,
    pub intensity: f32,
    pub color: Vector3<f32>,
    pub _dummy0: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: vec3(1.0, 1.0, 1.0).normalize(),
            intensity: 1.0,
            color: vec3(1.0, 1.0, 1.0),
            _dummy0: 0.0,
        }
    }
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

/// UBO struct with data that us uniform for every shader during
/// one frame (such us view matrix, ...).
#[repr(C)]
pub struct FrameMatrixData {
    /// View matrix.
    pub view: Matrix4<f32>,
    /// Projection matrix.
    pub projection: Matrix4<f32>,
    /// Inverse of view matrix.
    pub inv_projection: Matrix4<f32>,
    /// Inverse of projection matrix.
    pub inv_view: Matrix4<f32>,
}

/// Struct representing an uniform buffer that
#[repr(C)]
pub struct ObjectMatrixData {
    pub model: Matrix4<f32>,
}

#[repr(C)]
pub struct HosekWilkieParams {
    pub a: Vector3<f32>,
    pub padding0: f32,
    pub b: Vector3<f32>,
    pub padding1: f32,
    pub c: Vector3<f32>,
    pub padding2: f32,
    pub d: Vector3<f32>,
    pub padding3: f32,
    pub e: Vector3<f32>,
    pub padding4: f32,
    pub f: Vector3<f32>,
    pub padding5: f32,
    pub g: Vector3<f32>,
    pub padding6: f32,
    pub h: Vector3<f32>,
    pub padding7: f32,
    pub i: Vector3<f32>,
    pub padding8: f32,
    pub z: Vector3<f32>,
    pub padding9: f32,
    pub sun_direction: Vector3<f32>,
}
