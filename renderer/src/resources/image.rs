use std::sync::Arc;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::{Dimensions, ImageCreationError, ImageLayout, ImageUsage, ImmutableImage};
use vulkano::memory::DeviceMemoryAllocError;
use vulkano::sync::GpuFuture;

/// Helper function to convert `bf::image::Format` into
/// Vulkano `Format` enum.
fn to_vulkan_format(format: bf::image::Format) -> Format {
    match format {
        bf::image::Format::R8 => Format::R8Unorm,
        bf::image::Format::Dxt1 => Format::BC1_RGBUnormBlock,
        bf::image::Format::Dxt3 => Format::BC2UnormBlock,
        bf::image::Format::Dxt5 => Format::BC3UnormBlock,
        bf::image::Format::Rgb8 => Format::R8G8B8Unorm,
        bf::image::Format::Rgba8 => Format::R8G8B8A8Unorm,
        bf::image::Format::SrgbDxt1 => Format::BC1_RGBSrgbBlock,
        bf::image::Format::SrgbDxt3 => Format::BC2SrgbBlock,
        bf::image::Format::SrgbDxt5 => Format::BC3SrgbBlock,
        bf::image::Format::Srgb8 => Format::R8G8B8Srgb,
        bf::image::Format::Srgb8A8 => Format::R8G8B8A8Srgb,
        bf::image::Format::BC6H => Format::BC6HUfloatBlock,
        bf::image::Format::BC7 => Format::BC7UnormBlock,
        bf::image::Format::SrgbBC7 => Format::BC7SrgbBlock,
    }
}

#[derive(Debug)]
pub enum CreateImageError {
    CannotCreateImage(ImageCreationError),
    CannotAllocateBuffer(DeviceMemoryAllocError),
}

/// This function creates an `ImmutableImage` struct from provided `bf::image::Image` asset
/// without any conversion.
pub fn create_image(
    image: &bf::image::Image,
    queue: Arc<Queue>,
) -> Result<(Arc<ImmutableImage<Format>>, impl GpuFuture), CreateImageError> {
    // create image on the gpu and allocate memory for it
    let (immutable, init) = ImmutableImage::uninitialized(
        queue.device().clone(),
        Dimensions::Dim2d {
            width: image.width as u32,
            height: image.height as u32,
        },
        to_vulkan_format(image.format),
        image.mipmap_count(),
        ImageUsage {
            transfer_destination: true,
            sampled: true,
            ..ImageUsage::none()
        },
        ImageLayout::ShaderReadOnlyOptimal,
        Some(queue.family()),
    )
    .map_err(CreateImageError::CannotCreateImage)?;

    // we need to wrap the init into `Arc` as we need to send it multiple
    // times as owned variable in the for loop later
    let init = Arc::new(init);

    let mut cb = AutoCommandBufferBuilder::new(queue.device().clone(), queue.family()).unwrap();

    for (idx, mipmap) in image.mipmaps().enumerate() {
        let source = CpuAccessibleBuffer::from_iter(
            queue.device().clone(),
            BufferUsage::transfer_source(),
            false,
            mipmap.data.iter().cloned(),
        )
        .map_err(CreateImageError::CannotAllocateBuffer)?;

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

    let future = match cb.execute(queue) {
        Ok(f) => f,
        Err(_) => unreachable!(),
    };

    Ok((immutable, future))
}