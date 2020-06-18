//! Objects & procedures related to rendering.

use crate::assets::lookup;
use crate::assets::Storage;
use crate::camera::Camera;
use crate::hosek::make_hosek_wilkie_params;
use crate::render::ubo::{DirectionalLight, FrameMatrixData, HosekWilkieParams, ObjectMatrixData};
use crate::render::vertex::{NormalMappedVertex, PositionOnlyVertex};
use crate::resources::mesh::{create_full_screen_triangle, create_mesh, IndexedMesh};
use crate::samplers::Samplers;
use crate::{GameState, RendererConfiguration};
use cgmath::{vec3, SquareMatrix, Vector3, Zero};
use smallvec::SmallVec;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::{DescriptorSet, PipelineLayoutAbstract};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::framebuffer::{Framebuffer, FramebufferCreationError, Subpass};
use vulkano::framebuffer::{FramebufferAbstract, RenderPassAbstract};
use vulkano::image::{
    AttachmentImage, Dimensions, ImageCreationError, ImageUsage, ImmutableImage, SwapchainImage,
};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::depth_stencil::{Compare, DepthBounds, DepthStencil};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::swapchain::{
    ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain,
    SwapchainCreationError,
};
use vulkano::sync::GpuFuture;
use vulkano::{app_info_from_cargo_toml, swapchain};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::Size;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

// consts to descriptor set binding indices
pub const FRAME_DATA_UBO_DESCRIPTOR_SET: usize = 0;
pub const OBJECT_DATA_UBO_DESCRIPTOR_SET: usize = 2;
pub const SUBPASS_UBO_DESCRIPTOR_SET: usize = 1;
pub const LIGHTS_UBO_DESCRIPTOR_SET: usize = 2;
pub const SKY_DATA_UBO_DESCRIPTOR_SET: usize = 1;

pub mod object;
pub mod transform;
pub mod ubo;
pub mod vertex;

// global vulkan object not related to one render path
pub struct VulkanState {
    device: Arc<Device>,
    surface: Arc<Surface<Window>>,
    graphical_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,
}

impl VulkanState {
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
            .with_resizable(true)
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

        let transfer_queue_family = physical
            .queue_families()
            .find(|&q| q.explicitly_supports_transfers())
            .expect("cannot find explicit transfer queue");

        let (device, mut queues) = Device::new(
            physical,
            physical.supported_features(),
            &DeviceExtensions::supported_by_device(physical),
            [(graphical_queue_family, 0.5), (transfer_queue_family, 0.5)]
                .iter()
                .cloned(),
        )
        .expect("cannot create virtual device");

        let graphical_queue = queues.next().expect("no queue was created");
        let transfer_queue = queues.next().expect("no transfer queue was created");

        Self {
            device,
            surface,
            graphical_queue,
            transfer_queue,
        }
    }

    #[inline]
    pub fn surface(&self) -> Arc<Surface<Window>> {
        self.surface.clone()
    }

    #[inline]
    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    pub fn transfer_queue(&self) -> Arc<Queue> {
        self.transfer_queue.clone()
    }
}

// render path, vulkan instance, vulkan device, framebuffers, swapchain
pub struct RendererState {
    pub render_path: RenderPath,
    device: Arc<Device>,
    pub graphical_queue: Arc<Queue>,
    /* swapchain related stuff */
    swapchain: Arc<Swapchain<Window>>,
    framebuffers: SmallVec<[Arc<dyn FramebufferAbstract + Send + Sync>; 4]>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl RendererState {
    pub fn new(vulkan: &VulkanState, assets: &Storage) -> Self {
        let surface = vulkan.surface.clone();
        let device = vulkan.device.clone();
        let graphical_queue = vulkan.graphical_queue.clone();

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

        let render_path = RenderPath::new(
            graphical_queue.clone(),
            device.clone(),
            swapchain.clone(),
            assets,
        );

        let framebuffers = match swapchain_images
            .iter()
            .map(|it| render_path.create_framebuffer(it.clone()))
            .collect()
        {
            Ok(t) => t,
            Err(e) => panic!("cannot create framebuffers: {}", e),
        };

        RendererState {
            previous_frame_end: Some(Box::new(vulkano::sync::now(device.clone())) as Box<_>),
            render_path,
            swapchain,
            framebuffers,
            device,
            graphical_queue,
        }
    }

    pub fn set_window_size<S: Into<Size>>(&self, size: S) {
        self.swapchain.surface().window().set_inner_size(size)
    }

    pub fn render_frame(&mut self, game_state: &GameState) {
        // clean-up all resources from the previous frame
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // acquire next image. if the suboptimal is true we try to recreate the
        // swapchain after this frame rendering is done
        let (idx, suboptimal, fut) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(_) => {
                    self.recreate_swapchain();
                    return;
                }
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
            self.recreate_swapchain();
        }
    }

