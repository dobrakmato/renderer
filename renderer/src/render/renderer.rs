//! *Swapchain* creation & render-loop.

use crate::assets::Storage;
use crate::render::vulkan::VulkanState;
use crate::render::{Frame, RenderPath};
use crate::GameState;
use log::warn;
use smallvec::SmallVec;
use std::sync::Arc;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::framebuffer::FramebufferAbstract;
use vulkano::image::ImageUsage;
use vulkano::swapchain;
use vulkano::swapchain::{
    ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
    SwapchainCreationError,
};
use vulkano::sync::GpuFuture;
use winit::window::Window;

// render path, vulkan device, framebuffers, swapchain
pub struct RendererState {
    pub render_path: RenderPath,
    device: Arc<Device>,
    pub graphical_queue: Arc<Queue>,
    /* swapchain related stuff */
    swapchain: Arc<Swapchain<Window>>,
    framebuffers: SmallVec<[Arc<dyn FramebufferAbstract + Send + Sync>; 4]>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl RendererState {
    pub fn new(vulkan: &VulkanState, assets: &Storage) -> Self {
        let surface = vulkan.surface();
        let device = vulkan.device();
        let graphical_queue = vulkan.graphical_queue();

        let caps = surface
            .capabilities(device.physical_device())
            .expect("cannot get surface capabilities");

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

        let (swapchain, swapchain_images) = Swapchain::new(
            device.clone(),
            surface,
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
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .expect("cannot create swapchain");

        let render_path = RenderPath::new(
            graphical_queue.clone(),
            device.clone(),
            swapchain.clone(),
            assets,
        );

        let framebuffers = match swapchain_images
            .iter()
            .map(|it| render_path.create_framebuffer(it.clone()))
            .collect()
        {
            Ok(t) => t,
            Err(e) => panic!("cannot create framebuffers: {}", e),
        };

        RendererState {
            previous_frame_end: Some(Box::new(vulkano::sync::now(device.clone())) as Box<_>),
            render_path,
            swapchain,
            framebuffers,
            device,
            graphical_queue,
        }
    }

    pub fn render_frame(&mut self, game_state: &GameState) {
        // clean-up all resources from the previous frame
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // acquire next image. if the suboptimal is true we try to recreate the
        // swapchain after this frame rendering is done
        let (idx, suboptimal, fut) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(e) => {
                    let dimensions = self.swapchain.surface().window().inner_size().into();
                    warn!("Cannot acquire next image {:?}. Recreating swapchain...", e);
                    self.recreate_swapchain(dimensions);
                    return;
                }
            };

        let mut frame = Frame {
            render_path: &mut self.render_path,
            game_state,
            framebuffer: self.framebuffers[idx].clone(),
            builder: Some(
                AutoCommandBufferBuilder::primary_one_time_submit(
                    self.device.clone(),
                    self.graphical_queue.family(),
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
            .join(fut)
            .then_execute(self.graphical_queue.clone(), primary_cb)
            .unwrap()
            .then_swapchain_present(self.graphical_queue.clone(), self.swapchain.clone(), idx)
            .then_signal_fence_and_flush();

        // depending on the completion state of the submitted command buffer either
        // return to continue to next frame, or report and error
        match future {
            Ok(future) => {
                self.previous_frame_end = Some(Box::new(future) as Box<_>);
            }
            Err(e) => {
                // device unplugged or window resized
                eprintln!("{:?}", e);
                self.previous_frame_end =
                    Some(Box::new(vulkano::sync::now(self.device.clone())) as Box<_>);
            }
        }

        if suboptimal {
            warn!("Swapchain image is suboptimal! Recreating swapchain.");
            let dimensions = self.swapchain.surface().window().inner_size().into();
            self.recreate_swapchain(dimensions);
        }
    }

    pub fn recreate_swapchain(&mut self, dimensions: [u32; 2]) {
        let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimensions(dimensions)
        {
            Ok(r) => r,
            // This error tends to happen when the user is manually resizing the window.
            // Simply restarting the loop is the easiest way to fix this issue.
            Err(SwapchainCreationError::UnsupportedDimensions) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.render_path.recreate_buffers(dimensions);

        let new_framebuffers = new_images
            .iter()
            .map(|x| self.render_path.create_framebuffer(x.clone()))
            .map(|x| x.expect("cannot create framebuffer"))
            .collect();

        self.swapchain = new_swapchain;
        self.framebuffers = new_framebuffers;
    }
}
