use safe_transmute::TriviallyTransmutable;

#[derive(Default, Debug, Clone, Copy)]
pub struct BasicVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Default, Debug, Clone, Copy)]
pub struct PositionOnlyVertex {
    pub position: [f32; 3],
}

unsafe impl TriviallyTransmutable for BasicVertex {}
unsafe impl TriviallyTransmutable for PositionOnlyVertex {}

vulkano::impl_vertex!(BasicVertex, position, normal, uv);
vulkano::impl_vertex!(PositionOnlyVertex, position);

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
