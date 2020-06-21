use crate::render::hosek::HosekSky;
use crate::render::pools::UniformBufferPool;
use crate::render::ubo::DirectionalLight;
use crate::render::vertex::{NormalMappedVertex, PositionOnlyVertex};
use crate::render::{
    FrameMatrixPool, FRAME_DATA_UBO_DESCRIPTOR_SET, LIGHTS_UBO_DESCRIPTOR_SET,
    SUBPASS_UBO_DESCRIPTOR_SET,
};
use crate::resources::mesh::{create_full_screen_triangle, IndexedMesh};
use crate::samplers::Samplers;
use std::sync::Arc;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::{DescriptorSet, PipelineLayoutAbstract};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::{
    FramebufferAbstract, FramebufferCreationError, RenderPassAbstract, Subpass,
};
use vulkano::image::{AttachmentImage, ImageUsage, SwapchainImage};
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::swapchain::Swapchain;
use winit::window::Window;

pub type LightDataPool = UniformBufferPool<[DirectionalLight; 1024]>;

// long-lived global (vulkan) objects related to one render path (buffers, pipelines)
pub struct PBRDeffered {
    pub render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    pub samplers: Samplers,
    pub lights_buffer_pool: LightDataPool,
    pub fst: Arc<IndexedMesh<PositionOnlyVertex, u16>>,
    pub buffers: Buffers,
    pub sky: HosekSky,
}

// long-lived global buffers and data dependant on the render resolution
pub struct Buffers {
    pub hdr_buffer: Arc<AttachmentImage>,
    pub gbuffer1: Arc<AttachmentImage>,
    pub gbuffer2: Arc<AttachmentImage>,
    pub gbuffer3: Arc<AttachmentImage>,
    pub depth_buffer: Arc<AttachmentImage>,
    // pipelines are dependant on the viewport + buffers
    pub geometry_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub lighting_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub tonemap_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    // subpass descriptor sets dependant on buffers
    pub tonemap_ds: Arc<dyn DescriptorSet + Send + Sync>,
    pub lighting_gbuffer_ds: Arc<dyn DescriptorSet + Send + Sync>,
    pub geometry_frame_matrix_pool: FrameMatrixPool,
    pub lights_frame_matrix_pool: FrameMatrixPool,
}

impl Buffers {
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

        // create various buffers dependant on the resolution with this
        // simple & useful macro
        macro_rules! buffer {
            ($format:expr) => {
                buffer!($format, ImageUsage::none())
            };
            ($format:expr, $usage:expr) => {
                AttachmentImage::with_usage(
                    device.clone(),
                    dimensions,
                    $format,
                    ImageUsage {
                        transient_attachment: true,
                        input_attachment: true,
                        ..$usage
                    },
                )
                .map_err(|_| panic!("cannot create buffer {}", stringify!($format)))
                .unwrap()
            };
        }

        let depth_buffer = buffer!(Format::D16Unorm, ImageUsage::depth_stencil_attachment());
        let hdr_buffer = buffer!(Format::B10G11R11UfloatPack32);
        let gbuffer1 = buffer!(Format::A2B10G10R10UnormPack32);
        let gbuffer2 = buffer!(Format::R8G8B8A8Unorm);
        let gbuffer3 = buffer!(Format::R8G8B8A8Unorm);

        // create persistent descriptor sets that contains bindings to
        // buffers used in subpasses
        let tonemap_descriptor_set = Arc::new(
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
            tonemap_ds: tonemap_descriptor_set as Arc<_>,
            lighting_pipeline: lighting_pipeline as Arc<_>,
            lighting_gbuffer_ds: lighting_gbuffer_ds as Arc<_>,
            depth_buffer,
            gbuffer1,
            gbuffer2,
            gbuffer3,
            hdr_buffer,
        }
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

        let buffers = Buffers::new(render_pass.clone(), device.clone(), swapchain.dimensions());
        let sky = HosekSky::new(queue, render_pass.clone(), device.clone());

        Self {
            fst,
            render_pass: render_pass as Arc<_>,
            lights_buffer_pool: LightDataPool::new(
                device,
                buffers
                    .lighting_pipeline
                    .descriptor_set_layout(LIGHTS_UBO_DESCRIPTOR_SET)
                    .unwrap()
                    .clone(),
            ),
            buffers,
            sky,
            samplers,
        }
    }

    pub fn create_framebuffer(
        &self,
        final_image: Arc<SwapchainImage<Window>>,
    ) -> Result<Arc<dyn FramebufferAbstract + Send + Sync>, FramebufferCreationError> {
        Ok(Arc::new(
            Framebuffer::start(self.render_pass.clone())
                .add(self.buffers.gbuffer1.clone())?
                .add(self.buffers.gbuffer2.clone())?
                .add(self.buffers.gbuffer3.clone())?
                .add(self.buffers.depth_buffer.clone())?
                .add(self.buffers.hdr_buffer.clone())?
                .add(final_image)?
                .build()?,
        ))
    }

    pub fn recreate_buffers(&mut self, dimensions: [u32; 2]) {
        self.buffers = Buffers::new(
            self.render_pass.clone(),
            self.render_pass.device().clone(),
            dimensions,
        )
    }
}
