//! Module containing all logic for PHR deferred rendering pipeline.

use crate::render::fxaa::FXAA;
use crate::render::hosek::HosekSky;
use crate::render::mcguire13::McGuire13;
use crate::render::pools::UniformBufferPool;
use crate::render::samplers::Samplers;
use crate::render::ubo::DirectionalLight;
use crate::render::vertex::{NormalMappedVertex, PositionOnlyVertex};
use crate::render::{
    descriptor_set_layout, FrameMatrixPool, FRAME_DATA_UBO_DESCRIPTOR_SET,
    LIGHTS_UBO_DESCRIPTOR_SET, SUBPASS_UBO_DESCRIPTOR_SET,
};
use crate::resources::mesh::{create_full_screen_triangle, IndexedMesh};
use log::info;
use std::sync::Arc;
use vulkano::descriptor_set::DescriptorSet;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, DeviceOwned, Queue};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageUsage, SwapchainImage};
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::render_pass::{FramebufferAbstract, FramebufferCreationError, Subpass};
use vulkano::swapchain::Swapchain;
use winit::window::Window;

// use `R16G16B16A16Sfloat` for high quality and `B10G11R11UfloatPack32` for less memory usage
const HDR_BUFFER_FORMAT: Format = Format::R32G32B32A32Sfloat;
const DEPTH_BUFFER_FORMAT: Format = Format::D32Sfloat;

/// Uniform buffer poll for light data.
pub type LightDataPool = UniformBufferPool<[DirectionalLight; 100]>;

/// Long-lived objects & buffers that **do not** change when resolution
/// changes.
pub struct PBRDeffered {
    pub render_pass: Arc<RenderPass>,
    pub samplers: Samplers,
    pub lights_buffer_pool: LightDataPool,
    pub fst: Arc<IndexedMesh<PositionOnlyVertex, u16>>,
    pub buffers: Buffers,
    pub sky: HosekSky,
    pub fxaa: FXAA,
}

/// Long-lived objects & buffers that **do** change when resolution changes.
pub struct Buffers {
    pub transparency: McGuire13,

    pub hdr_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    pub gbuffer1: Arc<ImageView<Arc<AttachmentImage>>>,
    pub gbuffer2: Arc<ImageView<Arc<AttachmentImage>>>,
    pub gbuffer3: Arc<ImageView<Arc<AttachmentImage>>>,
    pub depth_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    pub ldr_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    pub main_framebuffer: Arc<dyn FramebufferAbstract + Send + Sync>,

    pub geometry_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub lighting_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub tonemap_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    // subpass descriptor sets dependant on buffers
    pub tonemap_ds: Arc<dyn DescriptorSet + Send + Sync>,
    pub lighting_gbuffer_ds: Arc<dyn DescriptorSet + Send + Sync>,

    pub geometry_frame_matrix_pool: FrameMatrixPool,
    pub lights_frame_matrix_pool: FrameMatrixPool,
    pub transparency_frame_matrix_pool: FrameMatrixPool,
}

// create various buffers dependant on the resolution with this
// simple & useful macro
macro_rules! buffer {
    ($device:tt, $dims:tt, $name:tt, $format:expr) => {
        buffer!($device, $dims, $name, $format, ImageUsage::none())
    };
    ($device:tt, $dims:tt, $name:tt, $format:expr, $usage:expr) => {{
        let x = AttachmentImage::with_usage(
            ($device).clone(),
            $dims,
            $format,
            ImageUsage {
                transient_attachment: true,
                input_attachment: true,
                ..$usage
            },
        )
        .expect(&format!("cannot create buffer {}", stringify!($format)));
        // device.set_object_name(&x, cstr::cstr!($name));
        ImageView::new(x).ok().unwrap()
    }};
}

impl Buffers {
    fn new(render_pass: Arc<RenderPass>, device: Arc<Device>, dims: [u32; 2]) -> Self {
        // we create required shaders for all graphical pipelines we use in this
        // render pass from precompiled (embedded) spri-v binary data from soruces.
        let vs =
            crate::render::shaders::vs_deferred_geometry::Shader::load(device.clone()).unwrap();
        let fs =
            crate::render::shaders::fs_deferred_geometry::Shader::load(device.clone()).unwrap();
        let tm_vs = crate::render::shaders::vs_passtrough::Shader::load(device.clone()).unwrap();
        let tm_fs = crate::render::shaders::fs_tonemap::Shader::load(device.clone()).unwrap();
        let dl_fs =
            crate::render::shaders::fs_deferred_lighting::Shader::load(device.clone()).unwrap();

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
                .render_pass(Subpass::from(render_pass.clone(), 5).unwrap())
                .build(device.clone())
                .expect("cannot build tonemap graphics pipeline"),
        );

