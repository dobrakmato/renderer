use cgmath::{Matrix4, Quaternion, Vector3};
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

pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Into<Matrix4<f32>> for Transform {
    fn into(self) -> Matrix4<f32> {
        let scale = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);
        let rotation = Matrix4::from(self.rotation);
        let translate = Matrix4::from_translation(self.position);

        translate * scale * rotation
    }
}

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
