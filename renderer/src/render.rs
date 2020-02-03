use crate::camera::Camera;
use crate::hosek::make_hosek_wilkie_params;
use crate::io::{load_geometry, load_image};
use crate::mesh::{create_full_screen_triangle, Mesh};
use crate::pod::{HosekWilkieParams, MaterialData, MatrixData};
use crate::samplers::Samplers;
use crate::sky::SkyParams;
use crate::window::SwapChain;
use crate::{make_ubo, GameState};
use cgmath::{vec3, Matrix4, Quaternion, Vector3};
use log::info;
use safe_transmute::TriviallyTransmutable;
use std::sync::Arc;
use vulkano::buffer::CpuBufferPool;
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::DescriptorSet;
use vulkano::device::{Device, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::{AttachmentImage, ImageAccess, ImageUsage, ImageViewAccess, ImmutableImage};
use vulkano::pipeline::depth_stencil::{Compare, DepthBounds, DepthStencil};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;

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

pub struct Renderer {
    pub viewport: Viewport,
    pub samplers: Samplers,
    pub graphical_queue: Arc<Queue>,
    pub device: Arc<Device>,
}

impl Renderer {
    #[inline]
    pub fn dimensions(&self) -> [u32; 2] {
        [
            self.viewport.dimensions[0] as u32,
            self.viewport.dimensions[1] as u32,
        ]
    }
}

// ----------------------------

/// Represents a global storage for data required to render frames. Opposite to
/// `Frame` this struct contains data that is useful for rendering of any frame
/// not exactly the one.
pub struct FrameSystem {
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    // all additional buffers needed by this render path
    hdr_buffer: Arc<AttachmentImage>,
    depth_buffer: Arc<AttachmentImage>,

    /***** KOKOTINY *****/
    pub geometry_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    skybox_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    tonemap_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    // constant descriptor sets
    tonemap_ds: Arc<dyn DescriptorSet + Send + Sync>,
    sky_params: SkyParams,
    fst: Mesh<PositionOnlyVertex, u16>,
    matrix_data_pool: CpuBufferPool<MatrixData>,
    hosek_wilkie_sky_pool: CpuBufferPool<HosekWilkieParams>,
    // resources
    rock_mesh: Mesh<BasicVertex, u16>,
    icosphere_mesh: Mesh<BasicVertex, u16>,
    plane_mesh: Mesh<BasicVertex, u16>,
    rock_albedo: Arc<ImmutableImage<Format>>,
    basic: Arc<ImmutableImage<Format>>,
    rock_material: Arc<dyn DescriptorSet + Send + Sync>,
    white_material: Arc<dyn DescriptorSet + Send + Sync>,
}

impl FrameSystem {
    pub fn new(renderer: &Renderer, swapchain: &SwapChain) -> Self {
        let (fst, fst_future) = create_full_screen_triangle(renderer.graphical_queue.clone())
            .expect("cannot create fst");

        // this example render path uses one render pass which renders all geometry and then
        // the skybox with one directional light without any shadows.
        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(
                renderer.device.clone(),
                attachments: {
                    hdr: {
                        load: Clear,
                        store: DontCare,
                        format: Format::B10G11R11UfloatPack32,
                        samples: 1,
                    },
                    color: {
                        load: DontCare,
                        store: Store,
                        format: swapchain.swapchain.format(),
                        samples: 1,
                    },
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: Format::D16Unorm,
                        samples: 1,
                    }
                },
                passes: [
                    {
                        color: [hdr],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                        color: [hdr],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                         color: [color],
                         depth_stencil: {},
                         input: [hdr]
                    }
                ]
            )
            .expect("cannot create render pass"),
        );

        // we create required shaders for all graphical pipelines we use in this
        // render pass from precompiled (embedded) spri-v binary data from soruces.
        let vs = crate::shaders::basic_vert::Shader::load(renderer.device.clone()).unwrap();
        let fs = crate::shaders::basic_frag::Shader::load(renderer.device.clone()).unwrap();
        let sky_vs = crate::shaders::sky_vert::Shader::load(renderer.device.clone()).unwrap();
        let sky_fs = crate::shaders::sky_frag::Shader::load(renderer.device.clone()).unwrap();
        let tm_vs = crate::shaders::tonemap_vert::Shader::load(renderer.device.clone()).unwrap();
        let tm_fs = crate::shaders::tonemap_frag::Shader::load(renderer.device.clone()).unwrap();

        // create basic pipeline for drawing
        let geometry_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<BasicVertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .fragment_shader(fs.main_entry_point(), ())
                .triangle_list()
                .viewports(Some(renderer.viewport.clone()))
                .depth_stencil(DepthStencil::simple_depth_test())
                .cull_mode_back()
                .front_face_clockwise()
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(renderer.device.clone())
                .expect("cannot create graphics pipeline"),
        );

        let skybox_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<BasicVertex>()
                .vertex_shader(sky_vs.main_entry_point(), ())
                .fragment_shader(sky_fs.main_entry_point(), ())
                .triangle_list()
                .viewports(Some(renderer.viewport.clone()))
                .depth_stencil(DepthStencil {
                    depth_compare: Compare::LessOrEqual,
                    depth_write: false,
                    depth_bounds_test: DepthBounds::Disabled,
                    stencil_front: Default::default(),
                    stencil_back: Default::default(),
                })
                .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
                .build(renderer.device.clone())
                .expect("cannot create aky pipeline"),
        );

        let tonemap_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<PositionOnlyVertex>()
                .vertex_shader(tm_vs.main_entry_point(), ())
                .fragment_shader(tm_fs.main_entry_point(), ())
                .triangle_list()
                .viewports(Some(renderer.viewport.clone()))
                .render_pass(Subpass::from(render_pass.clone(), 2).unwrap())
                .build(renderer.device.clone())
                .expect("cannot build tonemap graphics pipeline"),
        );

        let depth_buffer = AttachmentImage::with_usage(
            renderer.device.clone(),
            renderer.dimensions(),
            Format::D16Unorm,
            ImageUsage {
                transient_attachment: true,
                depth_stencil_attachment: true,
                ..ImageUsage::none()
            },
        )
        .expect("cannot create depth buffer");
        let hdr_buffer = AttachmentImage::with_usage(
            renderer.device.clone(),
            renderer.dimensions(),
            Format::B10G11R11UfloatPack32,
            ImageUsage {
                transient_attachment: true,
                input_attachment: true,
                ..ImageUsage::none()
            },
        )
        .expect("cannot create hdr buffer");

        // todo: decide whether we need this
        let tonemap_ds = Arc::new(
            PersistentDescriptorSet::start(tonemap_pipeline.clone(), 0)
                .add_image(hdr_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );

        // TODO: remove from render path
        info!("loading geometry and image data...");
        let rock_mesh = load_geometry(
            renderer.graphical_queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1.bf",
        );
        let icosphere_mesh = load_geometry(
            renderer.graphical_queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\icosphere.bf",
        );
        let plane_mesh = load_geometry(
            renderer.graphical_queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\plane.bf",
        );
        let rock_albedo = load_image(
            renderer.graphical_queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1_Base_Color.bf",
        );
        let basic = load_image(
            renderer.graphical_queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\basic.bf",
        );
        info!("data loaded!");

        let rock_material = Arc::new(
            PersistentDescriptorSet::start(geometry_pipeline.clone(), 0)
                .add_sampled_image(rock_albedo.clone(), renderer.samplers.aniso_repeat.clone())
                .unwrap()
                .add_buffer(make_ubo(
                    renderer.graphical_queue.clone(),
                    MaterialData {
                        albedo_color: vec3(1.0, 1.0, 1.0),
                        alpha_cutoff: 0.0,
                    },
                ))
                .unwrap()
                .build()
                .expect("cannot build pds"),
        );

        let white_material = Arc::new(
            PersistentDescriptorSet::start(geometry_pipeline.clone(), 0)
                .add_sampled_image(basic.clone(), renderer.samplers.aniso_repeat.clone())
                .unwrap()
                .add_buffer(make_ubo(
                    renderer.graphical_queue.clone(),
                    MaterialData {
                        albedo_color: vec3(1.0, 1.0, 1.0),
                        alpha_cutoff: 0.0,
                    },
                ))
                .unwrap()
                .build()
                .expect("cannot build pds"),
        );

        Self {
            fst,
            sky_params: SkyParams::default(),
            render_pass: render_pass as Arc<_>,
            geometry_pipeline: geometry_pipeline as Arc<_>,
            skybox_pipeline: skybox_pipeline as Arc<_>,
            tonemap_pipeline: tonemap_pipeline as Arc<_>,
            tonemap_ds: tonemap_ds as Arc<_>,
            matrix_data_pool: CpuBufferPool::uniform_buffer(renderer.device.clone()),
            hosek_wilkie_sky_pool: CpuBufferPool::uniform_buffer(renderer.device.clone()),
            depth_buffer,
            hdr_buffer,
            //
            rock_mesh,
            icosphere_mesh,
            plane_mesh,
            rock_albedo,
            basic,
            rock_material,
            white_material,
        }
    }

    /// Creates a new Frame object that can be used to render one frame. One Frame
    /// object is roughly equivalent to one swap-chain image.
    ///
    /// When swap-chain is recreated, old Frame objects are destructed and new are
    /// created by `FrameSystem.create_frame()` method.
    pub fn create_frame<I>(&mut self, final_image: I) -> Frame
    where
        I: ImageAccess + ImageViewAccess + Clone + Send + Sync + 'static,
    {
        Frame {
            framebuffer: Arc::new(
                Framebuffer::start(self.render_pass.clone())
                    .add(self.hdr_buffer.clone())
                    .unwrap()
                    .add(final_image)
                    .unwrap()
                    .add(self.depth_buffer.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ),
            system: self,
        }
    }
}

