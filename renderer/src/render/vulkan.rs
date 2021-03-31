//! Vulkan state & initialization.

use crate::RendererConfiguration;
use log::info;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use vulkano::app_info_from_cargo_toml;
use vulkano::device::{Device, DeviceCreationError, DeviceExtensions, Queue};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::swapchain::Surface;
use vulkano_win::{CreationError, VkSurfaceBuild};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

/// Lazily created *Vulkan* `Instance`.
static INSTANCE: OnceCell<Arc<Instance>> = OnceCell::new();

/// Flag that specified whether to use *Vulkan* validation layers.
const USE_VALIDATION_LAYERS: bool = true;

/// Creates or gets the already existing `Instance` struct representing
/// the Vulkan *Instance*.
fn get_or_create_instance() -> Arc<Instance> {
    INSTANCE
        .get_or_init(|| {
            info!("Creating Vulkan instance...");

            let layers = if USE_VALIDATION_LAYERS {
                Some("VK_LAYER_KHRONOS_validation")
            } else {
                None
            };

            // we create vulkan instance object with extensions
            // required to create a windows which we will render to.
            Instance::new(
                Some(&app_info_from_cargo_toml!()),
                &InstanceExtensions {
                    ext_debug_utils: true,
                    ..vulkano_win::required_extensions()
                },
                layers,
            )
            .expect("cannot create vulkan instance")
        })
        .clone()
}

/// Possible errors that may happen during [`VulkanState`](struct.VulkanState.html) creation.
#[derive(Debug)]
pub enum VulkanStateError {
    /// Window or surface couldn't be created.
    CannotCreateWindow(CreationError),
    /// Cannot find requested GPU.
    GPUNotFound(usize),
    /// Graphical queue family couldn't be found.
    GraphicalQueueFamilyNotAvailable,
    /// Transfer queue family couldn't be found.
    TransferQueueFamilyNotAvailable,
    /// Device couldn't be created.
    CannotCreateDevice(DeviceCreationError),
    /// Graphical queue was requested but never created.
    GraphicalQueueNotCreated,
    /// Transfer queue was requested but never created.
    TransferQueueNotCreated,
}

/// State of Vulkan in the application. Contains Vulkan *Device*, used
/// *surface* and *queues* that were created with the device.
///
/// Represents one running Vulkan application. Vulkan *Instance* is
/// created automatically when first `VulkanState` object is created.
pub struct VulkanState {
    device: Arc<Device>,
    surface: Arc<Surface<Window>>,
    graphical_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,
}

impl VulkanState {
    /// Creates or uses already created Vulkan instance and creates a new
    /// window with surface, device and queues for this `VulkanState`.
    pub fn new(
        conf: &RendererConfiguration,
        event_loop: &EventLoop<()>,
    ) -> Result<Self, VulkanStateError> {
        let instance = get_or_create_instance();
        let surface = WindowBuilder::new()
            .with_title("renderer")
            .with_inner_size(conf)
            .with_resizable(true)
            .build_vk_surface(event_loop, instance.clone())
            .map_err(VulkanStateError::CannotCreateWindow)?;

        // todo: move this to camera::init code
        surface.window().set_cursor_grab(true).unwrap();
        surface.window().set_cursor_visible(false);

        let physical = PhysicalDevice::enumerate(&instance)
            .nth(conf.gpu)
            .ok_or(VulkanStateError::GPUNotFound(conf.gpu))?;

        let graphical_queue_family = physical
            .queue_families()
            .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap())
            .ok_or(VulkanStateError::GraphicalQueueFamilyNotAvailable)?;

        let transfer_queue_family = physical
            .queue_families()
            .find(|&q| q.explicitly_supports_transfers())
            .ok_or(VulkanStateError::TransferQueueFamilyNotAvailable)?;

        let (device, mut queues) = Device::new(
            physical,
            physical.supported_features(),
            &DeviceExtensions::supported_by_device(physical),
            [(graphical_queue_family, 0.5), (transfer_queue_family, 0.5)]
                .iter()
                .cloned(),
        )
        .map_err(VulkanStateError::CannotCreateDevice)?;

        let graphical_queue = queues
            .next()
            .ok_or(VulkanStateError::GraphicalQueueNotCreated)?;
        let transfer_queue = queues
            .next()
            .ok_or(VulkanStateError::TransferQueueNotCreated)?;

        Ok(Self {
            device,
            surface,
            graphical_queue,
            transfer_queue,
        })
    }

    /// Returns new `Arc` to the surface used by this `VulkanState`.
    #[inline]
    pub fn surface(&self) -> Arc<Surface<Window>> {
        self.surface.clone()
    }

    /// Returns new `Arc` to the `Device` used by this `VulkanState`.
    #[inline]
    pub fn device(&self) -> Arc<Device> {
        self.device.clone()
    }

    /// Returns new `Arc` to the `Queue` with transfer capabilities
    /// used by this `VulkanState`.
    #[inline]
    pub fn transfer_queue(&self) -> Arc<Queue> {
        self.transfer_queue.clone()
    }

    /// Returns new `Arc` to the `Queue` with transfer graphical
    /// used by this `VulkanState`.
    #[inline]
    pub fn graphical_queue(&self) -> Arc<Queue> {
        self.graphical_queue.clone()
    }
}
