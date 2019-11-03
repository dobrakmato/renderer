use std::sync::Arc;
use std::time::Instant;
use vulkano::app_info_from_cargo_toml;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::format::{ClearValue, Format};
use vulkano::image::{Dimensions, StorageImage};
use vulkano::instance::debug::{DebugCallback, MessageSeverity, MessageType};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync::GpuFuture;

mod cs;
mod mandebrot_shader {
    #[allow(dead_code)] // Used to force recompilation of shader change
    const X: &str = include_str!("../shaders/mandelbrot.glsl");
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/mandelbrot.glsl"
    }
}

fn main() {
    let start = Instant::now();
    let app_info = app_info_from_cargo_toml!();
    let extensions = InstanceExtensions::supported_by_core().unwrap();
    let instance = Instance::new(Some(&app_info), &extensions, None)
        .unwrap_or_else(|e| panic!("cannot create vulkan instance: {:?}", e));

    println!("extensions: {:?}", extensions);
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

    let graphical_queue = physical
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
        &Features::none(),
        &DeviceExtensions::supported_by_device(physical),
        [(graphical_queue, 0.5)].iter().cloned(),
    )
    .expect("cannot create virtual device");
    println!("device ready in {}s!", start.elapsed().as_secs_f32());

    // extract the one queue we asked for
    let queue = queues.next().unwrap();

    let buffer1 = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), 0..64)
        .expect("cannot create buffer");
    let buffer2 =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), (0..64).map(|_| 0))
            .expect("cannot create buffer");

    println!("b1={:?}", &*buffer1.read().unwrap());
    println!("b2={:?}", &*buffer2.read().unwrap());

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .expect("oom error when creating command buffer")
        .copy_buffer(buffer1.clone(), buffer2.clone())
        .expect("copy buffer operation failed")
        .build()
        .expect("cannot build command buffer");

    let future = command_buffer
        .execute(queue.clone())
        .expect("cannot submit command buffer for execution");

    // we need to call this to inform vulkano that we can safely access the buffer
    future.then_signal_fence().flush().unwrap();

    println!("b1={:?}", &*buffer1.read().unwrap());
    println!("b2={:?}", &*buffer2.read().unwrap());

    let data = 0..65536u32;
    let buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), data)
        .expect("cannot create buffer");
    let shader = cs::Shader::load(device.clone()).expect("failed to created shader module");

    println!("b={:?}", &*buffer.read().unwrap());

    let pipeline = Arc::new(
        ComputePipeline::new(device.clone(), &shader.main_entry_point(), &())
            .expect("failed to create pipeline"),
    );

    let descriptor_set = Arc::new(
        PersistentDescriptorSet::start(pipeline.clone(), 0)
            .add_buffer(buffer.clone())
            .unwrap_or_else(|e| panic!("cannot add buffer to descriptor set: {}", e))
            .build()
            .unwrap_or_else(|e| panic!("cannot build descriptor set: {}", e)),
    );

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .expect("oom error when creating command buffer")
        .dispatch([1024, 1, 1], pipeline.clone(), descriptor_set.clone(), ())
        .expect("cannot add dispatch operation to command buffer")
        .build()
        .expect("cannot build command buffer");

    let future = command_buffer
        .execute(queue.clone())
        .expect("cannot submit command buffer to execution");

    future.then_signal_fence().flush().unwrap();

    println!("b={:?}", &*buffer.read().unwrap());

    let image = StorageImage::new(
        device.clone(),
        Dimensions::Dim2d {
            width: 1024,
            height: 1024,
        },
        Format::R8G8B8A8Unorm,
        Some(queue.family()),
    )
    .unwrap_or_else(|e| panic!("cannot create image: {}", e));

    let buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        (0..1024 * 1024 * 4).map(|_| 0u8),
    )
    .unwrap();

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .unwrap()
        .clear_color_image(image.clone(), ClearValue::Float([1.0, 0.0, 0.0, 1.0]))
        .unwrap()
        .copy_image_to_buffer(image.clone(), buffer.clone())
        .unwrap()
        .build()
        .unwrap();

    let future = command_buffer.execute(queue.clone()).unwrap();

    future.then_signal_fence().flush().unwrap();

    std::fs::write("img.raw", &*buffer.read().unwrap()).unwrap();

    let mandelbrot_shader =
        mandebrot_shader::Shader::load(device.clone()).expect("failed to created shader module");
    let mandelbrot_pipeline = Arc::new(
        ComputePipeline::new(device.clone(), &mandelbrot_shader.main_entry_point(), &())
            .expect("failed to create pipeline"),
    );

    let set = Arc::new(
        PersistentDescriptorSet::start(mandelbrot_pipeline.clone(), 0)
            .add_image(image.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family())
        .unwrap()
        .dispatch(
            [1024 / 8, 1024 / 8, 1],
            mandelbrot_pipeline.clone(),
            set.clone(),
            (),
        )
        .unwrap()
        .copy_image_to_buffer(image.clone(), buffer.clone())
        .unwrap()
        .build()
        .unwrap();

    let future = command_buffer.execute(queue.clone()).unwrap();

    future.then_signal_fence().flush().unwrap();

    std::fs::write("mandelbrot.raw", &*buffer.read().unwrap()).unwrap();
}