    fn recreate_swapchain(&mut self) {
        let dimensions: [u32; 2] = self.swapchain.surface().window().inner_size().into();
        let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimensions(dimensions)
        {
            Ok(r) => r,
            // This error tends to happen when the user is manually resizing the window.
            // Simply restarting the loop is the easiest way to fix this issue.
            Err(SwapchainCreationError::UnsupportedDimensions) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.render_path.recreate_buffers(dimensions);

        let new_framebuffers = new_images
            .iter()
            .map(|x| self.render_path.create_framebuffer(x.clone()))
            .map(|x| x.expect("cannot create framebuffer"))
            .collect();

        self.swapchain = new_swapchain;
        self.framebuffers = new_framebuffers;
    }
}

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

// long-lived global (vulkan) objects related to one render path (buffers, pipelines)
pub struct RenderPath {
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    pub buffers: RenderPathBuffers,
    pub samplers: Samplers,
    pub white_texture: Arc<ImmutableImage<Format>>,

    fst: Arc<IndexedMesh<PositionOnlyVertex, u16>>,
    frame_matrix_data: CpuBufferPool<FrameMatrixData>,
    hosek_wilkie_sky_pool: CpuBufferPool<HosekWilkieParams>,
    sky_mesh: Arc<IndexedMesh<NormalMappedVertex, u16>>,
}

// long-lived global buffers and data dependant on the render resolution
pub struct RenderPathBuffers {
    hdr_buffer: Arc<AttachmentImage>,
    geometry_buffer: GBuffer,
    depth_buffer: Arc<AttachmentImage>,
    // pipelines are dependant on the resolution
    pub geometry_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    lighting_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    skybox_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    tonemap_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    // constant descriptor sets
    tonemap_ds: Arc<dyn DescriptorSet + Send + Sync>,
    lighting_gbuffer_ds: Arc<dyn DescriptorSet + Send + Sync>,
    lights_buffer_pool: CpuBufferPool<[DirectionalLight; 1024]>,
}

impl RenderPathBuffers {
    fn new(
        render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
        device: Arc<Device>,
        dimensions: [u32; 2],
    ) -> Self {
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };

        // we create required shaders for all graphical pipelines we use in this
        // render pass from precompiled (embedded) spri-v binary data from soruces.
        let vs = crate::shaders::vs_deferred_geometry::Shader::load(device.clone()).unwrap();
        let fs = crate::shaders::fs_deferred_geometry::Shader::load(device.clone()).unwrap();
        let sky_vs = crate::shaders::sky_vert::Shader::load(device.clone()).unwrap();
        let sky_fs = crate::shaders::sky_frag::Shader::load(device.clone()).unwrap();
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
                .viewports(Some(viewport.clone()))
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
                .viewports(Some(viewport.clone()))
                .render_pass(Subpass::from(render_pass.clone(), 1).unwrap())
                .build(device.clone())
                .expect("cannot build tonemap graphics pipeline"),
        );

