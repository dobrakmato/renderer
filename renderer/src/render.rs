use log::{debug, info};
use safe_transmute::TriviallyTransmutable;
use std::sync::Arc;
use vulkano::app_info_from_cargo_toml;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::swapchain::{ColorSpace, PresentMode, Surface, SurfaceTransform, Swapchain};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::{EventsLoop, WindowBuilder};

#[allow(dead_code)]
pub struct Window {
    pub(crate) event_loop: EventsLoop,
    pub(crate) surface: Arc<Surface<winit::Window>>,
    pub(crate) swapchain: Arc<Swapchain<winit::Window>>,
    pub(crate) swapchain_images: Vec<Arc<SwapchainImage<winit::Window>>>,
    pub(crate) instance: Arc<Instance>,
    pub(crate) device: Arc<Device>,
    pub(crate) queues: Vec<Arc<Queue>>,
}

impl Window {
    /// Creates a new window and initialized the surface and
    /// swapchain objects for specified device and queue.
    pub fn new() -> Self {
        // todo: STEP 1: initialize vulkan instance
        let app_info = app_info_from_cargo_toml!();
        let extensions = vulkano_win::required_extensions();
        let instance = Instance::new(
            Some(&app_info),
            &extensions,
            Some("VK_LAYER_KHRONOS_validation"),
        )
        .expect("cannot create vulkan instance");

        let supported_extensions = InstanceExtensions::supported_by_core().unwrap();
        debug!("supported extensions: {:?}", supported_extensions);
        debug!("loaded extensions: {:?}", extensions);

        // todo: STEP 2: create a windows and event loop
        let events_loop = EventsLoop::new();
        let surface = WindowBuilder::new()
            .with_title("window")
            .with_dimensions(LogicalSize::new(1600.0, 900.0))
            .with_resizable(false)
            .build_vk_surface(&events_loop, instance.clone())
            .expect("cannot create window");

        // todo: STEP 3: create logical device and queues for dispatching commands
        // todo: support choosing a GPU
        let physical = PhysicalDevice::enumerate(&instance)
            .inspect(|device| {
                debug!("physical device: {}", device.name());
                debug!(" driver version: {}", device.driver_version());
                debug!(" api version: {:?}", device.api_version());
            })
            .next()
            .expect("no device available");
        info!("using physical device: {}", physical.name());

        for heap in physical.memory_heaps() {
            debug!(
                " heap #{} has a capacity of {} MB (device_local: {})",
                heap.id(),
                heap.size() as f32 / 1024.0 / 1024.0,
                heap.is_device_local()
            );
        }
        debug!("supported features: {:?}", physical.supported_features());
        debug!(
            "max_uniform_buffer_range: {}",
            physical.limits().max_uniform_buffer_range()
        );

        // find first queue that we can use to do graphics
        let graphical_queue_family = physical
            .queue_families()
            .inspect(|family| {
                debug!(
                    "this family queues: {}, graphics: {}, compute: {}",
                    family.queues_count(),
                    family.supports_graphics(),
                    family.supports_compute()
                );
            })
            .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap())
            .expect("couldn't find a graphical queue family");

        // create a logical device that support all features and
        // extensions that the backing physical device supports.
        let (device, queues) = Device::new(
            physical,
            physical.supported_features(),
            &DeviceExtensions::supported_by_device(physical),
            [(graphical_queue_family, 0.5)].iter().cloned(),
        )
        .expect("cannot create virtual device");

        let queues = queues.collect::<Vec<_>>();

        // todo: STEP 4: create swapchain for rendering to window
        let caps = surface
            .capabilities(device.physical_device())
            .expect("cannot get surface capabilities");

        info!("max_image_extent={:?}", caps.max_image_extent);
        info!("present_modes={:?}", caps.present_modes);
        info!("supported_formats={:?}", caps.supported_formats);

        let dimensions = caps.current_extent.unwrap_or(caps.max_image_extent);
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();

        // to render color correctly and compute in linear color space we must
        // request the vulkan explicitly. here we choose a first swapchain format
        // that has sRGB non-linear color space.
        let format = caps
            .supported_formats
            .iter()
            .find(|(_, cs)| *cs == ColorSpace::SrgbNonLinear)
            .map(|(f, _)| *f)
            .expect("cannot find srgb non-linear color space format!");

        // we prefer mailbox as it give less latency but fall back to
        // fifo as it should be supported on all configurations
        let present_mode = if caps.present_modes.mailbox {
            PresentMode::Mailbox
        } else {
            PresentMode::Fifo
        };
        info!("using present_mode={:?}  ", present_mode);

        let (swapchain, swapchain_images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            caps.supported_usage_flags,
            queues.get(0).unwrap(),
            SurfaceTransform::Identity,
            alpha,
            present_mode,
            true,
            None,
        )
        .expect("cannot create swapchain");

        Window {
            event_loop: events_loop,
            surface,
            swapchain,
            swapchain_images,
            instance,
            device,
            queues,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct BasicVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

unsafe impl TriviallyTransmutable for BasicVertex {}

vulkano::impl_vertex!(BasicVertex, position, normal, uv);

struct Frame {}

impl Frame {
    fn draw() {}
}

enum Pass {
    Shadows,
    Skybox,
    GBuffer,
    IndirectLighting,
    DirectLighting,
    Particles,
    Composite,
    PostProcessing,
    UI,
    Finished,
}
