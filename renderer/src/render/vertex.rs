use safe_transmute::TriviallyTransmutable;

#[derive(Default, Debug, Clone, Copy)]
pub struct PositionOnlyVertex {
    pub position: [f32; 3],
}

#[derive(Default, Debug, Clone, Copy)]
pub struct BasicVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

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
vulkano::impl_vertex!(PositionOnlyVertex, position);