        let depth_buffer = buffer!(
            device,
            dims,
            "Depth buffer",
            DEPTH_BUFFER_FORMAT,
            ImageUsage::depth_stencil_attachment()
        );
        let hdr_buffer = buffer!(device, dims, "HDR Buffer", HDR_BUFFER_FORMAT);
        let gbuffer1 = buffer!(device, dims, "GBuffer 1", Format::A2B10G10R10UnormPack32);
        let gbuffer2 = buffer!(device, dims, "GBuffer 2", Format::R8G8B8A8Unorm);
        let gbuffer3 = buffer!(device, dims, "GBuffer 3", Format::R8G8B8A8Unorm);
        let ldr_buffer = AttachmentImage::with_usage(
            device.clone(),
            dims,
            Format::B10G11R11UfloatPack32,
            ImageUsage {
                input_attachment: true,
                sampled: true,
                ..ImageUsage::none()
            },
        )
        .expect(&format!("cannot create buffer {}", stringify!($format)));
        // device.set_object_name(&ldr_buffer, cstr::cstr!("LDR Buffer"));
        let ldr_buffer = ImageView::new(ldr_buffer).ok().unwrap();

        // create transparency buffers
        let transparency = McGuire13::new(
            device.clone(),
            Subpass::from(render_pass.clone(), 3).unwrap(),
            Subpass::from(render_pass.clone(), 4).unwrap(),
            dims,
        );

