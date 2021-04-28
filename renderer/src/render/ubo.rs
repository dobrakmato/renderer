//! Structs for data passed to shaders via *Uniform Buffer Objects* and other mechanisms.

use cgmath::{Matrix4, Vector3};
use core::assert_alignment;

// todo: remove and use from shader! generated
/// UBO struct with data about PBR material that is currently being
/// used.
#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct MaterialData {
    /// Albedo PBR color.
    pub albedo_color: [f32; 3],
    /// Alpha cutoff if using `Masked` blend mode.
    pub alpha_cutoff: f32,
    /// Roughness PBR parameter.
    pub roughness: f32,
    /// Metallic PBR parameters.
    pub metallic: f32,
}

/// UBO struct with data that us uniform for every shader during
/// one frame (such us view matrix, ...).
#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct FrameMatrixData {
    /// View matrix.
    pub view: Matrix4<f32>,
    /// Projection matrix.
    pub projection: Matrix4<f32>,
    /// Inverse of view matrix.
    pub inv_projection: Matrix4<f32>,
    /// Inverse of projection matrix.
    pub inv_view: Matrix4<f32>,
    /// Camera position in world-space.
    pub camera_position: Vector3<f32>,
}

/// UBO struct representing an uniform buffer that contains data
/// related to currently rendered object (such as model matrix).
#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct ObjectMatrixData {
    /// Model matrix for currently renderer object.
    pub model: Matrix4<f32>,
}

/// UBO struct representing a directional light (light which
/// rays are parallel) and its properties.
#[derive(Copy, Clone)]
#[repr(C, align(16))]
pub struct DirectionalLight {
    /// Direction of the light (from the shaded pixel to the light source).
    pub direction: Vector3<f32>,
    /// Intensity of the light.
    pub intensity: f32,
    /// Color of the light.
    pub color: Vector3<f32>,
}

assert_alignment!(MaterialData, 16);
assert_alignment!(FrameMatrixData, 16);
assert_alignment!(ObjectMatrixData, 16);
assert_alignment!(DirectionalLight, 16);
