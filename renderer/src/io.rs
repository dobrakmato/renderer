use crate::content::{Load, Result};
use crate::image::ToVulkanFormat;
use crate::mesh::{IndexType, Mesh};
use crate::render::BasicVertex;
use bf::load_bf_from_bytes;
use log::error;
use safe_transmute::{Error, TriviallyTransmutable};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::{Dimensions, ImageLayout, ImageUsage, ImmutableImage};
use vulkano::sync::GpuFuture;

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

impl<I: IndexType + TriviallyTransmutable + Send + Sync + 'static> Load for Mesh<BasicVertex, I> {
    fn load(bytes: &[u8], queue: Arc<Queue>) -> Result<Self> {
        let geometry = load_bf_from_bytes(bytes)
            .expect("cannot load file")
            .try_to_geometry()
            .unwrap();

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

        let vertices =
            possible_non_zero_copy::<BasicVertex>(geometry.vertex_data.as_slice(), &mut vertex_vec);
        let indices = possible_non_zero_copy::<I>(geometry.index_data.as_slice(), &mut index_vec);

        fn buf<T: 'static + Clone + Send + Sync>(
            data: &[T],
            usage: BufferUsage,
            queue: Arc<Queue>,
        ) -> (Arc<ImmutableBuffer<[T]>>, impl GpuFuture) {
            let (buffer, future) = ImmutableBuffer::from_iter(data.iter().cloned(), usage, queue)
                .expect("cannot allocate memory for buffer");
            (buffer, future)
        }

        let (vertex_buffer, f1) = buf(vertices, BufferUsage::vertex_buffer(), queue.clone());
        let (index_buffer, f2) = buf(indices, BufferUsage::index_buffer(), queue);

        (
            Arc::new(Mesh {
                vertex_buffer,
                index_buffer,
            }),
            Some(Box::new(f1.join(f2))),
        )
    }
}

mod bf_material {
    cache_storage_impl!(bf::material::Material);
}

impl Load for bf::material::Material {
    fn load(bytes: &[u8], _: Arc<Queue>) -> Result<Self> {
        (
            Arc::new(
                load_bf_from_bytes(bytes)
                    .expect("cannot read bytes as bf::material::Material")
                    .try_to_material()
                    .expect("file is not bf::material::Material"),
            ),
            None,
        )
    }
}
