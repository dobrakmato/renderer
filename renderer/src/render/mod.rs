//! Objects & procedures related to rendering.

use crate::camera::Camera;
use crate::render::pbr::PBRDeffered;
use crate::render::pools::UniformBufferPool;
use crate::render::ubo::{DirectionalLight, FrameMatrixData};
use crate::GameState;
use cgmath::{EuclideanSpace, SquareMatrix, Vector3, Zero};
use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::UnsafeDescriptorSetLayout;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::format::ClearValue;
use vulkano::framebuffer::FramebufferAbstract;
use vulkano::image::SwapchainImage;
use vulkano::pipeline::viewport::Viewport;
use winit::window::Window;

// consts to descriptor set binding indices
pub const FRAME_DATA_UBO_DESCRIPTOR_SET: usize = 0;
pub const OBJECT_DATA_UBO_DESCRIPTOR_SET: usize = 2;
pub const SUBPASS_UBO_DESCRIPTOR_SET: usize = 1;
pub const LIGHTS_UBO_DESCRIPTOR_SET: usize = 2;

pub mod hosek;
pub mod object;
pub mod pbr;
pub mod pools;
pub mod renderer;
pub mod samplers;
mod shaders;
pub mod transform;
pub mod ubo;
pub mod vertex;
pub mod vulkan;

pub type FrameMatrixPool = UniformBufferPool<FrameMatrixData>;

/// Series of operations related to lighting and shading.
pub trait RenderPath {
    fn new(graphical_queue: Arc<Queue>, device: Arc<Device>) -> Box<Self>;
    /// Creates a *Framebuffer* with given `final_image` as final render target.
    fn create_framebuffer(&self, final_image: Arc<SwapchainImage<Window>>);
    /// Recreates internal state & buffers to support the new resolution.
    fn recreate_buffers(&self, new_dimensions: [u32; 2]);
}

/// Helper function to retrieve `UnsafeDescriptorSetLayout` from pipeline
/// by specifying the index of the layout.
///
/// # Panics
///
/// This function panics if `index` is invalid set index for provided pipeline.
///
pub fn descriptor_set_layout<T>(pipeline: &T, index: usize) -> Arc<UnsafeDescriptorSetLayout>
where
    T: PipelineLayoutAbstract,
{
    pipeline
        .descriptor_set_layout(index)
        .expect("cannot get descriptor set layout")
        .clone()
}

pub struct Frame<'r, 's> {
    render_path: &'r mut PBRDeffered,
    game_state: &'s GameState,
    framebuffer: Arc<dyn FramebufferAbstract + Send + Sync>,
    builder: Option<AutoCommandBufferBuilder>,
}

impl<'r, 's> Frame<'r, 's> {
    pub fn build(&mut self) -> AutoCommandBuffer {
        let dims = [
            self.framebuffer.dimensions()[0] as f32,
            self.framebuffer.dimensions()[1] as f32,
        ];
        let dynamic_state = DynamicState {
            viewports: Some(vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [dims[0] as f32, dims[1] as f32],
                depth_range: 0.0..1.0,
            }]),
            ..DynamicState::none()
        };
        let path = &mut self.render_path;
        let state = self.game_state;

        /* create FrameMatrixData (set=2) for this frame. */
        let view = self.game_state.camera.view_matrix();
        let projection = self.game_state.camera.projection_matrix();
        let fmd = FrameMatrixData {
            camera_position: self.game_state.camera.position.to_vec(),
            inv_view: view.invert().unwrap(),
            inv_projection: projection.invert().unwrap(),
            view,
            projection,
        };
        let geometry_frame_matrix_data = Arc::new(
            path.buffers
                .geometry_frame_matrix_pool
                .next(fmd)
                .expect("cannot take next buffer"),
        );
        let lights_frame_matrix_data = path
            .buffers
            .lights_frame_matrix_pool
            .next(fmd)
            .expect("cannot take next buffer");

        let mut b = self.builder.take().unwrap();

        b.begin_render_pass(
            self.framebuffer.clone(),
            false,
            vec![
                ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                ClearValue::Depth(1.0),
                ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                ClearValue::None,
            ],
        )
        .unwrap();

        // 1. SUBPASS - Geometry
        for x in state.objects.iter() {
            x.draw_indexed(&dynamic_state, geometry_frame_matrix_data.clone(), &mut b)
        }
        b.next_subpass(false).unwrap();

        // 2. SUBPASS - Lighting
        let mut lights = [DirectionalLight {
            direction: Vector3::zero(),
            intensity: 0.0,
            color: Vector3::zero(),
        }; 1024];
        for (idx, light) in state.directional_lights.iter().enumerate() {
            lights[idx] = *light;
        }
        let lighting_lights_ds = path.lights_buffer_pool.next(lights).unwrap();
        b.draw_indexed(
            path.buffers.lighting_pipeline.clone(),
            &dynamic_state,
            vec![path.fst.vertex_buffer().clone()],
            path.fst.index_buffer().clone(),
            (
                lights_frame_matrix_data,
                path.buffers.lighting_gbuffer_ds.clone(),
                lighting_lights_ds,
            ),
            shaders::fs_deferred_lighting::ty::PushConstants {
                resolution: dims,
                light_count: state.directional_lights.len() as u32,
            },
        )
        .expect("cannot do lighting pass")
        .next_subpass(false)
        .unwrap();

        // 3. SUBPASS - Skybox
        path.sky.draw(&dynamic_state, fmd, &mut b);
        b.next_subpass(false).unwrap();

        // 4. SUBPASS - Tonemap
        b.draw_indexed(
            path.buffers.tonemap_pipeline.clone(),
            &dynamic_state,
            vec![path.fst.vertex_buffer().clone()],
            path.fst.index_buffer().clone(),
            path.buffers.tonemap_ds.clone(),
            (),
        )
        .expect("cannot do tonemap pass");
        b.end_render_pass().unwrap();
        b.build().unwrap()
    }
}
