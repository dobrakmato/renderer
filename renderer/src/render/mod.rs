//! Objects & procedures related to rendering.

use crate::camera::Camera;
use crate::render::pbr::PBRDeffered;
use crate::render::pools::UniformBufferPool;
use crate::render::ubo::{DirectionalLight, FrameMatrixData};
use crate::resources::mesh::DynamicIndexedMesh;
use crate::GameState;
use bf::material::BlendMode;
use cgmath::{EuclideanSpace, SquareMatrix, Vector3, Zero};
use cstr::cstr;
use std::sync::Arc;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer, SubpassContents,
};
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::device::{Device, Queue};
use vulkano::format::ClearValue;
use vulkano::image::SwapchainImage;
use vulkano::pipeline::layout::PipelineLayout;
use vulkano::pipeline::viewport::Viewport;
use vulkano::render_pass::FramebufferAbstract;
use winit::window::Window;

// consts to descriptor set binding indices
pub const FRAME_DATA_UBO_DESCRIPTOR_SET: usize = 0;
pub const OBJECT_DATA_UBO_DESCRIPTOR_SET: usize = 2;
pub const SUBPASS_UBO_DESCRIPTOR_SET: usize = 1;
pub const LIGHTS_UBO_DESCRIPTOR_SET: usize = 2;

pub mod fxaa;
pub mod hosek;
pub mod mcguire13;
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

/// Helper function to retrieve `DescriptorSetLayout` from pipeline
/// by specifying the index of the layout.
///
/// # Panics
///
/// This function panics if `index` is invalid set index for provided pipeline.
///
pub fn descriptor_set_layout(pipeline: &PipelineLayout, index: usize) -> Arc<DescriptorSetLayout> {
    pipeline
        .descriptor_set_layouts()
        .get(index)
        .expect("cannot get descriptor set layout")
        .clone()
}

pub struct Frame<'r, 's> {
    render_path: &'r mut PBRDeffered,
    game_state: &'s GameState,
    framebuffer: Arc<dyn FramebufferAbstract + Send + Sync>,
    builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
}

