//! Objects & procedures related to rendering.

use crate::camera::Camera;
use crate::render::hosek::HosekSky;
use crate::render::pools::UniformBufferPool;
use crate::render::ubo::{DirectionalLight, FrameMatrixData};
use crate::render::vertex::{NormalMappedVertex, PositionOnlyVertex};
use crate::resources::image::create_single_pixel_image;
use crate::resources::mesh::{create_full_screen_triangle, IndexedMesh};
use crate::samplers::Samplers;
use crate::GameState;
use cgmath::{EuclideanSpace, SquareMatrix, Vector3, Zero};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::{DescriptorSet, PipelineLayoutAbstract};
use vulkano::device::{Device, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::framebuffer::{Framebuffer, FramebufferCreationError, Subpass};
use vulkano::framebuffer::{FramebufferAbstract, RenderPassAbstract};
use vulkano::image::{
    AttachmentImage, ImageCreationError, ImageUsage, ImmutableImage, SwapchainImage,
};
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::swapchain::Swapchain;
use winit::window::Window;

// consts to descriptor set binding indices
pub const FRAME_DATA_UBO_DESCRIPTOR_SET: usize = 0;
pub const OBJECT_DATA_UBO_DESCRIPTOR_SET: usize = 2;
pub const SUBPASS_UBO_DESCRIPTOR_SET: usize = 1;
pub const LIGHTS_UBO_DESCRIPTOR_SET: usize = 2;

pub mod hosek;
pub mod object;
pub mod pools;
pub mod renderer;
pub mod transform;
pub mod ubo;
pub mod vertex;
pub mod vulkan;

struct GBuffer {
    buffer1: Arc<AttachmentImage>,
    buffer2: Arc<AttachmentImage>,
    buffer3: Arc<AttachmentImage>,
}

impl GBuffer {
    pub fn new(device: Arc<Device>, dimensions: [u32; 2]) -> Result<Self, ImageCreationError> {
        Ok(Self {
            buffer1: AttachmentImage::with_usage(
                device.clone(),
                dimensions,
                Format::A2B10G10R10UnormPack32,
                ImageUsage {
                    transient_attachment: true,
                    input_attachment: true,
                    ..ImageUsage::none()
                },
            )?,
            buffer2: AttachmentImage::with_usage(
                device.clone(),
                dimensions,
                Format::R8G8B8A8Unorm,
                ImageUsage {
                    transient_attachment: true,
                    input_attachment: true,
                    ..ImageUsage::none()
                },
            )?,
            buffer3: AttachmentImage::with_usage(
                device,
                dimensions,
                Format::R8G8B8A8Unorm,
                ImageUsage {
                    transient_attachment: true,
                    input_attachment: true,
                    ..ImageUsage::none()
                },
            )?,
        })
    }
}

pub type FrameMatrixPool = UniformBufferPool<FrameMatrixData>;

// long-lived global (vulkan) objects related to one render path (buffers, pipelines)
pub struct PBRDeffered {
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    pub buffers: RenderPathBuffers,
    pub samplers: Samplers,
    pub white_texture: Arc<ImmutableImage<Format>>,
    pub black_texture: Arc<ImmutableImage<Format>>,
    pub normal_texture: Arc<ImmutableImage<Format>>,

    fst: Arc<IndexedMesh<PositionOnlyVertex, u16>>,
    pub sky: HosekSky,
}

// long-lived global buffers and data dependant on the render resolution
pub struct RenderPathBuffers {
    hdr_buffer: Arc<AttachmentImage>,
    geometry_buffer: GBuffer,
    depth_buffer: Arc<AttachmentImage>,
    // pipelines are dependant on the resolution
    pub geometry_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    lighting_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    tonemap_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    // constant descriptor sets
    tonemap_ds: Arc<dyn DescriptorSet + Send + Sync>,
    lighting_gbuffer_ds: Arc<dyn DescriptorSet + Send + Sync>,
    lights_buffer_pool: CpuBufferPool<[DirectionalLight; 1024]>,
    geometry_frame_matrix_pool: FrameMatrixPool,
    lights_frame_matrix_pool: FrameMatrixPool,
}

impl RenderPathBuffers {
    fn new(
        render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
        device: Arc<Device>,
        dimensions: [u32; 2],
    ) -> Self {
        // we create required shaders for all graphical pipelines we use in this
        // render pass from precompiled (embedded) spri-v binary data from soruces.
        let vs = crate::shaders::vs_deferred_geometry::Shader::load(device.clone()).unwrap();
        let fs = crate::shaders::fs_deferred_geometry::Shader::load(device.clone()).unwrap();
        let tm_vs = crate::shaders::vs_passtrough::Shader::load(device.clone()).unwrap();
        let tm_fs = crate::shaders::fs_tonemap::Shader::load(device.clone()).unwrap();
        let dl_fs = crate::shaders::fs_deferred_lighting::Shader::load(device.clone()).unwrap();

        // create basic pipeline for drawing
        let geometry_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<NormalMappedVertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .fragment_shader(fs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .depth_stencil(DepthStencil::simple_depth_test())
                .cull_mode_back()
                .front_face_clockwise()
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .expect("cannot create graphics pipeline"),
        );

        let lighting_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<PositionOnlyVertex>()
                .vertex_shader(tm_vs.main_entry_point(), ())
                .fragment_shader(dl_fs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
                .build(device.clone())
                .expect("cannot build tonemap graphics pipeline"),
        );

        let tonemap_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<PositionOnlyVertex>()
                .vertex_shader(tm_vs.main_entry_point(), ())
                .fragment_shader(tm_fs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .render_pass(Subpass::from(render_pass.clone(), 3).unwrap())
                .build(device.clone())
                .expect("cannot build tonemap graphics pipeline"),
        );

        let depth_buffer = AttachmentImage::with_usage(
            device.clone(),
            dimensions,
            Format::D16Unorm,
            ImageUsage {
                transient_attachment: true,
                depth_stencil_attachment: true,
                input_attachment: true,
                ..ImageUsage::none()
            },
        )
        .expect("cannot create depth buffer");

        let hdr_buffer = AttachmentImage::with_usage(
            device.clone(),
            dimensions,
            Format::B10G11R11UfloatPack32,
            ImageUsage {
                transient_attachment: true,
                input_attachment: true,
                ..ImageUsage::none()
            },
        )
        .expect("cannot create hdr buffer");

        let geometry_buffer =
            GBuffer::new(device.clone(), dimensions).expect("cannot create geometry buffer");

        // todo: decide whether we need this for using subpassLoad in shaders
        let tonemap_ds = Arc::new(
            PersistentDescriptorSet::start(
                tonemap_pipeline
                    .descriptor_set_layout(0) // workaround: vulkano does not work with sparse indices
                    .unwrap()
                    .clone(),
            )
            .add_image(hdr_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
        );
        let lighting_gbuffer_ds = Arc::new(
            PersistentDescriptorSet::start(
                lighting_pipeline
                    .descriptor_set_layout(SUBPASS_UBO_DESCRIPTOR_SET)
                    .unwrap()
                    .clone(),
            )
            .add_image(geometry_buffer.buffer1.clone())
            .unwrap()
            .add_image(geometry_buffer.buffer2.clone())
            .unwrap()
            .add_image(geometry_buffer.buffer3.clone())
            .unwrap()
            .add_image(depth_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
        );

        let lights_buffer_pool = CpuBufferPool::new(device.clone(), BufferUsage::uniform_buffer());

        Self {
            geometry_frame_matrix_pool: FrameMatrixPool::new(
                device.clone(),
                geometry_pipeline
                    .descriptor_set_layout(FRAME_DATA_UBO_DESCRIPTOR_SET)
                    .unwrap()
                    .clone(),
            ),
            lights_frame_matrix_pool: FrameMatrixPool::new(
                device,
                lighting_pipeline
                    .descriptor_set_layout(FRAME_DATA_UBO_DESCRIPTOR_SET)
                    .unwrap()
                    .clone(),
            ),
            geometry_pipeline: geometry_pipeline as Arc<_>,
            tonemap_pipeline: tonemap_pipeline as Arc<_>,
            tonemap_ds: tonemap_ds as Arc<_>,
            lighting_pipeline: lighting_pipeline as Arc<_>,
            lighting_gbuffer_ds: lighting_gbuffer_ds as Arc<_>,
            lights_buffer_pool,
            depth_buffer,
            geometry_buffer,
            hdr_buffer,
        }
    }
}

impl PBRDeffered {
    pub fn new(queue: Arc<Queue>, device: Arc<Device>, swapchain: Arc<Swapchain<Window>>) -> Self {
        // first we generate some useful resources on the fly
        let (fst, _) = create_full_screen_triangle(queue.clone()).expect("cannot create fst");
        let (white_texture, _) = create_single_pixel_image(queue.clone(), [255; 4]).unwrap();
        let (black_texture, _) = create_single_pixel_image(queue.clone(), [0; 4]).unwrap();
        let (normal_texture, _) =
            create_single_pixel_image(queue.clone(), [128, 128, 255, 255]).unwrap();

        // this example render path uses one render pass which renders all geometry and then
        // the skybox with one directional light without any shadows.
        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(
                device.clone(),
                attachments: {
                    gbuffer1: {
                        load: Clear,
                        store: Store,
                        format: Format::A2B10G10R10UnormPack32,
                        samples: 1,
                    },
                    gbuffer2: {
                        load: Clear,
                        store: Store,
                        format: Format::R8G8B8A8Unorm,
                        samples: 1,
                    },
                    gbuffer3: {
                        load: Clear,
                        store: Store,
                        format: Format::R8G8B8A8Unorm,
                        samples: 1,
                    },
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: Format::D16Unorm,
                        samples: 1,
                    },
                    hdr: {
                        load: Clear,
                        store: DontCare,
                        format: Format::B10G11R11UfloatPack32,
                        samples: 1,
                    },
                    final_color: {
                        load: DontCare,
                        store: Store,
                        format: swapchain.format(),
                        samples: 1,
                    }
                },
                passes: [
                    {
                        color: [gbuffer1, gbuffer2, gbuffer3],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                        color: [hdr],
                        depth_stencil: {},
                        input: [gbuffer1, gbuffer2, gbuffer3, depth]
                    },
                    {
                        color: [hdr],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                         color: [final_color],
                         depth_stencil: {},
                         input: [hdr]
                    }
                ]
            )
            .expect("cannot create render pass"),
        );

        let samplers = Samplers::new(device.clone()).unwrap();

        let buffers =
            RenderPathBuffers::new(render_pass.clone(), device.clone(), swapchain.dimensions());
        let sky = HosekSky::new(queue, render_pass.clone(), device.clone());

        Self {
            fst,
            buffers,
            render_pass: render_pass as Arc<_>,
            sky,
            samplers,
            white_texture,
            black_texture,
            normal_texture,
        }
    }

    pub fn create_framebuffer(
        &self,
        final_image: Arc<SwapchainImage<Window>>,
    ) -> Result<Arc<dyn FramebufferAbstract + Send + Sync>, FramebufferCreationError> {
        Ok(Arc::new(
            Framebuffer::start(self.render_pass.clone())
                .add(self.buffers.geometry_buffer.buffer1.clone())?
                .add(self.buffers.geometry_buffer.buffer2.clone())?
                .add(self.buffers.geometry_buffer.buffer3.clone())?
                .add(self.buffers.depth_buffer.clone())?
                .add(self.buffers.hdr_buffer.clone())?
                .add(final_image)?
                .build()?,
        ))
    }

    pub fn recreate_buffers(&mut self, dimensions: [u32; 2]) {
        self.buffers = RenderPathBuffers::new(
            self.render_pass.clone(),
            self.render_pass.device().clone(),
            dimensions,
        )
    }
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
        let lighting_lights_ds = Arc::new(
            PersistentDescriptorSet::start(
                path.buffers
                    .lighting_pipeline
                    .descriptor_set_layout(LIGHTS_UBO_DESCRIPTOR_SET)
                    .unwrap()
                    .clone(),
            )
            .add_buffer(path.buffers.lights_buffer_pool.next(lights).unwrap())
            .unwrap()
            .build()
            .unwrap(),
        );
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
            crate::shaders::fs_deferred_lighting::ty::PushConstants {
                camera_pos: state.camera.position.into(),
                resolution: dims,
                light_count: state.directional_lights.len() as u32,
                _dummy0: [0u8; 4],
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
