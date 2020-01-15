use safe_transmute::TriviallyTransmutable;

#[derive(Default, Debug, Clone, Copy)]
pub struct BasicVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

unsafe impl TriviallyTransmutable for BasicVertex {}

vulkano::impl_vertex!(BasicVertex, position, normal, uv);

trait Pass<VDef, VSkinnedDef> {}

// kazdy subpass ma svoj secondary command buffer
// secondary sa potom joinu do primary v render pass

enum SubPass {
    Cube,
    Finished,
}

// render graph
// - kazdy node vytvori secondary command buffer
// - ked sa spajaju tak sa join!
// - ked sa rozdeluju tak idu na rozne queue