        let skybox_pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<NormalMappedVertex>()
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
                .render_pass(Subpass::from(render_pass.clone(), 2).unwrap())
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
            geometry_pipeline: geometry_pipeline as Arc<_>,
            skybox_pipeline: skybox_pipeline as Arc<_>,
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

impl RenderPath {
    pub fn new(
        queue: Arc<Queue>,
        device: Arc<Device>,
        swapchain: Arc<Swapchain<Window>>,
        assets: &Storage,
    ) -> Self {
        // first we generate some useful resources on the fly
        let (fst, _) = create_full_screen_triangle(queue.clone()).expect("cannot create fst");
        let (white_texture, _) = ImmutableImage::from_iter(
            [255u8; 4].iter().cloned(),
            Dimensions::Dim2d {
                width: 1,
                height: 1,
            },
            Format::R8G8B8A8Unorm,
            queue.clone(),
        )
        .expect("cannot create white texture");

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
        let (sky_mesh, _) = create_mesh(
            &assets.request_load(lookup("./icosphere.obj")).wait(),
            queue,
        )
        .unwrap();

        Self {
            fst,
            buffers: RenderPathBuffers::new(
                render_pass.clone(),
                device.clone(),
                swapchain.dimensions(),
            ),
            render_pass: render_pass as Arc<_>,
            frame_matrix_data: CpuBufferPool::uniform_buffer(device.clone()),
            hosek_wilkie_sky_pool: CpuBufferPool::uniform_buffer(device.clone()),
            sky_mesh,
            samplers,
            white_texture,
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
        let dims = [
            self.framebuffer.dimensions()[0] as f32,
            self.framebuffer.dimensions()[1] as f32,
        ];

        /* create FrameMatrixData (set=2) for this frame. */
        let view = self.game_state.camera.view_matrix();
        let projection = self.game_state.camera.projection_matrix();
        let frame_matrix_data = Arc::new(
            path.frame_matrix_data
                .next(FrameMatrixData {
                    inv_view: view.invert().unwrap(),
                    inv_projection: projection.invert().unwrap(),
                    view,
                    projection,
                })
                .expect("cannot take next buffer"),
        );

        // todo: remove duplicates
        let ds_frame_matrix_data_geometry = Arc::new(
            PersistentDescriptorSet::start(
                path.buffers
                    .geometry_pipeline
                    .descriptor_set_layout(FRAME_DATA_UBO_DESCRIPTOR_SET)
                    .unwrap()
                    .clone(),
            )
            .add_buffer(frame_matrix_data.clone())
            .expect("cannot add ubo to pds set=1")
            .build()
            .expect("cannot build pds set=1"),
        );
        let ds_frame_matrix_data_lighting = PersistentDescriptorSet::start(
            path.buffers
                .lighting_pipeline
                .descriptor_set_layout(FRAME_DATA_UBO_DESCRIPTOR_SET)
                .unwrap()
                .clone(),
        )
        .add_buffer(frame_matrix_data.clone())
        .expect("cannot add ubo to pds set=1")
        .build()
        .expect("cannot build pds set=1");
        let ds_frame_matrix_data_skybox = PersistentDescriptorSet::start(
            path.buffers
                .skybox_pipeline
                .descriptor_set_layout(FRAME_DATA_UBO_DESCRIPTOR_SET)
                .unwrap()
                .clone(),
        )
        .add_buffer(frame_matrix_data)
        .expect("cannot add ubo to pds set=1")
        .build()
        .expect("cannot build pds set=1");

        /* create HosekWilkieParams (sky params) for this frame. */
        let params = make_hosek_wilkie_params(
            state
                .directional_lights
                .get(0)
                .expect("need at least one directional light")
                .direction,
            2.0,
            vec3(0.0, 0.0, 0.0),
        );
        let ubo_sky_hw = path.hosek_wilkie_sky_pool.next(params).unwrap();
        let sky_hw_params = PersistentDescriptorSet::start(
            path.buffers
                .skybox_pipeline
                .descriptor_set_layout(SKY_DATA_UBO_DESCRIPTOR_SET)
                .unwrap()
                .clone(),
        )
        .add_buffer(ubo_sky_hw)
        .expect("cannot add ubo to pds set=1")
        .build()
        .expect("cannot build pds set=1");

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
            x.draw_indexed(
                &no_dynamic_state,
                ds_frame_matrix_data_geometry.clone(),
                &mut b,
            )
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
            &no_dynamic_state,
            vec![path.fst.vertex_buffer().clone()],
            path.fst.index_buffer().clone(),
            (
                ds_frame_matrix_data_lighting,
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
        b.draw_indexed(
            path.buffers.skybox_pipeline.clone(),
            &no_dynamic_state,
            vec![path.sky_mesh.vertex_buffer().clone()],
            path.sky_mesh.index_buffer().clone(),
            (ds_frame_matrix_data_skybox, sky_hw_params),
            (state.camera.position, state.start.elapsed().as_secs_f32()),
        )
        .expect("cannot do skybox pass")
        .next_subpass(false)
        .unwrap();

        // 4. SUBPASS - Tonemap
        b.draw_indexed(
            path.buffers.tonemap_pipeline.clone(),
            &no_dynamic_state,
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
