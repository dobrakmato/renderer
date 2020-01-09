use crate::render::{BasicVertex, Window};
use bf::load_bf_from_bytes;
use log::warn;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, ImmutableBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::format::ClearValue;
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage};
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain;
use vulkano::sync::GpuFuture;
use winit::{Event, WindowEvent};

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

fn main() {
    simple_logger::init().unwrap();

    #[cfg(debug_assertions)]
    warn!("this is a debug build. performance may hurt.");

    let mut app = Window::new();

    let queue = app.queues.get(0).unwrap();
    let swapchain_format = app.swapchain.format();

    // upload vertex data to gpu
    let bytes =
        std::fs::read("C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1.bf")
            .expect("cannot read file");
    let file = load_bf_from_bytes(bytes.as_slice()).expect("cannot decode bf file");
    let geometry = file.try_to_geometry().unwrap();

    let vertices = BasicVertex::slice_from_bytes(geometry.vertex_data);
    let indices = unsafe {
        std::slice::from_raw_parts(
            geometry.index_data.as_ptr() as *const u16, // todo: safety?
            geometry.index_data.len() / 2,
        )
    };

    // todo: why we need to iter.clone() ???
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
    loop {
        previous_frame_end.cleanup_finished(); // todo: why is this here?

        // acquire next framebuffer to write to
        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(app.swapchain.clone(), None) {
                Ok(r) => r,
                Err(err) => panic!("{:?}", err), // device unplugged or window resized
            };
        let framebuffer = framebuffers[image_num].clone();

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
                    (),
                    (),
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
