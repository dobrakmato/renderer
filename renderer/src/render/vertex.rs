//! Declaration of different `Vertex` types.

use safe_transmute::TriviallyTransmutable;

/// Vertex that consists only of *position*.
///
/// Layout of this vertex is following:
///
/// | f32_0      | f32_1      | f32_2      | f32_3     |
/// |------------|------------|------------|-----------|
/// | position.x | position.y | position.z |*(padding)*|
#[derive(Default, Debug, Clone, Copy)]
pub struct PositionOnlyVertex {
    pub position: [f32; 4],
}

/// Vertex that consists of *position*, *normal* and one *uv coordinate*.
///
/// Layout of this vertex is following:
///
/// | f32_0      | f32_1      | f32_2      | f32_3     |
/// |------------|------------|------------|-----------|
/// | position.x | position.y | position.z | normal.x  |
/// | normal.y   | normal.z   | uv.x       | uv.y      |
#[derive(Default, Debug, Clone, Copy)]
pub struct BasicVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// Vertex that consists of *position*, *normal*, one *uv coordinate* and *tangent*.
///
/// Layout of this vertex is following:
///
/// | f32_0      | f32_1      | f32_2      | f32_3     |
/// |------------|------------|------------|-----------|
/// | position.x | position.y | position.z | normal.x  |
/// | normal.y   | normal.z   | uv.x       | uv.y      |
/// | tangent.x  | tangent.y  | tangent.z  |*(padding)*|
///
#[derive(Default, Debug, Clone, Copy)]
pub struct NormalMappedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 4],
}

unsafe impl TriviallyTransmutable for PositionOnlyVertex {}

unsafe impl TriviallyTransmutable for BasicVertex {}

unsafe impl TriviallyTransmutable for NormalMappedVertex {}

vulkano::impl_vertex!(NormalMappedVertex, position, normal, uv, tangent);
vulkano::impl_vertex!(BasicVertex, position, normal, uv);
vulkano::impl_vertex!(PositionOnlyVertex, position);