pub struct Frame<'s> {
    system: &'s mut FrameSystem,
    framebuffer: Arc<dyn FramebufferAbstract + Send + Sync>,
}

impl<'s> Frame<'s> {
    pub fn render(&self, renderer: &Renderer, state: &GameState) -> AutoCommandBuffer {
        let no_dynamic_state = DynamicState::none();

        // create descriptor sets
        let rock_transform = Transform {
            position: vec3(0.0, 1.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: vec3(0.03, 0.03, 0.03),
        };
        let ubo_rock = self
            .system
            .matrix_data_pool
            .next(MatrixData {
                model: rock_transform.into(),
                view: state.camera.view_matrix(),
                projection: state.camera.projection_matrix(),
            })
            .expect("cannot create next sub-buffer");
        let rock_ds = PersistentDescriptorSet::start(self.system.geometry_pipeline.clone(), 1)
            .add_buffer(ubo_rock)
            .expect("cannot add ubo to pds set=1")
            .build()
            .expect("cannot build pds set=1");
        let plane_transform = Transform {
            position: vec3(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: vec3(30.0, 1.0, 30.0),
        };
        let ubo_plane = self
            .system
            .matrix_data_pool
            .next(MatrixData {
                model: plane_transform.into(),
                view: state.camera.view_matrix(),
                projection: state.camera.projection_matrix(),
            })
            .expect("cannot create next sub-buffer");
        let plane_ds = PersistentDescriptorSet::start(self.system.geometry_pipeline.clone(), 1)
            .add_buffer(ubo_plane)
            .expect("cannot add ubo to pds set=1")
            .build()
            .expect("cannot build pds set=1");
        let params = make_hosek_wilkie_params(state.sun_dir, 2.0, vec3(0.0, 0.0, 0.0));
        let ubo_sky_hw = self.system.hosek_wilkie_sky_pool.next(params).unwrap();
        let sky_hw_params = PersistentDescriptorSet::start(self.system.skybox_pipeline.clone(), 1)
            .add_buffer(ubo_sky_hw)
            .expect("cannot add ubo to pds set=1")
            .build()
            .expect("cannot build pds set=1");
        let ubo_sky = self
            .system
            .matrix_data_pool
            .next(MatrixData {
                model: Matrix4::from_scale(200.0),
                view: state.camera.view_matrix(),
                projection: state.camera.projection_matrix(),
            })
            .expect("cannot create next sub-buffer");

        let per_object_descriptor_set_sky =
            PersistentDescriptorSet::start(self.system.skybox_pipeline.clone(), 0)
                .add_buffer(ubo_sky)
                .expect("cannot add ubo to pds set=1")
                .build()
                .expect("cannot build pds set=1");

        AutoCommandBufferBuilder::primary_one_time_submit(
            renderer.device.clone(),
            renderer.graphical_queue.family(),
        )
        .unwrap()
        .begin_render_pass(
            self.framebuffer.clone(),
            false,
            vec![
                ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                ClearValue::None,
                ClearValue::Depth(1.0),
            ],
        )
        .unwrap()
        .draw_indexed(
            self.system.geometry_pipeline.clone(),
            &no_dynamic_state,
            vec![self.system.rock_mesh.vertex_buffer.clone()],
            self.system.rock_mesh.index_buffer.clone(),
            (self.system.rock_material.clone(), rock_ds),
            state.sun_dir,
        )
        .unwrap()
        .draw_indexed(
            self.system.geometry_pipeline.clone(),
            &no_dynamic_state,
            vec![self.system.plane_mesh.vertex_buffer.clone()],
            self.system.plane_mesh.index_buffer.clone(),
            (self.system.white_material.clone(), plane_ds),
            state.sun_dir,
        )
        .unwrap()
        .next_subpass(false)
        .unwrap()
        .draw_indexed(
            self.system.skybox_pipeline.clone(),
            &no_dynamic_state,
            vec![self.system.icosphere_mesh.vertex_buffer.clone()],
            self.system.icosphere_mesh.index_buffer.clone(),
            (per_object_descriptor_set_sky, sky_hw_params),
            (state.camera.position, state.start.elapsed().as_secs_f32()),
        )
        .unwrap()
        .next_subpass(false)
        .unwrap()
        .draw_indexed(
            self.system.tonemap_pipeline.clone(),
            &no_dynamic_state,
            vec![self.system.fst.vertex_buffer.clone()],
            self.system.fst.index_buffer.clone(),
            self.system.tonemap_ds.clone(),
            (),
        )
        .unwrap()
        .end_render_pass()
        .unwrap()
        .build()
        .unwrap()
    }
}
