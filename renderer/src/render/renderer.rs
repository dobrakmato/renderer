//! *Swapchain* creation & render-loop.

use crate::render::pbr::PBRDeffered;
use crate::render::vulkan::VulkanState;
use crate::render::Frame;
use crate::GameState;
use log::debug;
use log::error;
use log::warn;
use smallvec::SmallVec;
use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::render_pass::FramebufferAbstract;
use vulkano::swapchain;
use vulkano::swapchain::{
    Capabilities, CapabilitiesError, ColorSpace, FullscreenExclusive, PresentMode, Swapchain,
    SwapchainCreationError,
};
use vulkano::sync::{FlushError, GpuFuture, SharingMode};
use winit::window::Window;

/// All possible errors that can happen while creating [`RendererState`](struct.RendererState.html).
#[derive(Debug)]
pub enum RendererStateError {
    CapabilitiesError(CapabilitiesError),
    CannotFindFormat,
    CannotCreateSwapchain(SwapchainCreationError),
}

/// Struct that manages the process of rendering. It contains functions related
/// to render-loop processing, reactions to incoming system messages such as
/// *swapchain* recreation caused by resolution change.
///
/// This class does not perform any rendering or command buffer recording, it only
/// provides low-level wrapper around render-loop.
pub struct RendererState {
    /// The `Device` that is used for rendering.
    device: Arc<Device>,
    /// The `Queue` that will the recorded primary command buffer be submitted to.
    graphical_queue: Arc<Queue>,
    /// Current `Swapchain` object.
    swapchain: Arc<Swapchain<Window>>,
    /// Vector of *swapchain* images.
    swapchain_images: Vec<Arc<ImageView<Arc<SwapchainImage<Window>>>>>,
    /// Vector of current framebuffers.
    framebuffers: SmallVec<[Arc<dyn FramebufferAbstract + Send + Sync>; 4]>,
    /// Whether the vector of framebuffers is out-of-date. Framebuffers may become out-of-date
    /// when resolution of the application changes and need to be recreated before rendering
    /// can continue. They are also out-of-date the first time this object is constructed.
    should_recreate_swapchain: bool,
    /// Future of when the last frame finished rendering & is presented on the screen.
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    /// Current rendering path.
    pub render_path: PBRDeffered,
}

