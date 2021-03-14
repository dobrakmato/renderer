//! Fast approximate anti-aliasing.

use crate::render::descriptor_set_layout;
use crate::render::vertex::PositionOnlyVertex;
use crate::resources::mesh::{create_full_screen_triangle, IndexedMesh};
use std::sync::Arc;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::DescriptorSet;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::{
    FramebufferAbstract, FramebufferCreationError, RenderPassAbstract, Subpass,
};
use vulkano::image::{AttachmentImage, SwapchainImage};
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineAbstract};
use vulkano::sampler::Sampler;
use winit::window::Window;

pub mod shaders {
    pub mod fragment {
        const X: &str = include_str!("../../../shaders/fs_fxaa.glsl");
        vulkano_shaders::shader! {
            ty: "fragment",
            path: "shaders/fs_fxaa.glsl"
        }
    }
}

const FXAA_DESCRIPTOR_SET: usize = 0;

pub struct FXAA {
    pub render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    pub pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub fst: Arc<IndexedMesh<PositionOnlyVertex, u16>>,
    pub ldr_buffer_ds: Arc<dyn DescriptorSet + Send + Sync>,
}

impl FXAA {
    pub fn new(
        queue: Arc<Queue>,
        device: Arc<Device>,
        swapchain_format: Format,
        ldr_buffer: Arc<AttachmentImage>,
        sampler: Arc<Sampler>,
    ) -> Self {
        // first we generate some useful resources on the fly
        let (fst, _) = create_full_screen_triangle(queue.clone()).expect("cannot create fst");

        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(
                device.clone(),
                attachments: {
                    final_color: {
                        load: DontCare,
                        store: Store,
                        format: swapchain_format,
                        samples: 1,
                    }
                },
                passes: [
                    {
                         color: [final_color],
                         depth_stencil: {},
                         input: []
                    }
                ]
            )
            .expect("cannot create render pass for fxaa"),
        );

        let vs = crate::render::shaders::vs_passtrough::Shader::load(device.clone()).unwrap();
        let fs = crate::render::fxaa::shaders::fragment::Shader::load(device.clone()).unwrap();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<PositionOnlyVertex>()
                .vertex_shader(vs.main_entry_point(), ())
                .fragment_shader(fs.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .depth_stencil(DepthStencil::disabled())
                .cull_mode_back()
                .front_face_clockwise()
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .expect("cannot create graphics pipeline"),
        );

        let ldr_buffer_ds = Arc::new(
            PersistentDescriptorSet::start(descriptor_set_layout(&pipeline, FXAA_DESCRIPTOR_SET))
                .add_sampled_image(ldr_buffer, sampler)
                .unwrap()
                .build()
                .unwrap(),
        );

        Self {
            fst,
            pipeline,
            render_pass,
            ldr_buffer_ds: ldr_buffer_ds as Arc<_>,
        }
    }

    pub fn create_framebuffer(
        &self,
        final_image: Arc<SwapchainImage<Window>>,
    ) -> Result<Arc<dyn FramebufferAbstract + Send + Sync>, FramebufferCreationError> {
        Ok(Arc::new(
            Framebuffer::start(self.render_pass.clone())
                .add(final_image)?
                .build()?,
        ))
    }
}
