use std::sync::Arc;
use std::time::Instant;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::framebuffer::{Framebuffer, Subpass};
use vulkano::image::SwapchainImage;
use vulkano::instance::debug::{DebugCallback, MessageSeverity, MessageType};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain::{PresentMode, Surface, SurfaceTransform, Swapchain};
use vulkano::sync::GpuFuture;
use vulkano::{app_info_from_cargo_toml, swapchain};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::{Event, EventsLoop, WindowBuilder, WindowEvent};

mod mandebrot_shader {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/mandelbrot.glsl");
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/mandelbrot.glsl"
    }
}

struct Window {
    events_loop: EventsLoop,
    surface: Arc<Surface<winit::Window>>,
    swapchain: Arc<Swapchain<winit::Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<winit::Window>>>,
}

impl Window {
    fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        let events_loop = EventsLoop::new();
        let surface = WindowBuilder::new()
            .with_title("window")
            .with_dimensions(LogicalSize::new(1600.0, 900.0))
            .with_resizable(false)
            .build_vk_surface(&events_loop, device.instance().clone())
            .expect("cannot create window");

        let caps = surface
            .capabilities(device.physical_device())
            .expect("cannot get surface capabilities");

        println!(" max_image_extent={:?}", caps.max_image_extent);
        println!(" present_modes={:?}", caps.present_modes);
        println!(" supported_formats={:?}", caps.supported_formats);

        let dimensions = caps.current_extent.unwrap_or(caps.max_image_extent);
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;

        let (swapchain, swapchain_images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            caps.supported_usage_flags,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            true,
            None,
        )
        .unwrap_or_else(|e| panic!("cannot create swapchain: {}", e));

        Window {
            events_loop,
            surface,
            swapchain,
            swapchain_images,
        }
    }
}

fn main() {
    let start = Instant::now();
    let app_info = app_info_from_cargo_toml!();
    let extensions = vulkano_win::required_extensions();
    let instance = Instance::new(Some(&app_info), &extensions, None)
        .unwrap_or_else(|e| panic!("cannot create vulkan instance: {:?}", e));

    println!(
        "supported extensions: {:?}",
        InstanceExtensions::supported_by_core().unwrap()
    );
    println!("loaded extensions: {:?}", extensions);
    let _callback = DebugCallback::new(
        &instance,
        MessageSeverity::errors_and_warnings(),
        MessageType::all(),
        |msg| {
            println!("[debug] {:?}", msg.description);
        },
    )
    .ok();

    let physical = PhysicalDevice::enumerate(&instance)
        .inspect(|device| {
            println!("physical device: {}", device.name());
            println!(" driver version: {}", device.driver_version());
            println!(" api version: {:?}", device.api_version());
        })
        .next()
        .expect("no device available");
    println!("using physical device: {}", physical.name());
    for heap in physical.memory_heaps() {
        println!(
            " heap #{} has a capacity of {} MB (device_local: {})",
            heap.id(),
            heap.size() as f32 / 1024.0 / 1024.0,
            heap.is_device_local()
        );
    }
    println!("supported features: {:?}", physical.supported_features());
    println!(
        "max_uniform_buffer_range: {}",
        physical.limits().max_uniform_buffer_range()
    );

    let graphical_queue_family = physical
        .queue_families()
        .inspect(|family| {
            println!(
                " family queues: {}, graphics: {}, compute: {}",
                family.queues_count(),
                family.supports_graphics(),
                family.supports_compute()
            );
        })
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &DeviceExtensions::supported_by_device(physical),
        [(graphical_queue_family, 0.5)].iter().cloned(),
    )
    .expect("cannot create virtual device");
    println!("device ready in {}s!", start.elapsed().as_secs_f32());

    // extract the one queue we asked for
    let queue = queues.next().unwrap();

    let mut window = Window::new(device.clone(), queue.clone());
    let swapchain_format = (*window.swapchain.clone()).format();

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain_format,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
    );

    let vertex_buffer = {
        #[derive(Default, Debug, Clone)]
        struct Vertex {
            position: [f32; 2],
        }
        vulkano::impl_vertex!(Vertex, position);

        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            [
                Vertex {
                    position: [-0.75, -0.75],
                },
                Vertex {
                    position: [0.0, 0.75],
                },
                Vertex {
                    position: [0.75, -0.75],
                },
            ]
            .iter()
            .cloned(),
        )
        .unwrap()
    };

    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: "
#version 450
layout(location = 0) in vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}"
        }
    }

    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: "
#version 450
layout(location = 0) out vec4 f_color;
void main() {
    f_color = vec4(gl_FragCoord.x/1600, gl_FragCoord.y/900, 0.0, 1.0);
}
"
        }
    }

    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer()
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let dimensions = window.swapchain.dimensions();
    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    let dynamic_state = DynamicState {
        viewports: Some(vec![viewport]),
        ..DynamicState::none()
    };
    let framebuffers = window
        .swapchain_images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();

    let mut previous_frame_end = Box::new(vulkano::sync::now(device.clone())) as Box<dyn GpuFuture>;

    println!("window ready in {}s!", start.elapsed().as_secs_f32());
    let mut frames = 0;
    loop {
        previous_frame_end.cleanup_finished();
        window
            .surface
            .window()
            .set_title(&format!("frame: {}", frames));

        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(window.swapchain.clone(), None) {
                Ok(r) => r,
                Err(err) => panic!("{:?}", err),
            };

        let clear_values = vec![[0.0, 0.0, (frames as f32 / 60.0).sin().abs(), 1.0].into()];
        let command_buffer =
            AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                .unwrap()
                .begin_render_pass(framebuffers[image_num].clone(), false, clear_values)
                .unwrap()
                .draw(
                    pipeline.clone(),
                    &dynamic_state,
                    vertex_buffer.clone(),
                    (),
                    (),
                )
                .unwrap()
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();
        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(queue.clone(), window.swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        frames += 1;

        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(vulkano::sync::now(device.clone())) as Box<_>;
            }
        }

        /* handle input & poll events */
        let mut done = false;
        window.events_loop.poll_events(|ev| {
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
