use crate::image::ToVulkanFormat;
use crate::mesh::Mesh;
use crate::render::BasicVertex;
use bf::{load_bf_from_bytes, IndexType};
use log::error;
use safe_transmute::{Error, TriviallyTransmutable};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::{Dimensions, ImageLayout, ImageUsage, ImmutableImage};
use vulkano::sync::GpuFuture;

// todo: provide better loading facility

/// This function loads a geometry BF file into GPU
/// memory and returns vertex and index buffers.
pub fn load_geometry(queue: Arc<Queue>, file: &str) -> Mesh<BasicVertex, u16> {
    let bytes = std::fs::read(file).expect("cannot read file");
    let file = load_bf_from_bytes(bytes.as_slice()).expect("cannot decode bf file");
    let geometry = file.try_to_geometry().expect("bf file is not geometry");

    // dummy Vecs to extend life-time of variables
    let mut vertex_vec = Vec::new();
    let mut index_vec = Vec::new();

    /// This function either just returns the transmuted slice
    /// or performs a copy if the data is misaligned.
    fn possible_non_zero_copy<'a, T: TriviallyTransmutable>(
        bytes: &'a [u8],
        possible_owner: &'a mut std::vec::Vec<T>,
    ) -> &'a [T] {
        match safe_transmute::transmute_many_pedantic::<T>(bytes) {
            Ok(safe) => safe,
            Err(Error::Unaligned(e)) => {
                error!(
                    "cannot zero-copy unaligned &[{:?}] data: {:?}",
                    std::any::type_name::<T>(),
                    e
                );
                *possible_owner = e.copy();
                possible_owner.as_slice()
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    // todo: support multiple index types
    assert_eq!(geometry.index_type, IndexType::U16);

    let vertices = possible_non_zero_copy::<BasicVertex>(geometry.vertex_data, &mut vertex_vec);
    let indices = possible_non_zero_copy::<u16>(geometry.index_data, &mut index_vec);

    fn create_buffer_wait<T: 'static + Clone + Send + Sync>(
        data: &[T],
        usage: BufferUsage,
        queue: Arc<Queue>,
    ) -> Arc<ImmutableBuffer<[T]>> {
        let (buffer, future) = ImmutableBuffer::from_iter(data.iter().cloned(), usage, queue)
            .expect("cannot allocate memory for buffer");
        future.then_signal_fence_and_flush().ok();
        buffer
    }

    let vertex_buffer = create_buffer_wait(vertices, BufferUsage::vertex_buffer(), queue.clone());
    let index_buffer = create_buffer_wait(indices, BufferUsage::index_buffer(), queue);

    Mesh {
        vertex_buffer,
        index_buffer,
    }
}

/// This function loads an image BF file into GPU
/// memory and returns immutable image.
pub fn load_image(queue: Arc<Queue>, file: &str) -> Arc<ImmutableImage<Format>> {
    let bytes = std::fs::read(file).expect("cannot read file");
    let file = load_bf_from_bytes(bytes.as_slice()).expect("cannot decode bf file");
    let image = file.try_to_image().expect("bf file is not image");

    // todo: rewrite to unsafe code to allocate single buffer and copy all mipmaps
    //       in one vkCmdCopyBufferToImage call using multiple regions

    // create image on the gpu and allocate memory for it
    let (immutable, init) = ImmutableImage::uninitialized(
        queue.device().clone(),
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
        Some(queue.family()),
    )
    .expect("cannot create immutable image");
    let init = Arc::new(init);

    let mut cb = AutoCommandBufferBuilder::new(queue.device().clone(), queue.family()).unwrap();

    for (idx, mipmap) in image.mipmaps().enumerate() {
        // todo: bug in vulkano #1292
        let mut padded = mipmap.data.to_vec();
        padded.extend_from_slice(&[0u8; 4096]);

        let source = CpuAccessibleBuffer::from_iter(
            queue.device().clone(),
            BufferUsage::transfer_source(),
            false,
            padded.iter().cloned(),
        )
        .unwrap();

        cb = cb
            .copy_buffer_to_image_dimensions(
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

    future.then_signal_fence_and_flush().ok();

    immutable
}
