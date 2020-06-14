use crate::content::{Load, Result};
use crate::image::ToVulkanFormat;
use bf::load_bf_from_bytes;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::{Dimensions, ImageLayout, ImageUsage, ImmutableImage};

impl Load for ImmutableImage<Format> {
    fn load(bytes: &[u8], transfer_queue: Arc<Queue>) -> Result<Self> {
        let image = load_bf_from_bytes(bytes)
            .expect("cannot load file")
            .try_to_image()
            .unwrap();

        // create image on the gpu and allocate memory for it
        let (immutable, init) = ImmutableImage::uninitialized(
            transfer_queue.device().clone(),
            Dimensions::Dim2d {
                width: image.width as u32,
                height: image.height as u32,
            },
            image.format.to_vulkan_format(),
            image.mipmap_count(),
            ImageUsage {
                transfer_destination: true,
                sampled: true,
                ..ImageUsage::none()
            },
            ImageLayout::ShaderReadOnlyOptimal,
            Some(transfer_queue.family()),
        )
        .expect("cannot create immutable image");
        let init = Arc::new(init);

        let mut cb =
            AutoCommandBufferBuilder::new(transfer_queue.device().clone(), transfer_queue.family())
                .unwrap();

        for (idx, mipmap) in image.mipmaps().enumerate() {
            let source = CpuAccessibleBuffer::from_iter(
                transfer_queue.device().clone(),
                BufferUsage::transfer_source(),
                false,
                mipmap.data.iter().cloned(),
            )
            .unwrap();

            cb.copy_buffer_to_image_dimensions(
                source,
                init.clone(),
                [0, 0, 0],
                [mipmap.width as u32, mipmap.height as u32, 1],
                0,
                1,
                idx as u32,
            )
            .unwrap();
        }

        let cb = cb.build().unwrap();

        let future = match cb.execute(transfer_queue) {
            Ok(f) => f,
            Err(_) => unreachable!(),
        };

        (immutable, Some(Box::new(future)))
    }
}

cache_storage_impl!(ImmutableImage<Format>);
