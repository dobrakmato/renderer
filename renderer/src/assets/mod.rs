//! Functionality related to loading assets & managing their memory.

use downcast_rs::{impl_downcast, Downcast};

mod content;
mod lookup;

pub use content::Content;
pub use lookup::lookup;

/// Marker trait that specifies some struct as an "asset" meaning it
/// can be deserialized from a slice of bytes, stored and loaded using
/// a `Storage`.
pub trait Asset: Downcast + Send + Sync + 'static {}

impl_downcast!(Asset);

impl Asset for bf::material::Material {}
impl Asset for bf::mesh::Mesh {}
impl Asset for bf::image::Image {}
impl Asset for bf::tree::Tree {}
