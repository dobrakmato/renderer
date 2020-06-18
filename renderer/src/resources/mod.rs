//! Runtime representations of images, meshes and materials.
//!
//! All `create_` functions accept parameter of type `Arc<Queue>`. This is the Vulkan
//! queue that will be used to upload the data to the GPU buffers / images.

pub mod image;
pub mod material;
pub mod mesh;
