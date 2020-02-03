use crate::Configuration;
use log::{debug, info};
use std::sync::Arc;
use vulkano::command_buffer::CommandBuffer;
use vulkano::device::{Device, DeviceExtensions, DeviceOwned, Queue};
use vulkano::format::Format;
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::swapchain::{PresentMode, Surface, SurfaceTransform, Swapchain};
use vulkano::sync::GpuFuture;
use vulkano::{app_info_from_cargo_toml, swapchain};
use vulkano_win::VkSurfaceBuild;
use winit::dpi::LogicalSize;
use winit::{EventsLoop, WindowBuilder};

pub struct SwapChain {
    pub swapchain: Arc<Swapchain<winit::Window>>,
    pub images: Vec<Arc<SwapchainImage<winit::Window>>>,
    previous_frame_end: Box<dyn GpuFuture>,
    queue: Arc<Queue>,
}

impl SwapChain {
    pub fn new(
        surface: Arc<Surface<winit::Window>>,
        device: Arc<Device>,
        graphical_queue: Arc<Queue>,
    ) -> Self {
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
            .find(|(f, _)| *f == Format::B8G8R8A8Srgb)
            .map(|(f, _)| *f)
            .expect("cannot find srgb non-linear color space format!");

        // we prefer mailbox as it give less latency but fall back to
        // fifo as it should be supported on all configurations
        let present_mode = if caps.present_modes.mailbox {
            PresentMode::Mailbox
        } else {
            PresentMode::Fifo
        };
        info!("using present_mode={:?}", present_mode);
        info!("using format={:?}", format);

        // here we create a swapchain with previously decided arguments
        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            ImageUsage {
                color_attachment: true,
                transfer_destination: true,
                ..ImageUsage::none()
            },
            &graphical_queue,
            SurfaceTransform::Identity,
            alpha,
            present_mode,
            true,
            None,
        )
        .expect("cannot create swapchain");

        SwapChain {
            previous_frame_end: Box::new(vulkano::sync::now(swapchain.device().clone())),
            swapchain,
            images,
            queue: graphical_queue,
        }
    }

    #[inline]
    pub fn dimensions(&self) -> [u32; 2] {
        self.swapchain.dimensions()
    }

    /// Renders the single frame using the provided `render_fn` on the
    /// main graphical queue passed to the constructor of this struct.
    pub fn render_frame<F: FnMut(usize, Arc<SwapchainImage<winit::Window>>) -> Cb, Cb>(
        mut self,
        mut render_fn: F,
    ) -> Self
    where
        Cb: CommandBuffer + 'static,
    {
        // clean-up all resources from the previous frame
        self.previous_frame_end.cleanup_finished();

        // acquire next framebuffer to write to
        let (idx, fut) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(err) => panic!("{:?}", err), // device unplugged or window resized
        };

        let color_attachment = self.images[idx].clone();
        let command_buffer = render_fn(idx, color_attachment);

        // wait for image to be available and then present drawn the image
        // to screen.
        let future = fut
            .join(self.previous_frame_end)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), idx)
            .then_signal_fence_and_flush();

        // depending on the completion state of the submitted command buffer either
        // return to continue to next frame, or report and error
        match future {
            Ok(future) => {
                self.previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(e) => {
                // device unplugged or window resized
                eprintln!("{:?}", e);
                self.previous_frame_end =
                    Box::new(vulkano::sync::now(self.swapchain.device().clone())) as Box<_>;
            }
        }

        self
    }
}

#[allow(dead_code)]
pub struct Window {
    pub instance: Arc<Instance>,
    pub surface: Arc<Surface<winit::Window>>,
    pub event_loop: EventsLoop,
    pub device: Arc<Device>,
    pub graphical_queue: Arc<Queue>,
    pub compute_queue: Arc<Queue>,
    pub transfer_queue: Arc<Queue>,
}

impl Window {
    /// Creates a new window and initialized the surface and
    /// swapchain objects for specified device and queue.
    pub fn new(conf: Configuration) -> Self {
        // STEP 1: initialize vulkan instance
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

        // STEP 2: create a windows and event loop
        let events_loop = EventsLoop::new();
        let surface = WindowBuilder::new()
            .with_title("renderer")
            .with_dimensions(LogicalSize::new(
                conf.resolution[0] as f64,
                conf.resolution[1] as f64,
            ))
            .with_resizable(false)
            .build_vk_surface(&events_loop, instance.clone())
            .expect("cannot create window");

        // STEP 3: create logical device and queues for dispatching commands
        let physical = PhysicalDevice::enumerate(&instance)
            .inspect(|device| {
                debug!("physical device: {}", device.name());
                debug!(" driver version: {}", device.driver_version());
                debug!(" api version: {:?}", device.api_version());
            })
            .nth(conf.gpu)
            .expect("no gpu device available");
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

        physical.queue_families().for_each(|family| {
            debug!(
                "this family queues: {}, graphics: {}, compute: {}, transfer: {}, sparse: {}, min_tran_gran: {:?}",
                family.queues_count(),
                family.supports_graphics(),
                family.supports_compute(),
                family.explicitly_supports_transfers(),
                family.supports_sparse_binding(),
                family.min_image_transfer_granularity(),
            );
        });

        // find first queue that we can use to do graphics
        let graphical_queue_family = physical
            .queue_families()
            .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap())
            .expect("couldn't find a graphical queue family");

        let transfer_queue_family = physical
            .queue_families()
            .find(|q| q.explicitly_supports_transfers())
            .expect("cannot find dedicated transfer queue family");

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
        let graphical_queue = queues.get(0).unwrap();

        Window {
            event_loop: events_loop,
            surface,
            instance,
            device,
            graphical_queue: graphical_queue.clone(),
            transfer_queue: graphical_queue.clone(),
            compute_queue: graphical_queue.clone(),
        }
    }
}