impl<'r, 's> Frame<'r, 's> {
    pub fn build(&mut self) -> PrimaryAutoCommandBuffer {
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
        let frame_matrix_data = Arc::new(
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
        let transparency_frame_matrix_data = Arc::new(
            path.buffers
                .transparency_frame_matrix_pool
                .next(fmd)
                .expect("cannot take next buffer"),
        );

        let mut b = self.builder.take().unwrap();

        b.begin_render_pass(
            path.buffers.main_framebuffer.clone(),
            SubpassContents::Inline,
            vec![
                ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                ClearValue::Depth(1.0),
                ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                ClearValue::None,
                // transparency
                ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                ClearValue::Float([1.0, 0.0, 0.0, 0.0]),
            ],
        )
        .unwrap();

        // 1.1. SUBPASS - Opaque Geometry
        b.debug_marker_begin(cstr!("Geometry Pass"), [1.0, 0.0, 0.0, 1.0])
            .unwrap();
        for x in state
            .objects
            .iter()
            .filter(|x| x.material.blend_mode() == BlendMode::Opaque)
        {
            let object_matrix_data = x
                .object_matrix_data()
                .expect("cannot create ObjectMatrixData for this frame");

            // todo: get rid of this dispatch somehow
            match &*x.mesh {
                DynamicIndexedMesh::U16(m) => b
                    .draw_indexed(
                        x.pipeline.clone(),
                        &dynamic_state,
                        vec![m.vertex_buffer().clone()],
                        m.index_buffer().clone(),
                        (
                            frame_matrix_data.clone(),
                            x.material.descriptor_set(),
                            object_matrix_data,
                        ),
                        (),
                    )
                    .expect("cannot DrawIndexed this mesh"),
                DynamicIndexedMesh::U32(m) => b
                    .draw_indexed(
                        x.pipeline.clone(),
                        &dynamic_state,
                        vec![m.vertex_buffer().clone()],
                        m.index_buffer().clone(),
                        (
                            frame_matrix_data.clone(),
                            x.material.descriptor_set(),
                            object_matrix_data,
                        ),
                        (),
                    )
                    .expect("cannot DrawIndexed this mesh"),
            };
        }
        b.next_subpass(SubpassContents::Inline).unwrap();
        b.debug_marker_end().unwrap();

        // 1.2. SUBPASS - Lighting
        b.debug_marker_begin(cstr!("Lighting Pass"), [1.0, 1.0, 0.0, 1.0])
            .unwrap();
        let mut lights = [DirectionalLight {
            direction: Vector3::zero(),
            intensity: 0.0,
            color: Vector3::zero(),
        }; 100];
        for (idx, light) in state.directional_lights.iter().enumerate() {
            lights[idx] = *light;
        }
        let lighting_lights_ds = Arc::new(path.lights_buffer_pool.next(lights).unwrap());
        b.draw_indexed(
            path.buffers.lighting_pipeline.clone(),
            &dynamic_state,
            vec![path.fst.vertex_buffer().clone()],
            path.fst.index_buffer().clone(),
            (
                lights_frame_matrix_data,
                path.buffers.lighting_gbuffer_ds.clone(),
                lighting_lights_ds.clone(),
            ),
            shaders::fs_deferred_lighting::ty::PushConstants {
                resolution: dims,
                light_count: state.directional_lights.len() as u32,
            },
        )
        .expect("cannot do lighting pass")
        .next_subpass(SubpassContents::Inline)
        .unwrap();
        b.debug_marker_end().unwrap();

        // 1.3. SUBPASS - Skybox
        b.debug_marker_begin(cstr!("Skybox"), [0.0, 0.0, 1.0, 1.0])
            .unwrap();
        path.sky.draw(&dynamic_state, fmd, &mut b);
        b.next_subpass(SubpassContents::Inline).unwrap();
        b.debug_marker_end().unwrap();

        // 1.4. SUBPASS - Transparent Geometry
        b.debug_marker_begin(cstr!("Accumulate Transparency Pass"), [1.0, 0.2, 0.5, 1.0])
            .unwrap();
        for x in state
            .objects
            .iter()
            .filter(|x| x.material.blend_mode() == BlendMode::Translucent)
        {
            let object_matrix_data = x
                .object_matrix_data()
                .expect("cannot create ObjectMatrixData for this frame");

            // todo: get rid of this dispatch somehow
            match &*x.mesh {
                DynamicIndexedMesh::U16(m) => b
                    .draw_indexed(
                        path.buffers.transparency.accumulation_pipeline.clone(),
                        &dynamic_state,
                        vec![m.vertex_buffer().clone()],
                        m.index_buffer().clone(),
                        (
                            transparency_frame_matrix_data.clone(),
                            x.material.descriptor_set(),
                            object_matrix_data,
                            lighting_lights_ds.clone(),
                        ),
                        mcguire13::shaders::accumulation_fs::ty::PushConstants {
                            resolution: dims,
                            light_count: state.directional_lights.len() as u32,
                        },
                    )
                    .expect("cannot DrawIndexed this mesh"),
                DynamicIndexedMesh::U32(m) => b
                    .draw_indexed(
                        path.buffers.transparency.accumulation_pipeline.clone(),
                        &dynamic_state,
                        vec![m.vertex_buffer().clone()],
                        m.index_buffer().clone(),
                        (
                            transparency_frame_matrix_data.clone(),
                            x.material.descriptor_set(),
                            object_matrix_data,
                            lighting_lights_ds.clone(),
                        ),
                        mcguire13::shaders::accumulation_fs::ty::PushConstants {
                            resolution: dims,
                            light_count: state.directional_lights.len() as u32,
                        },
                    )
                    .expect("cannot DrawIndexed this mesh"),
            };
        }
        b.next_subpass(SubpassContents::Inline).unwrap();
        b.debug_marker_end().unwrap();
        b.debug_marker_begin(cstr!("Resolve Transparency Pass"), [1.0, 0.2, 0.5, 1.0])
            .unwrap();
        b.draw_indexed(
            path.buffers.transparency.resolve_pipeline.clone(),
            &dynamic_state,
            vec![path.fst.vertex_buffer().clone()],
            path.fst.index_buffer().clone(),
            path.buffers.transparency.resolve_ds.clone(),
            (),
        )
        .expect("cannot do transparency resolve pass");
        b.next_subpass(SubpassContents::Inline).unwrap();
        b.debug_marker_end().unwrap();

        // 1.5. SUBPASS - Tonemap
        b.debug_marker_begin(cstr!("Tonemap"), [0.5, 0.5, 1.0, 0.0])
            .unwrap();
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
        b.debug_marker_end().unwrap();

        // 2.1 FXAA
        b.debug_marker_begin(cstr!("FXAA"), [1.0, 0.3, 0.0, 1.0]);
        b.begin_render_pass(
            self.framebuffer.clone(),
            SubpassContents::Inline,
            vec![ClearValue::None],
        )
        .unwrap();
        b.draw_indexed(
            path.fxaa.fxaa_pipeline.clone(),
            &dynamic_state,
            vec![path.fxaa.fst.vertex_buffer().clone()],
            path.fxaa.fst.index_buffer().clone(),
            path.fxaa.fxaa_descriptor_set.clone(),
            fxaa::shaders::fragment::ty::PushConstants { resolution: dims },
        )
        .expect("cannot do fxaa pass");
        b.end_render_pass();
        b.debug_marker_end();

        b.build().unwrap()
    }
}
