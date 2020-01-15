use crate::camera::{Camera, PerspectiveCamera};
use crate::io::{load_geometry, load_image};
use crate::render::BasicVertex;
use crate::window::{SwapChain, Window};
use cgmath::{
    vec3, Deg, InnerSpace, Matrix4, PerspectiveFov, Point3, Quaternion, Rotation, Vector3, Zero,
};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Instant;
use vulkano::buffer::CpuBufferPool;
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::format::ClearValue;
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage};
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::sampler::Sampler;
use vulkano::sampler::{Filter, MipmapMode, SamplerAddressMode};
use winit::{DeviceEvent, Event, VirtualKeyCode, WindowEvent};

mod camera;
mod image;
mod io;
mod mesh;
mod render;
mod shaders;
mod window;

pub struct Configuration {
    pub fullscreen: bool,
    pub resolution: [u16; 2],
    pub gpu: usize,
}

fn main() {
    // initialize logging at start of the application
    simple_logger::init().unwrap();

    // load configuration from a file
    let conf = Configuration {
        fullscreen: false,
        resolution: [1600, 900],
        gpu: 0,
    };

    #[cfg(debug_assertions)]
    warn!("this is a debug build. performance may hurt.");

    // initialize vulkan and swapchain
    let mut app = Window::new(conf);
    let mut swapchain = SwapChain::new(
        app.surface.clone(),
        app.device.clone(),
        app.graphical_queue.clone(),
    );

    // initialize renderer
    let queue = app.graphical_queue.clone();
    let swapchain_format = swapchain.swapchain.format();

    info!("loading geometry and image data...");
    let rock_mesh = load_geometry(
        app.graphical_queue.clone(),
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1.bf",
    );
    let plane_mesh = load_geometry(
        app.graphical_queue.clone(),
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\plane.bf",
    );
    let rock_albedo = load_image(
        app.graphical_queue.clone(),
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1_Base_Color.bf",
    );
    let basic = load_image(
        app.graphical_queue.clone(),
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\basic.bf",
    );
    info!("data loaded!");

    // create shaders on gpu from precompiled spir-v code
    let vs = shaders::basic_vert::Shader::load(app.device.clone()).unwrap();
    let fs = shaders::basic_frag::Shader::load(app.device.clone()).unwrap();

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
    let dims = swapchain.dimensions();
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
        SamplerAddressMode::Repeat,
        SamplerAddressMode::Repeat,
        SamplerAddressMode::Repeat,
        0.0,
        16.0,
        1.0,
        100.0,
    )
    .expect("cannot create sampler");

    // create descriptor set 0
    let descriptor_set0_rock = Arc::new(
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_sampled_image(rock_albedo.clone(), sampler.clone())
            .expect("cannot add sampled image to descriptor set")
            .build()
            .expect("cannot build descriptor set"),
    );

    let descriptor_set0_basic = Arc::new(
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_sampled_image(basic.clone(), sampler.clone())
            .expect("cannot add sampled image to descriptor set")
            .build()
            .expect("cannot build descriptor set"),
    );

    // create uniform buffer for descriptor set 1
    struct MatrixData(Matrix4<f32>, Matrix4<f32>, Matrix4<f32>);
    let ubo_matrix_data_pool = CpuBufferPool::<MatrixData>::uniform_buffer(app.device.clone());
    let ubo_matrix_data_pool_plane =
        CpuBufferPool::<MatrixData>::uniform_buffer(app.device.clone());

    // create framebuffers for each swapchain image
    let framebuffers = swapchain
        .images
        .iter()
        .map(|color| {
            let depth = AttachmentImage::with_usage(
                app.device.clone(),
                dims,
                Format::D16Unorm,
                ImageUsage {
                    transient_attachment: true,
                    depth_stencil_attachment: true,
                    ..ImageUsage::none()
                },
            )
            .unwrap();

            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(color.clone())
                    .unwrap()
                    .add(depth)
                    .unwrap()
                    .build()
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    let mut camera = PerspectiveCamera {
        position: Point3::new(0.0, 3.0, 0.0),
        forward: vec3(1.0, 0.0, 0.0),
        up: vec3(0.0, -1.0, 0.0),
        fov: Deg(90.0).into(),
        aspect_ratio: 16.0 / 9.0,
        near: 0.01,
        far: 100.0,
    };
    let start = Instant::now();
    loop {
        swapchain = swapchain.render_frame(|image_num| {
            let scale = Matrix4::from_scale(0.03);
            let rotation = Matrix4::from_angle_y(Deg(start.elapsed().as_secs_f32() * 60.0));
            let translate = Matrix4::from_translation(vec3(0.0, 1.0, 0.0));
            let mvp = MatrixData(
                translate * scale * rotation,
                camera.view_matrix(),
                camera.projection_matrix(),
            );
            let ubo_rock = ubo_matrix_data_pool
                .next(mvp)
                .expect("cannot create next sub-buffer");
            let per_object_descriptor_set = PersistentDescriptorSet::start(pipeline.clone(), 1)
                .add_buffer(ubo_rock)
                .expect("cannot add ubo to pds set=1")
                .build()
                .expect("cannot build pds set=1");

            let scale = Matrix4::from_nonuniform_scale(10.0, 1.0, 10.0);
            let mvp = MatrixData(scale, camera.view_matrix(), camera.projection_matrix());
            let ubo_plane = ubo_matrix_data_pool_plane
                .next(mvp)
                .expect("cannot create next sub-buffer");

            let per_object_descriptor_set_plane =
                PersistentDescriptorSet::start(pipeline.clone(), 1)
                    .add_buffer(ubo_plane)
                    .expect("cannot add ubo to pds set=1")
                    .build()
                    .expect("cannot build pds set=1");

            let framebuffer = framebuffers[image_num].clone();

            // start building the command buffer that will contain all
            // rendering commands for this frame.
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
                    rock_mesh.vertex_buffer.clone(),
                    rock_mesh.index_buffer.clone(),
                    (descriptor_set0_rock.clone(), per_object_descriptor_set),
                    start.elapsed().as_secs_f32(),
                )
                .unwrap()
                .draw_indexed(
                    pipeline.clone(),
                    &DynamicState::none(),
                    plane_mesh.vertex_buffer.clone(),
                    plane_mesh.index_buffer.clone(),
                    (
                        descriptor_set0_basic.clone(),
                        per_object_descriptor_set_plane,
                    ),
                    start.elapsed().as_secs_f32(),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap()
        });

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

            if let Event::DeviceEvent { event, .. } = ev {
                if let DeviceEvent::Key(k) = event {
                    if let Some(t) = k.virtual_keycode {
                        let speed = if k.modifiers.shift { 0.1 } else { 0.05 };
                        match t {
                            VirtualKeyCode::A => camera.move_left(speed),
                            VirtualKeyCode::D => camera.move_right(speed),
                            VirtualKeyCode::S => camera.move_backward(speed),
                            VirtualKeyCode::W => camera.move_forward(speed),
                            VirtualKeyCode::Space => camera.move_up(speed),
                            VirtualKeyCode::LControl => camera.move_down(speed),
                            _ => {}
                        }
                    }
                }
            }
        });
        if done {
            break;
        }
    }
}
