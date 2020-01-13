use crate::image::ToVulkanFormat;
use crate::render::{BasicVertex, Window};
use bf::load_bf_from_bytes;
use cgmath::{vec3, Deg, Matrix4, PerspectiveFov, Point3};
use log::{error, info, warn};
use safe_transmute::{Error, TriviallyTransmutable};
use std::sync::Arc;
use std::time::Instant;
use vulkano::buffer::{BufferUsage, CpuBufferPool, ImmutableBuffer};
use vulkano::command_buffer::{
    AutoCommandBuffer, AutoCommandBufferBuilder, CommandBufferExecFuture, DynamicState,
};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, Queue};
use vulkano::format::ClearValue;
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::image::{AttachmentImage, Dimensions, ImageUsage, ImmutableImage};
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::sampler::Sampler;
use vulkano::sampler::{Filter, MipmapMode, SamplerAddressMode};
use vulkano::swapchain;
use vulkano::sync::{GpuFuture, NowFuture};
use winit::{Event, WindowEvent};

mod image;
mod render;

mod basic_vert {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/basic_vert.glsl");
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/basic_vert.glsl"
    }
}

mod basic_frag {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/basic_frag.glsl");
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/basic_frag.glsl"
    }
}

fn load_geometry(
    queue: Arc<Queue>,
    file: &str,
) -> (
    Arc<ImmutableBuffer<[BasicVertex]>>,
    Arc<ImmutableBuffer<[u16]>>,
) {
    let bytes = std::fs::read(file).expect("cannot read file");
    let file = load_bf_from_bytes(bytes.as_slice()).expect("cannot decode bf file");
    let geometry = file.try_to_geometry().expect("bf file is not geometry");

    // dummy Vecs to extend life-time of variables
    let mut vertex_vec = Vec::new();
    let mut index_vec = Vec::new();

    /// This function either just returns the transmuted slice
    /// or performs a copy if the data is misaligned.
    fn possible_non_zero_copy<'a, T: TriviallyTransmutable>(
        bytes: &'a [u8],
        possible_owner: &'a mut std::vec::Vec<T>,
    ) -> &'a [T] {
        match safe_transmute::transmute_many_pedantic::<T>(bytes) {
            Ok(safe) => safe,
            Err(Error::Unaligned(e)) => {
                error!(
                    "cannot zero-copy unaligned &[{:?}] data: {:?}",
                    std::any::type_name::<T>(),
                    e
                );
                *possible_owner = e.copy();
                possible_owner.as_slice()
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    let vertices = possible_non_zero_copy::<BasicVertex>(geometry.vertex_data, &mut vertex_vec);
    let indices = possible_non_zero_copy::<u16>(geometry.index_data, &mut index_vec);

    let (vertex_buffer, future) = ImmutableBuffer::from_iter(
        vertices.iter().cloned(),
        BufferUsage::vertex_buffer(),
        queue.clone(),
    )
    .expect("cannot allocate memory for vertex buffer");
    future.then_signal_fence_and_flush().ok();

    let (index_buffer, future) = ImmutableBuffer::from_iter(
        indices.iter().cloned(),
        BufferUsage::index_buffer(),
        queue.clone(),
    )
    .expect("cannot allocate memory for vertex buffer");
    future.then_signal_fence_and_flush().ok();

    (vertex_buffer, index_buffer)
}

fn main() {
    simple_logger::init().unwrap();

    #[cfg(debug_assertions)]
    warn!("this is a debug build. performance may hurt.");

    let mut app = Window::new();

    let queue = app.queues.get(0).unwrap();
    let swapchain_format = app.swapchain.format();

    info!("loading geometry and image data...");

    // upload vertex data to gpu
    let (vertex_buffer, index_buffer) = load_geometry(
        queue.clone(),
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1.bf",
    );

    let bytes = std::fs::read(
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1_Base_Color.bf",
    )
    .expect("cannot read file");
    let file = load_bf_from_bytes(bytes.as_slice()).expect("cannot decode bf file");
    let image = file.try_to_image().unwrap();
    let mipmap0 = &image.mipmap_data[0..(image.width as usize
        * image.height as usize
        * image.format.bits_per_pixel() as usize
        / 8)
        + 3078]; // todo: fix vulkano bug - wrong calculated size

    let (image, future) = ImmutableImage::from_iter(
        mipmap0.iter().cloned(),
        Dimensions::Dim2d {
            width: image.width as u32,
            height: image.height as u32,
        },
        image.format.to_vulkan_format(),
        queue.clone(),
    )
    .expect("cannot allocate image");
    future.then_signal_fence_and_flush().ok();

    info!("data loaded!");

    // create shaders on gpu from precompiled spir-v code
    let vs = basic_vert::Shader::load(app.device.clone()).unwrap();
    let fs = basic_frag::Shader::load(app.device.clone()).unwrap();

    // define a render pass object with one pass
    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(
            app.device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain_format,
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .unwrap(),
    );

    // create basic pipeline for drawing
    let dims = app.swapchain.dimensions();
    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<BasicVertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .fragment_shader(fs.main_entry_point(), ())
            .triangle_list()
            .viewports(
                [Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dims[0] as f32, dims[1] as f32],
                    depth_range: 0.0..1.0,
                }]
                .iter()
                .cloned(),
            )
            .depth_stencil(DepthStencil::simple_depth_test())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(app.device.clone())
            .expect("cannot create graphics pipeline"),
    );

    // create sampler
    let sampler = Sampler::new(
        app.device.clone(),
        Filter::Linear,
        Filter::Linear,
        MipmapMode::Linear,
        SamplerAddressMode::ClampToEdge,
        SamplerAddressMode::ClampToEdge,
        SamplerAddressMode::ClampToEdge,
        0.0,
        16.0,
        1.0,
        100.0,
    )
    .expect("cannot create sampler");

    // create descriptor set 0
    let descriptor_set0 = Arc::new(
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_sampled_image(image.clone(), sampler.clone())
            .expect("cannot add sampled image to descriptor set")
            .build()
            .expect("cannot build descriptor set"),
    );

    // create uniform buffer for descriptor set 1
    struct MatrixData(Matrix4<f32>, Matrix4<f32>, Matrix4<f32>);
    let ubo_matrix_data_pool = CpuBufferPool::<MatrixData>::uniform_buffer(app.device.clone());

    // create framebuffers for each swapchain image
    let framebuffers = app
        .swapchain_images
        .iter()
        .map(|image| {
            let depth = AttachmentImage::with_usage(
                app.device.clone(),
                dims,
                Format::D16Unorm,
                ImageUsage {
                    transient_attachment: true,
                    input_attachment: true,
                    ..ImageUsage::none()
                },
            )
            .unwrap();

            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .add(depth)
                    .unwrap()
                    .build()
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    // main game-loop
    let mut previous_frame_end: Box<dyn GpuFuture> =
        Box::new(vulkano::sync::now(app.device.clone()));
    let start = Instant::now();
    loop {
        previous_frame_end.cleanup_finished(); // todo: why is this here?

        // acquire next framebuffer to write to
        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(app.swapchain.clone(), None) {
                Ok(r) => r,
                Err(err) => panic!("{:?}", err), // device unplugged or window resized
            };
        let framebuffer = framebuffers[image_num].clone();

        // simple update: recalculate mvp for our object
        let scale = Matrix4::from_scale(0.03);
        let rotation = Matrix4::from_angle_y(Deg(start.elapsed().as_secs_f32() * 60.0));
        let translate = Matrix4::from_translation(vec3(0.0, 0.0, 0.0));
        let view = Matrix4::look_at(
            Point3::new(0.3, 0.3, 1.0),
            Point3::new(0.0, 0.0, 0.0),
            vec3(0.0, -1.0, 0.0),
        );
        let projection: Matrix4<f32> = PerspectiveFov {
            fovy: Deg(90.0).into(),
            aspect: 16.0 / 9.0,
            near: 0.01,
            far: 100.0,
        }
        .into();
        let mvp = MatrixData(translate * scale * rotation, view, projection);
        let ubo = ubo_matrix_data_pool
            .next(mvp)
            .expect("cannot create next sub-buffer");

        let per_object_descriptor_set = PersistentDescriptorSet::start(pipeline.clone(), 1)
            .add_buffer(ubo)
            .expect("cannot add ubo to pds set=1")
            .build()
            .expect("cannot build pds set=1");

        // start building the command buffer that will contain all
        // rendering commands for this frame.
        let command_buffer =
            AutoCommandBufferBuilder::primary_one_time_submit(app.device.clone(), queue.family())
                .unwrap()
                .begin_render_pass(
                    framebuffer,
                    false,
                    vec![
                        ClearValue::Float([0.0, 0.0, 0.0, 0.0]),
                        ClearValue::Depth(1.0),
                    ],
                )
                .unwrap()
                .draw_indexed(
                    pipeline.clone(),
                    &DynamicState::none(),
                    vertex_buffer.clone(),
                    index_buffer.clone(),
                    (descriptor_set0.clone(), per_object_descriptor_set),
                    start.elapsed().as_secs_f32(),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();

        // wait for image to be available and then present drawn the image
        // to screen.
        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(queue.clone(), app.swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(e) => {
                // device unplugged or window resized
                println!("{:?}", e);
                previous_frame_end = Box::new(vulkano::sync::now(app.device.clone())) as Box<_>;
            }
        }

        /* handle input & poll events */
        let mut done = false;
        app.event_loop.poll_events(|ev| {
            if let Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } = ev
            {
                done = true
            }
        });
        if done {
            break;
        }
    }
}
