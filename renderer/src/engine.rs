use crate::camera::Camera;
use crate::hosek::make_hosek_wilkie_params;
use crate::input::Input;
use crate::io::{load_geometry, load_image};
use crate::mesh::{create_full_screen_triangle, Mesh};
use crate::pod::{HosekWilkieParams, MaterialData, MatrixData};
use crate::render::{BasicVertex, PositionOnlyVertex, Transform};
use crate::samplers::Samplers;
use crate::sky::SkyParams;
use crate::{GameState, RendererConfiguration};
use cgmath::{vec3, Matrix4, Quaternion, Rad};
use log::info;
use smallvec::SmallVec;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::{DescriptorSet, PipelineLayoutAbstract};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::framebuffer::{FramebufferAbstract, RenderPassAbstract};
use vulkano::image::{AttachmentImage, ImageUsage, ImmutableImage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::depth_stencil::{Compare, DepthBounds, DepthStencil};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::sampler::Sampler;
use vulkano::swapchain::{
    ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
};
use vulkano::sync::GpuFuture;
use vulkano::{app_info_from_cargo_toml, swapchain};
use vulkano_win::VkSurfaceBuild;
use winit::event::{DeviceEvent, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

/// main struct containing everything
pub struct Engine {
    pub game_state: GameState,
    pub renderer_state: RendererState,
    pub input_state: Input,
    pub event_loop: Option<EventLoop<()>>,
}

impl Engine {
    pub fn run_forever(mut self) -> ! {
        self.event_loop.take().unwrap().run(move |ev, _, flow| {
            match ev {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *flow = ControlFlow::Exit,
                    WindowEvent::Focused(focus) => self.input_state.set_input_enabled(focus),
                    _ => {}
                },
                Event::DeviceEvent { event, .. } => {
                    if let DeviceEvent::Key(k) = event {
                        self.input_state.handle_event(k)
                    }
                    if let DeviceEvent::MouseMotion { delta } = event {
                        if self.input_state.input_enabled {
                            self.game_state
                                .camera
                                .rotate(Rad(delta.0 as f32 * 0.001), Rad(delta.1 as f32 * 0.001))
                        }
                    }
                }
                Event::RedrawEventsCleared => {
                    self.renderer_state.render_frame(&self.game_state);

                    /* game update for next frame */
                    let speed = if self.input_state.is_key_down(VirtualKeyCode::LShift) {
                        0.01
                    } else {
                        0.005
                    };
                    if self.input_state.is_key_down(VirtualKeyCode::A) {
                        self.game_state.camera.move_left(speed)
                    }
                    if self.input_state.is_key_down(VirtualKeyCode::D) {
                        self.game_state.camera.move_right(speed)
                    }
                    if self.input_state.is_key_down(VirtualKeyCode::S) {
                        self.game_state.camera.move_backward(speed)
                    }
                    if self.input_state.is_key_down(VirtualKeyCode::W) {
                        self.game_state.camera.move_forward(speed)
                    }
                    if self.input_state.is_key_down(VirtualKeyCode::Space) {
                        self.game_state.camera.move_up(speed)
                    }
                    if self.input_state.is_key_down(VirtualKeyCode::LControl) {
                        self.game_state.camera.move_down(speed)
                    }
                }
                _ => {}
            }
        });
    }
}

// render path, vulkan instance, vulkan device, framebuffers, swapchain
pub struct RendererState {
    render_path: RenderPath,
    /* global vulkan objects */
    instance: Arc<Instance>,
    device: Arc<Device>,
    graphical_queue: Arc<Queue>,
    /* swapchain related stuff */
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,
    framebuffers: SmallVec<[Arc<dyn FramebufferAbstract + Send + Sync>; 4]>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl RendererState {
    pub fn new(conf: RendererConfiguration, event_loop: &EventLoop<()>) -> Self {
        // we create vulkan instance object with extensions
        // required to create a windows which we will render to.
        let instance = Instance::new(
            Some(&app_info_from_cargo_toml!()),
            &vulkano_win::required_extensions(),
            Some("VK_LAYER_KHRONOS_validation"),
        )
        .expect("cannot create vulkan instance");

        let surface = WindowBuilder::new()
            .with_title("renderer")
            .with_inner_size(conf)
            .with_resizable(false)
            .build_vk_surface(event_loop, instance.clone())
            .expect("cannot create window");

        surface.window().set_cursor_grab(true).unwrap();
        surface.window().set_cursor_visible(false);

        let physical = PhysicalDevice::enumerate(&instance)
            .nth(conf.gpu)
            .expect("cannot find requested gpu");

        let graphical_queue_family = physical
            .queue_families()
            .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap())
            .expect("couldn't find a graphical queue family that's supported by surface");

        let (device, mut queues) = Device::new(
            physical,
            physical.supported_features(),
            &DeviceExtensions::supported_by_device(physical),
            [(graphical_queue_family, 0.5)].iter().cloned(),
        )
        .expect("cannot create virtual device");

        let graphical_queue = queues.next().expect("no queue was created");

        let caps = surface
            .capabilities(device.physical_device())
            .expect("cannot get surface capabilities");

        let dimensions = caps.current_extent.unwrap_or(caps.max_image_extent);
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();

        // to render color correctly and compute in linear color space we must
        // request the vulkan explicitly. here we choose a first swapchain format
        // that has sRGB non-linear color space.
        let format = caps
            .supported_formats
            .iter()
            .find(|(f, _)| *f == Format::B8G8R8A8Srgb)
            .map(|(f, _)| *f)
            .expect("cannot find srgb non-linear color space format!");

        // we prefer mailbox as it give less latency but fall back to
        // fifo as it should be supported on all configurations
        let present_mode = if caps.present_modes.mailbox {
            PresentMode::Mailbox
        } else {
            PresentMode::Fifo
        };

        let (swapchain, swapchain_images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            ImageUsage {
                color_attachment: true,
                transfer_destination: true,
                ..ImageUsage::none()
            },
            &graphical_queue,
            SurfaceTransform::Identity,
            alpha,
            present_mode,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .expect("cannot create swapchain");

        let render_path =
            RenderPath::new(graphical_queue.clone(), device.clone(), swapchain.clone());
        let framebuffers = swapchain_images
            .iter()
            .map(|it| render_path.create_framebuffer(it.clone()))
            .collect();

        RendererState {
            previous_frame_end: Some(Box::new(vulkano::sync::now(device.clone())) as Box<_>),
            render_path,
            instance,
            device,
            graphical_queue,
            swapchain,
            swapchain_images,
            framebuffers,
        }
    }

    fn render_frame(&mut self, game_state: &GameState) {
        // clean-up all resources from the previous frame
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // acquire next image. if the suboptimal is true we try to recreate the
        // swapchain after this frame rendering is done
        let (idx, suboptimal, fut) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(err) => panic!("{:?}", err), // device unplugged or window resized
            };

        let mut frame = Frame {
            render_path: &mut self.render_path,
            game_state,
            framebuffer: self.framebuffers[idx].clone(),
            builder: Some(
                AutoCommandBufferBuilder::primary_one_time_submit(
                    self.device.clone(),
                    self.graphical_queue.family(),
                )
                .unwrap(),
            ),
        };

        // let frame create and records it's command buffer(s).
        let primary_cb = frame.build();

        // wait for image to be available and then present drawn the image
        // to screen.
        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(fut)
            .then_execute(self.graphical_queue.clone(), primary_cb)
            .unwrap()
            .then_swapchain_present(self.graphical_queue.clone(), self.swapchain.clone(), idx)
            .then_signal_fence_and_flush();

        // depending on the completion state of the submitted command buffer either
        // return to continue to next frame, or report and error
        match future {
            Ok(future) => {
                self.previous_frame_end = Some(Box::new(future) as Box<_>);
            }
            Err(e) => {
                // device unplugged or window resized
                eprintln!("{:?}", e);
                self.previous_frame_end =
                    Some(Box::new(vulkano::sync::now(self.device.clone())) as Box<_>);
            }
        }

        if suboptimal {
            // todo: force recreate swap-chain
        }
    }
}

// long-lived global (vulkan) objects related to one render path (buffers, pipelines)
pub struct RenderPath {
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
    rock_material: Arc<Material>,
    white_material: Arc<Material>,
}

impl RenderPath {
    pub fn new(queue: Arc<Queue>, device: Arc<Device>, swapchain: Arc<Swapchain<Window>>) -> Self {
        let dims = swapchain.dimensions();
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dims[0] as f32, dims[1] as f32],
            depth_range: 0.0..1.0,
        };

        let (fst, fst_future) =
            create_full_screen_triangle(queue.clone()).expect("cannot create fst");

        // this example render path uses one render pass which renders all geometry and then
        // the skybox with one directional light without any shadows.
        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(
                device.clone(),
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
                        format: swapchain.format(),
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
        let vs = crate::shaders::basic_vert::Shader::load(device.clone()).unwrap();
        let fs = crate::shaders::basic_frag::Shader::load(device.clone()).unwrap();
        let sky_vs = crate::shaders::sky_vert::Shader::load(device.clone()).unwrap();
        let sky_fs = crate::shaders::sky_frag::Shader::load(device.clone()).unwrap();
        let tm_vs = crate::shaders::tonemap_vert::Shader::load(device.clone()).unwrap();
        let tm_fs = crate::shaders::tonemap_frag::Shader::load(device.clone()).unwrap();

        // create basic pipeline for drawing
        let geometry_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<BasicVertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .fragment_shader(fs.main_entry_point(), ())
                .triangle_list()
                .viewports(Some(viewport.clone()))
                .depth_stencil(DepthStencil::simple_depth_test())
                .cull_mode_back()
                .front_face_clockwise()
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .expect("cannot create graphics pipeline"),
        );

        let skybox_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<BasicVertex>()
                .vertex_shader(sky_vs.main_entry_point(), ())
                .fragment_shader(sky_fs.main_entry_point(), ())
                .triangle_list()
                .viewports(Some(viewport.clone()))
                .depth_stencil(DepthStencil {
                    depth_compare: Compare::LessOrEqual,
                    depth_write: false,
                    depth_bounds_test: DepthBounds::Disabled,
                    stencil_front: Default::default(),
                    stencil_back: Default::default(),
                })
                .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
                .build(device.clone())
                .expect("cannot create aky pipeline"),
        );

        let tonemap_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<PositionOnlyVertex>()
                .vertex_shader(tm_vs.main_entry_point(), ())
                .fragment_shader(tm_fs.main_entry_point(), ())
                .triangle_list()
                .viewports(Some(viewport.clone()))
                .render_pass(Subpass::from(render_pass.clone(), 2).unwrap())
                .build(device.clone())
                .expect("cannot build tonemap graphics pipeline"),
        );

        let depth_buffer = AttachmentImage::with_usage(
            device.clone(),
            swapchain.dimensions(),
            Format::D16Unorm,
            ImageUsage {
                transient_attachment: true,
                depth_stencil_attachment: true,
                ..ImageUsage::none()
            },
        )
        .expect("cannot create depth buffer");
        let hdr_buffer = AttachmentImage::with_usage(
            device.clone(),
            swapchain.dimensions(),
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
            PersistentDescriptorSet::start(
                tonemap_pipeline.descriptor_set_layout(0).unwrap().clone(),
            )
            .add_image(hdr_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
        );

        // TODO: remove from render path
        info!("loading geometry and image data...");
        let rock_mesh = load_geometry(
            queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1.bf",
        );
        let icosphere_mesh = load_geometry(
            queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\icosphere.bf",
        );
        let plane_mesh = load_geometry(
            queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\plane.bf",
        );
        let rock_albedo = load_image(
            queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1_Base_Color.bf",
        );
        let basic = load_image(
            queue.clone(),
            "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\basic.bf",
        );
        info!("data loaded!");

        let samplers = Samplers::new(device.clone()).unwrap();

        let rock_material = Material::new(
            geometry_pipeline.clone(),
            device.clone(),
            samplers.aniso_repeat.clone(),
            rock_albedo,
            MaterialData {
                albedo_color: vec3(1.0, 1.0, 1.0),
                alpha_cutoff: 0.0,
            },
        );

        let white_material = Material::new(
            geometry_pipeline.clone(),
            device.clone(),
            samplers.aniso_repeat.clone(),
            basic,
            MaterialData {
                albedo_color: vec3(1.0, 0.25, 0.0),
                alpha_cutoff: 0.0,
            },
        );

        Self {
            fst,
            sky_params: SkyParams::default(),
            render_pass: render_pass as Arc<_>,
            geometry_pipeline: geometry_pipeline as Arc<_>,
            skybox_pipeline: skybox_pipeline as Arc<_>,
            tonemap_pipeline: tonemap_pipeline as Arc<_>,
            tonemap_ds: tonemap_ds as Arc<_>,
            matrix_data_pool: CpuBufferPool::uniform_buffer(device.clone()),
            hosek_wilkie_sky_pool: CpuBufferPool::uniform_buffer(device.clone()),
            depth_buffer,
            hdr_buffer,
            //
            rock_mesh,
            icosphere_mesh,
            plane_mesh,
            rock_material,
            white_material,
        }
    }

    pub fn create_framebuffer(
        &self,
        final_image: Arc<SwapchainImage<Window>>,
    ) -> Arc<dyn FramebufferAbstract + Send + Sync> {
        Arc::new(
            Framebuffer::start(self.render_pass.clone())
                .add(self.hdr_buffer.clone())
                .unwrap()
                .add(final_image)
                .unwrap()
                .add(self.depth_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        )
    }
}