        let framebuffer = Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(gbuffer1.clone())
                .expect("cannot add attachment to framebuffer")
                .add(gbuffer2.clone())
                .expect("cannot add attachment to framebuffer")
                .add(gbuffer3.clone())
                .expect("cannot add attachment to framebuffer")
                .add(depth_buffer.clone())
                .expect("cannot add attachment to framebuffer")
                .add(hdr_buffer.clone())
                .expect("cannot add attachment to framebuffer")
                .add(ldr_buffer.clone())
                .expect("cannot add attachment to framebuffer")
                .add(transparency.accumulation.clone())
                .expect("cannot add attachment to framebuffer")
                .add(transparency.revealage.clone())
                .expect("cannot add attachment to framebuffer")
                .build()
                .expect("cannot build framebuffer"),
        );

        // create persistent descriptor sets that contains bindings to
        // buffers used in subpasses
        let tonemap_descriptor_set = Arc::new(
            PersistentDescriptorSet::start(descriptor_set_layout(tonemap_pipeline.layout(), 0))
                .add_image(hdr_buffer.clone())
                .unwrap()
                .build()
                .unwrap(),
        );
        let lighting_gbuffer_ds = Arc::new(
            PersistentDescriptorSet::start(descriptor_set_layout(
                lighting_pipeline.layout(),
                SUBPASS_UBO_DESCRIPTOR_SET,
            ))
            .add_image(gbuffer1.clone())
            .unwrap()
            .add_image(gbuffer2.clone())
            .unwrap()
            .add_image(gbuffer3.clone())
            .unwrap()
            .add_image(depth_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
        );

        Self {
            geometry_frame_matrix_pool: FrameMatrixPool::new(
                device.clone(),
                descriptor_set_layout(geometry_pipeline.layout(), FRAME_DATA_UBO_DESCRIPTOR_SET),
            ),
            lights_frame_matrix_pool: FrameMatrixPool::new(
                device.clone(),
                descriptor_set_layout(lighting_pipeline.layout(), FRAME_DATA_UBO_DESCRIPTOR_SET),
            ),
            transparency_frame_matrix_pool: FrameMatrixPool::new(
                device,
                descriptor_set_layout(
                    transparency.accumulation_pipeline.layout(),
                    FRAME_DATA_UBO_DESCRIPTOR_SET,
                ),
            ),
            geometry_pipeline: geometry_pipeline as Arc<_>,
            tonemap_pipeline: tonemap_pipeline as Arc<_>,
            tonemap_ds: tonemap_descriptor_set as Arc<_>,
            lighting_pipeline: lighting_pipeline as Arc<_>,
            lighting_gbuffer_ds: lighting_gbuffer_ds as Arc<_>,
            main_framebuffer: framebuffer as Arc<_>,
            transparency,
            depth_buffer,
            gbuffer1,
            gbuffer2,
            gbuffer3,
            hdr_buffer,
            ldr_buffer,
        }
    }

    pub fn dimensions_changed(&mut self, render_pass: Arc<RenderPass>, dims: [u32; 2]) {
        info!("Dimensions changed to {:?}. Recreating buffers.", dims);
        let device = render_pass.device().clone();
        let depth_buffer = buffer!(
            device,
            dims,
            "Depth buffer",
            DEPTH_BUFFER_FORMAT,
            ImageUsage::depth_stencil_attachment()
        );
        let hdr_buffer = buffer!(device, dims, "HDR Buffer", HDR_BUFFER_FORMAT);
        let gbuffer1 = buffer!(device, dims, "GBuffer 1", Format::A2B10G10R10UnormPack32);
        let gbuffer2 = buffer!(device, dims, "GBuffer 2", Format::R8G8B8A8Unorm);
        let gbuffer3 = buffer!(device, dims, "GBuffer 3", Format::R8G8B8A8Unorm);
        let ldr_buffer = AttachmentImage::with_usage(
            device.clone(),
            dims,
            Format::B10G11R11UfloatPack32,
            ImageUsage {
                input_attachment: true,
                sampled: true,
                ..ImageUsage::none()
            },
        )
        .expect(&format!("cannot create buffer {}", stringify!($format)));
        let ldr_buffer = ImageView::new(ldr_buffer).ok().unwrap();

        self.depth_buffer = depth_buffer;
        self.hdr_buffer = hdr_buffer;
        self.gbuffer1 = gbuffer1;
        self.gbuffer2 = gbuffer2;
        self.gbuffer3 = gbuffer3;
        self.ldr_buffer = ldr_buffer;

        self.transparency.dimensions_changed(dims);

        self.tonemap_ds = Arc::new(
            PersistentDescriptorSet::start(descriptor_set_layout(
                self.tonemap_pipeline.layout(),
                0,
            ))
            .add_image(self.hdr_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
        );
        self.lighting_gbuffer_ds = Arc::new(
            PersistentDescriptorSet::start(descriptor_set_layout(
                self.lighting_pipeline.layout(),
                SUBPASS_UBO_DESCRIPTOR_SET,
            ))
            .add_image(self.gbuffer1.clone())
            .unwrap()
            .add_image(self.gbuffer2.clone())
            .unwrap()
            .add_image(self.gbuffer3.clone())
            .unwrap()
            .add_image(self.depth_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
        );
        self.main_framebuffer = Arc::new(
            Framebuffer::start(render_pass)
                .add(self.gbuffer1.clone())
                .expect("cannot add attachment to framebuffer")
                .add(self.gbuffer2.clone())
                .expect("cannot add attachment to framebuffer")
                .add(self.gbuffer3.clone())
                .expect("cannot add attachment to framebuffer")
                .add(self.depth_buffer.clone())
                .expect("cannot add attachment to framebuffer")
                .add(self.hdr_buffer.clone())
                .expect("cannot add attachment to framebuffer")
                .add(self.ldr_buffer.clone())
                .expect("cannot add attachment to framebuffer")
                .add(self.transparency.accumulation.clone())
                .expect("cannot add attachment to framebuffer")
                .add(self.transparency.revealage.clone())
                .expect("cannot add attachment to framebuffer")
                .build()
                .expect("cannot build framebuffer"),
        );
    }
}

impl PBRDeffered {
    pub fn new(queue: Arc<Queue>, device: Arc<Device>, swapchain: Arc<Swapchain<Window>>) -> Self {
        // first we generate some useful resources on the fly
        let (fst, _) = create_full_screen_triangle(queue.clone()).expect("cannot create fst");

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
                        format: DEPTH_BUFFER_FORMAT,
                        samples: 1,
                    },
                    hdr: {
                        load: Clear,
                        store: DontCare,
                        format: HDR_BUFFER_FORMAT,
                        samples: 1,
                    },
                    ldr: {
                        load: DontCare,
                        store: Store,
                        format: Format::B10G11R11UfloatPack32,
                        samples: 1,
                    },
                    trans_accum: {
                        load: Clear,
                        store: DontCare,
                        format: crate::render::mcguire13::ACCUMULATION_BUFFER_FORMAT,
                        samples: 1,
                    },
                    trans_reveal: {
                        load: Clear,
                        store: DontCare,
                        format: crate::render::mcguire13::REVEALAGE_BUFFER_FORMAT,
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
                        color: [trans_accum, trans_reveal],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                        color: [hdr],
                        depth_stencil: {depth},
                        input: [trans_accum, trans_reveal]
                    },
                    {
                         color: [ldr],
                         depth_stencil: {},
                         input: [hdr]
                    }
                ]
            )
            .expect("cannot create render pass"),
        );

        let samplers = Samplers::new(device.clone()).unwrap();
        let buffers = Buffers::new(render_pass.clone(), device.clone(), swapchain.dimensions());
        let sky = HosekSky::new(queue.clone(), render_pass.clone(), device.clone());

        Self {
            fst,
            render_pass: render_pass as Arc<_>,
            lights_buffer_pool: LightDataPool::new(
                device.clone(),
                buffers
                    .lighting_pipeline
                    .layout()
                    .descriptor_set_layouts()
                    .get(LIGHTS_UBO_DESCRIPTOR_SET)
                    .unwrap()
                    .clone(),
            ),
            fxaa: FXAA::new(
                queue.clone(),
                device.clone(),
                swapchain.format(),
                buffers.ldr_buffer.clone(),
            ),
            buffers,
            sky,
            samplers,
        }
    }

    pub fn create_framebuffer(
        &self,
        final_image: Arc<ImageView<Arc<SwapchainImage<Window>>>>,
    ) -> Result<Arc<dyn FramebufferAbstract + Send + Sync>, FramebufferCreationError> {
        self.fxaa.create_framebuffer(final_image)
    }

    pub fn dimensions_changed(&mut self, dimensions: [u32; 2]) {
        self.buffers
            .dimensions_changed(self.render_pass.clone(), dimensions);
        self.fxaa
            .recreate_descriptor(self.buffers.ldr_buffer.clone());
    }
}
