use crate::camera::{Camera, PerspectiveCamera};
use crate::hosek::make_hosek_wilkie_params;
use crate::input::Input;
use crate::io::{load_geometry, load_image};
use crate::mesh::{create_full_screen_triangle, Mesh};
use crate::pod::{HosekWilkieParams, MaterialData, MatrixData};
use crate::render::{BasicVertex, PositionOnlyVertex, Transform};
use crate::samplers::Samplers;
use crate::window::{SwapChain, Window};
use cgmath::{
    vec3, Deg, Euler, InnerSpace, Matrix4, One, PerspectiveFov, Point3, Quaternion, Rad, Rotation,
    Vector3, Zero,
};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Instant;
use vulkano::buffer::{BufferUsage, CpuBufferPool, ImmutableBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::Queue;
use vulkano::format::ClearValue;
use vulkano::format::Format;
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::image::{AttachmentImage, ImageUsage};
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::pipeline::depth_stencil::{Compare, DepthBounds, DepthStencil};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::sampler::Sampler;
use vulkano::sampler::{Filter, MipmapMode, SamplerAddressMode};
use vulkano::sync::{GpuFuture, JoinFuture};
use winit::{DeviceEvent, Event, VirtualKeyCode, WindowEvent};

mod camera;
mod hosek;
mod image;
mod input;
mod io;
mod material;
mod mesh;
mod pod;
mod render;
mod samplers;
mod shaders;
mod window;

pub struct Configuration {
    pub fullscreen: bool,
    pub resolution: [u16; 2],
    pub gpu: usize,
}

fn make_ubo<T>(queue: Arc<Queue>, data: T) -> Arc<ImmutableBuffer<T>>
where
    T: 'static + Send + Sync + Sized,
{
    let (buff, fut) = ImmutableBuffer::from_data(data, BufferUsage::uniform_buffer(), queue)
        .expect("cannot allocate ubo!");
    fut.then_signal_fence_and_flush().ok();
    buff
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

    // create sampler
    let samplers = Samplers::new(app.device.clone()).expect("cannot create samplers");

    // initialize renderer
    let queue = app.graphical_queue.clone();
    let swapchain_format = swapchain.swapchain.format();

    // initialize full-screen triangle
    let (fst, fst_future) =
        create_full_screen_triangle(app.graphical_queue.clone()).expect("cannot create fst");
    fst_future.then_signal_fence_and_flush().ok();

    // create shaders on gpu from precompiled spir-v code
    let vs = shaders::basic_vert::Shader::load(app.device.clone()).unwrap();
    let fs = shaders::basic_frag::Shader::load(app.device.clone()).unwrap();
    let sky_vs = shaders::sky_vert::Shader::load(app.device.clone()).unwrap();
    let sky_fs = shaders::sky_frag::Shader::load(app.device.clone()).unwrap();
    let tonemap_vs = shaders::tonemap_vert::Shader::load(app.device.clone()).unwrap();
    let tonemap_fs = shaders::tonemap_frag::Shader::load(app.device.clone()).unwrap();

    // define a render pass object with one pass
    let render_pass = Arc::new(
        vulkano::ordered_passes_renderpass!(
            app.device.clone(),
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
        .unwrap(),
    );

    let dims = swapchain.dimensions();
    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dims[0] as f32, dims[1] as f32],
        depth_range: 0.0..1.0,
    };

    // create basic pipeline for drawing
    let pipeline = Arc::new(
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
            .build(app.device.clone())
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
            .build(app.device.clone())
            .expect("cannot create aky pipeline"),
    );

    let tonemap_pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<PositionOnlyVertex>()
            .vertex_shader(tonemap_vs.main_entry_point(), ())
            .fragment_shader(tonemap_fs.main_entry_point(), ())
            .triangle_list()
            .viewports(Some(viewport.clone()))
            .render_pass(Subpass::from(render_pass.clone(), 2).unwrap())
            .build(app.device.clone())
            .expect("cannot build tonemap graphics pipeline"),
    );

    info!("loading geometry and image data...");
    let rock_mesh = load_geometry(
        app.graphical_queue.clone(),
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\Rock_1.bf",
    );
    let icosphere_mesh = load_geometry(
        app.graphical_queue.clone(),
        "C:\\Users\\Matej\\CLionProjects\\renderer\\target\\debug\\icosphere.bf",
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

    // create descriptor set 0
    let rock_material = Arc::new(
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_sampled_image(rock_albedo.clone(), samplers.aniso_repeat.clone())
            .unwrap()
            .add_buffer(make_ubo(
                queue.clone(),
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
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_sampled_image(basic.clone(), samplers.aniso_repeat.clone())
            .unwrap()
            .add_buffer(make_ubo(
                queue.clone(),
                MaterialData {
                    albedo_color: vec3(1.0, 1.0, 1.0),
                    alpha_cutoff: 0.0,
                },
            ))
            .unwrap()
            .build()
            .expect("cannot build pds"),
    );

    let orange_material = Arc::new(
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_sampled_image(basic.clone(), samplers.aniso_repeat.clone())
            .unwrap()
            .add_buffer(make_ubo(
                queue.clone(),
                MaterialData {
                    albedo_color: vec3(1.0, 0.5, 0.0),
                    alpha_cutoff: 0.0,
                },
            ))
            .unwrap()
            .build()
            .expect("cannot build pds"),
    );

    // create uniform buffer for descriptor set 1
    let matrix_data_pool = CpuBufferPool::<MatrixData>::uniform_buffer(app.device.clone());
    let hosek_wilkie_sky_pool =
        CpuBufferPool::<HosekWilkieParams>::uniform_buffer(app.device.clone());

    // create additional render buffers
    let depth_buffer = AttachmentImage::with_usage(
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
    let hdr_buffer = AttachmentImage::with_usage(
        app.device.clone(),
        dims,
        Format::B10G11R11UfloatPack32,
        ImageUsage {
            transient_attachment: true,
            input_attachment: true,
            ..ImageUsage::none()
        },
    )
    .unwrap();
    let tonemap_descriptor_set = Arc::new(
        PersistentDescriptorSet::start(tonemap_pipeline.clone(), 0)
            .add_image(hdr_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    // create framebuffers for each swapchain image
    let framebuffers = swapchain
        .images
        .iter()
        .map(|color| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(hdr_buffer.clone())
                    .unwrap()
                    .add(color.clone())
                    .unwrap()
                    .add(depth_buffer.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    app.surface.window().grab_cursor(true).unwrap();
    app.surface.window().hide_cursor(true);

    let mut input = Input::default();
    let mut camera = PerspectiveCamera {
        position: Point3::new(0.0, 3.0, 0.0),
        forward: vec3(1.0, 0.0, 0.0),
        up: vec3(0.0, -1.0, 0.0),
        fov: Deg(120.0).into(),
        aspect_ratio: dims[0] as f32 / dims[1] as f32,
        near: 0.01,
        far: 100.0,
    };
    let mut sun_dir = Vector3::new(1.0, 0.1, 0.0).normalize();
    let start = Instant::now();
    loop {
        swapchain = swapchain.render_frame(|image_num, color_attachment| {
            let t = start.elapsed().as_secs_f32() * 0.125;
            sun_dir = vec3(t.sin(), t.cos(), 0.0);

            let params = make_hosek_wilkie_params(sun_dir, 2.0, vec3(0.0, 0.0, 0.0));
            let ubo_sky_hw = hosek_wilkie_sky_pool.next(params).unwrap();
            let sky_hw_params = PersistentDescriptorSet::start(skybox_pipeline.clone(), 1)
                .add_buffer(ubo_sky_hw)
                .expect("cannot add ubo to pds set=1")
                .build()
                .expect("cannot build pds set=1");

            let rock_transform = Transform {
                position: vec3(0.0, 1.0, 0.0),
                rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                scale: vec3(0.03, 0.03, 0.03),
            };

            let ubo_rock = matrix_data_pool
                .next(MatrixData {
                    model: rock_transform.into(),
                    view: camera.view_matrix(),
                    projection: camera.projection_matrix(),
                })
                .expect("cannot create next sub-buffer");
            let per_object_descriptor_set = PersistentDescriptorSet::start(pipeline.clone(), 1)
                .add_buffer(ubo_rock)
                .expect("cannot add ubo to pds set=1")
                .build()
                .expect("cannot build pds set=1");

            let plane_transform = Transform {
                position: vec3(0.0, 0.0, 0.0),
                rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                scale: vec3(30.0, 1.0, 30.0),
            };
            let ubo_plane = matrix_data_pool
                .next(MatrixData {
                    model: plane_transform.into(),
                    view: camera.view_matrix(),
                    projection: camera.projection_matrix(),
                })
                .expect("cannot create next sub-buffer");

            let per_object_descriptor_set_plane =
                PersistentDescriptorSet::start(pipeline.clone(), 1)
                    .add_buffer(ubo_plane)
                    .expect("cannot add ubo to pds set=1")
                    .build()
                    .expect("cannot build pds set=1");

            let framebuffer = framebuffers[image_num].clone();

            let ubo_sky = matrix_data_pool
                .next(MatrixData {
                    model: Matrix4::from_scale(200.0),
                    view: camera.view_matrix(),
                    projection: camera.projection_matrix(),
                })
                .expect("cannot create next sub-buffer");

            let per_object_descriptor_set_sky =
                PersistentDescriptorSet::start(skybox_pipeline.clone(), 0)
                    .add_buffer(ubo_sky)
                    .expect("cannot add ubo to pds set=1")
                    .build()
                    .expect("cannot build pds set=1");

            // start building the command buffer that will contain all
            // rendering commands for this frame.
            AutoCommandBufferBuilder::primary_one_time_submit(app.device.clone(), queue.family())
                .unwrap()
                .begin_render_pass(
                    framebuffer,
                    false,
                    vec![
                        ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                        ClearValue::None,
                        ClearValue::Depth(1.0),
                    ],
                )
                .unwrap()
                .draw_indexed(
                    pipeline.clone(),
                    &DynamicState::none(),
                    rock_mesh.vertex_buffer.clone(),
                    rock_mesh.index_buffer.clone(),
                    (rock_material.clone(), per_object_descriptor_set),
                    (sun_dir),
                )
                .unwrap()
                .draw_indexed(
                    pipeline.clone(),
                    &DynamicState::none(),
                    plane_mesh.vertex_buffer.clone(),
                    plane_mesh.index_buffer.clone(),
                    (white_material.clone(), per_object_descriptor_set_plane),
                    (sun_dir),
                )
                .unwrap()
                .next_subpass(false)
                .unwrap()
                .draw_indexed(
                    skybox_pipeline.clone(),
                    &DynamicState::none(),
                    icosphere_mesh.vertex_buffer.clone(),
                    icosphere_mesh.index_buffer.clone(),
                    (per_object_descriptor_set_sky, sky_hw_params),
                    (camera.position, start.elapsed().as_secs_f32()),
                )
                .unwrap()
                .next_subpass(false)
                .unwrap()
                .draw_indexed(
                    tonemap_pipeline.clone(),
                    &DynamicState::none(),
                    fst.vertex_buffer.clone(),
                    fst.index_buffer.clone(),
                    tonemap_descriptor_set.clone(),
                    (),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap()
        });

        /* handle input & poll events */
        let mut done = false;
        app.event_loop.poll_events(|ev| match ev {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => done = true,
                WindowEvent::Focused(focus) => input.set_input_state(focus),
                _ => {}
            },
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::Key(k) = event {
                    input.handle_event(k)
                }
                if let DeviceEvent::MouseMotion { delta } = event {
                    if input.input_enabled {
                        camera.rotate(Rad(delta.0 as f32 * 0.001), Rad(delta.1 as f32 * 0.001))
                    }
                }
            }
            _ => {}
        });
        if done {
            break;
        }

        app.surface
            .window()
            .set_title(&format!("{:?}", camera.position));

        /* game update for next frame */
        let speed = if input.is_key_down(VirtualKeyCode::LShift) {
            0.01
        } else {
            0.005
        };
        if input.is_key_down(VirtualKeyCode::A) {
            camera.move_left(speed)
        }
        if input.is_key_down(VirtualKeyCode::D) {
            camera.move_right(speed)
        }
        if input.is_key_down(VirtualKeyCode::S) {
            camera.move_backward(speed)
        }
        if input.is_key_down(VirtualKeyCode::W) {
            camera.move_forward(speed)
        }
        if input.is_key_down(VirtualKeyCode::Space) {
            camera.move_up(speed)
        }
        if input.is_key_down(VirtualKeyCode::LControl) {
            camera.move_down(speed)
        }
    }
}
