use crate::render::descriptor_set_layout;
use crate::render::mcguire13::shaders::{
    get_or_load_acc_fragment_shader, get_or_load_acc_vertex_shader,
    get_or_load_resolve_fragment_shader,
};
use crate::render::vertex::{NormalMappedVertex, PositionOnlyVertex};
use std::sync::Arc;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::DescriptorSet;
use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageUsage};
use vulkano::pipeline::blend::{AttachmentBlend, BlendFactor, BlendOp};
use vulkano::pipeline::depth_stencil::{Compare, DepthBounds, DepthStencil};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::GraphicsPipelineAbstract;
use vulkano::render_pass::Subpass;

pub mod shaders;

pub const ACCUMULATION_BUFFER_FORMAT: Format = Format::R16G16B16A16Sfloat;
pub const REVEALAGE_BUFFER_FORMAT: Format = Format::R16Sfloat;

// Integrate to you render pass
pub struct McGuire13 {
    device: Arc<Device>,
    // buffers
    pub accumulation: Arc<ImageView<Arc<AttachmentImage>>>,
    pub revealage: Arc<ImageView<Arc<AttachmentImage>>>,
    // pipelines for two passes
    pub accumulation_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub resolve_pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,

    // descriptor sets
    pub resolve_ds: Arc<dyn DescriptorSet + Send + Sync>,
}

impl McGuire13 {
    pub fn new(
        device: Arc<Device>,
        accum_subpass: Subpass,
        resolve_subpass: Subpass,
        dims: [u32; 2],
    ) -> Self {
        let accumulation = make_buffer(device.clone(), ACCUMULATION_BUFFER_FORMAT, dims);
        let revealage = make_buffer(device.clone(), REVEALAGE_BUFFER_FORMAT, dims);

        let accum_vs = get_or_load_acc_vertex_shader(device.clone());
        let accum_fs = get_or_load_acc_fragment_shader(device.clone());

        let accumulation_pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<NormalMappedVertex>()
            .vertex_shader(accum_vs.main_entry_point(), ())
            .fragment_shader(accum_fs.main_entry_point(), ())
            .triangle_list()
            .blend_individual(vec![
                AttachmentBlend {
                    enabled: true,
                    color_op: BlendOp::Add,
                    color_source: BlendFactor::One,
                    color_destination: BlendFactor::One,
                    alpha_op: BlendOp::Add,
                    alpha_source: BlendFactor::One,
                    alpha_destination: BlendFactor::One,
                    mask_red: true,
                    mask_green: true,
                    mask_blue: true,
                    mask_alpha: true,
                },
                AttachmentBlend {
                    enabled: true,
                    color_op: BlendOp::Add,
                    color_source: BlendFactor::Zero,
                    color_destination: BlendFactor::OneMinusSrcAlpha,
                    alpha_op: BlendOp::Add,
                    alpha_source: BlendFactor::Zero,
                    alpha_destination: BlendFactor::OneMinusSrcAlpha,
                    mask_red: true,
                    mask_green: true,
                    mask_blue: true,
                    mask_alpha: true,
                },
            ]) // per target blending setup
            .cull_mode_back()
            .front_face_clockwise()
            .viewports_dynamic_scissors_irrelevant(1)
            .depth_stencil(DepthStencil {
                depth_write: false,
                depth_compare: Compare::Less,
                depth_bounds_test: DepthBounds::Disabled,
                stencil_front: Default::default(),
                stencil_back: Default::default(),
            })
            .render_pass(accum_subpass)
            .build(device.clone())
            .expect("cannot build transparency graphics pipeline");

        let resolve_vs =
            crate::render::shaders::vs_passtrough::Shader::load(device.clone()).unwrap();
        let resolve_fs = get_or_load_resolve_fragment_shader(device.clone());

        let resolve_pipeline = GraphicsPipeline::start()
            .vertex_input_single_buffer::<PositionOnlyVertex>()
            .vertex_shader(resolve_vs.main_entry_point(), ())
            .fragment_shader(resolve_fs.main_entry_point(), ())
            .triangle_list()
            .blend_collective(AttachmentBlend {
                enabled: true,
                color_op: BlendOp::Add,
                color_source: BlendFactor::OneMinusSrcAlpha,
                color_destination: BlendFactor::SrcAlpha,
                alpha_op: BlendOp::Add,
                alpha_source: BlendFactor::OneMinusSrcAlpha,
                alpha_destination: BlendFactor::SrcAlpha,
                mask_red: true,
                mask_green: true,
                mask_blue: true,
                mask_alpha: true,
            })
            .viewports_dynamic_scissors_irrelevant(1)
            .render_pass(resolve_subpass)
            .build(device.clone())
            .expect("cannot build transparency graphics pipeline");

        let resolve_ds =
            PersistentDescriptorSet::start(descriptor_set_layout(&resolve_pipeline, 0))
                .add_image(accumulation.clone())
                .unwrap()
                .add_image(revealage.clone())
                .unwrap()
                .build()
                .unwrap();

        Self {
            device,
            accumulation,
            revealage,
            resolve_ds: Arc::new(resolve_ds),
            accumulation_pipeline: Arc::new(accumulation_pipeline),
            resolve_pipeline: Arc::new(resolve_pipeline),
        }
    }

    pub fn dimensions_changed(&mut self, new_dimensions: [u32; 2]) {
        self.accumulation = make_buffer(
            self.device.clone(),
            ACCUMULATION_BUFFER_FORMAT,
            new_dimensions,
        );
        self.revealage = make_buffer(self.device.clone(), REVEALAGE_BUFFER_FORMAT, new_dimensions);

        self.resolve_ds = Arc::new(
            PersistentDescriptorSet::start(descriptor_set_layout(&self.resolve_pipeline, 0))
                .add_image(self.accumulation.clone())
                .unwrap()
                .add_image(self.revealage.clone())
                .unwrap()
                .build()
                .unwrap(),
        );
    }
}

// creates a new float buffer for transparency
fn make_buffer(
    device: Arc<Device>,
    format: Format,
    dims: [u32; 2],
) -> Arc<ImageView<Arc<AttachmentImage>>> {
    let buffer = AttachmentImage::with_usage(
        device.clone(),
        dims,
        format,
        ImageUsage {
            input_attachment: true,
            ..ImageUsage::none()
        },
    )
    .expect(&format!("cannot create buffer {}", stringify!($format)));
    ImageView::new(buffer).expect("cannot create image view")
}
