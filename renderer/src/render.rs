use safe_transmute::TriviallyTransmutable;

#[derive(Default, Debug, Clone, Copy)]
pub struct BasicVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

unsafe impl TriviallyTransmutable for BasicVertex {}

vulkano::impl_vertex!(BasicVertex, position, normal, uv);

struct Frame {}

impl Frame {
    fn draw() {}
}

enum Pass {
    Shadows,
    Skybox,
    GBuffer,
    IndirectLighting,
    DirectLighting,
    Particles,
    Composite,
    PostProcessing,
    UI,
    Finished,
}