pub struct Frame<'r, 's> {
    render_path: &'r mut RenderPath,
    game_state: &'s GameState,
    framebuffer: Arc<dyn FramebufferAbstract + Send + Sync>,
    builder: Option<AutoCommandBufferBuilder>,
}

impl<'r, 's> Frame<'r, 's> {
    pub fn build(&mut self) -> AutoCommandBuffer {
        let no_dynamic_state = DynamicState::none();
        let path = &mut self.render_path;
        let state = self.game_state;

        // create descriptor sets
        let rock_transform = Transform {
            position: vec3(0.0, 1.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: vec3(0.03, 0.03, 0.03),
        };
        let ubo_rock = path
            .matrix_data_pool
            .next(MatrixData {
                model: rock_transform.into(),
                view: state.camera.view_matrix(),
                projection: state.camera.projection_matrix(),
            })
            .expect("cannot create next sub-buffer");
        let rock_ds = PersistentDescriptorSet::start(
            path.geometry_pipeline
                .descriptor_set_layout(1)
                .unwrap()
                .clone(),
        )
        .add_buffer(ubo_rock)
        .expect("cannot add ubo to pds set=1")
        .build()
        .expect("cannot build pds set=1");
        let plane_transform = Transform {
            position: vec3(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: vec3(30.0, 1.0, 30.0),
        };
        let ubo_plane = path
            .matrix_data_pool
            .next(MatrixData {
                model: plane_transform.into(),
                view: state.camera.view_matrix(),
                projection: state.camera.projection_matrix(),
            })
            .expect("cannot create next sub-buffer");
        let plane_ds = PersistentDescriptorSet::start(
            path.geometry_pipeline
                .descriptor_set_layout(1)
                .unwrap()
                .clone(),
        )
        .add_buffer(ubo_plane)
        .expect("cannot add ubo to pds set=1")
        .build()
        .expect("cannot build pds set=1");
        let params = make_hosek_wilkie_params(state.sun_dir, 2.0, vec3(0.0, 0.0, 0.0));
        let ubo_sky_hw = path.hosek_wilkie_sky_pool.next(params).unwrap();
        let sky_hw_params = PersistentDescriptorSet::start(
            path.skybox_pipeline
                .descriptor_set_layout(1)
                .unwrap()
                .clone(),
        )
        .add_buffer(ubo_sky_hw)
        .expect("cannot add ubo to pds set=1")
        .build()
        .expect("cannot build pds set=1");
        let ubo_sky = path
            .matrix_data_pool
            .next(MatrixData {
                model: Matrix4::from_scale(200.0),
                view: state.camera.view_matrix(),
                projection: state.camera.projection_matrix(),
            })
            .expect("cannot create next sub-buffer");

        let per_object_descriptor_set_sky = PersistentDescriptorSet::start(
            path.skybox_pipeline
                .descriptor_set_layout(0)
                .unwrap()
                .clone(),
        )
        .add_buffer(ubo_sky)
        .expect("cannot add ubo to pds set=1")
        .build()
        .expect("cannot build pds set=1");

        self.builder
            .take()
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
                path.geometry_pipeline.clone(),
                &no_dynamic_state,
                vec![path.rock_mesh.vertex_buffer.clone()],
                path.rock_mesh.index_buffer.clone(),
                (path.rock_material.descriptor_set.clone(), rock_ds),
                state.sun_dir,
            )
            .unwrap()
            .draw_indexed(
                path.geometry_pipeline.clone(),
                &no_dynamic_state,
                vec![path.plane_mesh.vertex_buffer.clone()],
                path.plane_mesh.index_buffer.clone(),
                (path.white_material.descriptor_set.clone(), plane_ds),
                state.sun_dir,
            )
            .unwrap()
            .next_subpass(false)
            .unwrap()
            .draw_indexed(
                path.skybox_pipeline.clone(),
                &no_dynamic_state,
                vec![path.icosphere_mesh.vertex_buffer.clone()],
                path.icosphere_mesh.index_buffer.clone(),
                (per_object_descriptor_set_sky, sky_hw_params),
                (state.camera.position, state.start.elapsed().as_secs_f32()),
            )
            .unwrap()
            .next_subpass(false)
            .unwrap()
            .draw_indexed(
                path.tonemap_pipeline.clone(),
                &no_dynamic_state,
                vec![path.fst.vertex_buffer.clone()],
                path.fst.index_buffer.clone(),
                path.tonemap_ds.clone(),
                (),
            )
            .unwrap()
            .end_render_pass()
            .unwrap()
            .build()
            .unwrap()
    }
}

pub struct Material {
    uniform_buffer: Arc<CpuAccessibleBuffer<MaterialData>>,
    // descriptor set that contains uniform objects that are related to this material instance
    descriptor_set: Arc<dyn DescriptorSet + Send + Sync>,
    data: MaterialData,
}

impl Material {
    pub fn new(
        geometry_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
        device: Arc<Device>,
        sampler: Arc<Sampler>,
        albedo: Arc<ImmutableImage<Format>>,
        data: MaterialData,
    ) -> Arc<Material> {
        let uniform_buffer =
            CpuAccessibleBuffer::from_data(device, BufferUsage::uniform_buffer(), false, data)
                .unwrap();
        let descriptor_set = Arc::new(
            PersistentDescriptorSet::start(
                geometry_pipeline.descriptor_set_layout(0).unwrap().clone(),
            )
            .add_sampled_image(albedo, sampler)
            .unwrap()
            .add_buffer(uniform_buffer.clone())
            .unwrap()
            .build()
            .expect("cannot build pds"),
        );

        Arc::new(Material {
            uniform_buffer,
            descriptor_set,
            data,
        })
    }

    pub fn update(&self, cmd: AutoCommandBufferBuilder) -> AutoCommandBufferBuilder {
        cmd.update_buffer(self.uniform_buffer.clone(), self.data)
            .unwrap()
    }
}