impl RendererState {
    /// Creates a new renderer from provided vulkan state struct.
    pub fn new(vulkan: &VulkanState) -> Result<Self, RendererStateError> {
        let surface = vulkan.surface();
        let device = vulkan.device();
        let graphical_queue = vulkan.graphical_queue();

        let caps: Capabilities = surface
            .capabilities(device.physical_device())
            .map_err(RendererStateError::CapabilitiesError)?;

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
            .ok_or(RendererStateError::CannotFindFormat)?;

        debug!("Chosen {:?} format for swapchain buffers.", format);

        // we prefer mailbox as it give less latency but fall back to
        // fifo as it should be supported on all configurations
        let present_mode = if caps.present_modes.mailbox {
            PresentMode::Mailbox
        } else {
            PresentMode::Fifo
        };

        // lets create a swapchain and vector of created swapchain images
        let (swapchain, swapchain_images) = Swapchain::start(device.clone(), surface)
            .num_images(caps.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .layers(1)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(SharingMode::Exclusive)
            .transform(caps.current_transform)
            .composite_alpha(alpha)
            .present_mode(present_mode)
            .fullscreen_exclusive(FullscreenExclusive::Default)
            .clipped(true)
            .color_space(ColorSpace::SrgbNonLinear)
            .build()
            .map_err(RendererStateError::CannotCreateSwapchain)?;

        let render_path =
            PBRDeffered::new(graphical_queue.clone(), device.clone(), swapchain.clone());

        let swapchain_images = swapchain_imgs_to_views(swapchain_images);
        let framebuffers = match swapchain_images
            .iter()
            .map(|it| render_path.create_framebuffer(it.clone()))
            .collect()
        {
            Ok(t) => t,
            Err(e) => panic!("cannot (re)create framebuffers: {}", e),
        };

        // todo: move RenderPath creation to constructor params, or something
        Ok(RendererState {
            previous_frame_end: now(device.clone()),
            should_recreate_swapchain: true,
            framebuffers,
            render_path,
            swapchain_images,
            swapchain,
            device,
            graphical_queue,
        })
    }

    /// Renders single frame. This function is called from render-loop.
    ///
    /// This function updates internal state of this struct, it is responsible
    /// for freeing unused resources from previous frames.
    pub fn render_frame(&mut self, game_state: &GameState) {
        // clean-up all resources from the previous frame
        if let Some(t) = self.previous_frame_end.as_mut() {
            t.cleanup_finished();
        }

        // if framebuffers are out-of date, we need to recreate them.
        if self.should_recreate_swapchain {
            self.recreate_swapchain();
            self.recreate_framebuffers();

            // force recreation of internal buffers and state of the current
            // render path
            self.render_path
                .dimensions_changed(self.swapchain.dimensions());

            self.should_recreate_swapchain = false;
        }

        // acquire next image from swapchain that will be used for rendering. if the
        // suboptimal flag is true we try to recreate the swapchain after this frame.
        //
        // if the acquire operation fails, we recreate swapchain right away and skip
        // rendering of this frame
        let (idx, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(e) => {
                    warn!("Cannot acquire next image {:?}. Recreating swapchain...", e);
                    self.recreate_swapchain();
                    return;
                }
            };

        if suboptimal {
            self.should_recreate_swapchain = true;
        }

        // build primary command buffer by distributing command buffer
        // recording into multiple threads as parallel job
        let mut frame = Frame {
            render_path: &mut self.render_path,
            game_state,
            framebuffer: self.framebuffers[idx].clone(),
            builder: Some(
                AutoCommandBufferBuilder::primary(
                    self.device.clone(),
                    self.graphical_queue.family(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap(),
            ),
        };

        // let frame create and records it's command buffer(s).
        let primary_cb = frame.build();

        // wait for image to be available and then present drawn the image
        // to screen.
        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.graphical_queue.clone(), primary_cb)
            .unwrap()
            .then_swapchain_present(self.graphical_queue.clone(), self.swapchain.clone(), idx)
            .then_signal_fence_and_flush();

        // depending on the completion state of the submitted command buffer either
        // return to continue to next frame, or report and error
        match future {
            Ok(f) => {
                self.previous_frame_end = Some(f.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.should_recreate_swapchain = true;
                self.previous_frame_end = now(self.device.clone());
            }
            Err(e) => {
                error!("Error occurred during rendering a frame {:?}", e);
                self.previous_frame_end = now(self.device.clone());
            }
        }
    }

    /// Forces recreation of *swapchain* and it's images. Transitively the *framebuffers*   
    /// and internal buffers of current render path will be also recreated.
    pub fn recreate_swapchain(&mut self) {
        // new dimensions of the swapchain
        let new_dimensions = self.swapchain.surface().window().inner_size().into();

        let (swapchain, imgs) = match Swapchain::recreate(&self.swapchain)
            .dimensions(new_dimensions)
            .build()
        {
            Ok(r) => r,
            // This error tends to happen when the user is manually resizing the window.
            // Simply restarting the loop is the easiest way to fix this issue.
            Err(SwapchainCreationError::UnsupportedDimensions) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = swapchain;
        self.swapchain_images = swapchain_imgs_to_views(imgs);
    }

    /// Recreates current *framebuffers* by calling `create_framebuffer` method
    /// on current render path with current *swapchain images*.
    ///
    /// This method is called when current *framebuffers* become out-of-date.
    fn recreate_framebuffers(&mut self) {
        self.framebuffers = match self
            .swapchain_images
            .iter()
            .map(|it| self.render_path.create_framebuffer(it.clone()))
            .collect()
        {
            Ok(t) => t,
            Err(e) => panic!("cannot (re)create framebuffers: {}", e),
        };
    }
}

/// Converts a `Vec<SwapchainImage>` to `Vec<ImageView>` without double Arc-ing the
/// image resource.
fn swapchain_imgs_to_views(
    imgs: Vec<Arc<SwapchainImage<Window>>>,
) -> Vec<Arc<ImageView<Arc<SwapchainImage<Window>>>>> {
    imgs.into_iter()
        .map(|x| ImageView::new(x).ok().unwrap())
        .collect()
}

/// Creates a **now** GpuFuture wrapped in `Box` and `Option`.
#[inline]
fn now(device: Arc<Device>) -> Option<Box<dyn GpuFuture>> {
    Some(vulkano::sync::now(device).boxed())
}
