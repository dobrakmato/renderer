use bf::{load_bf_from_bytes, Container};
use std::any::Any;
use std::sync::Arc;

mod batch;
mod lookup;
mod storage;

pub use batch::{BatchLoad, BatchLoadResults};
pub use lookup::lookup;
pub use storage::Storage;

/// Result of asset load operation.
pub type LoadResult<T> = std::result::Result<T, AssetLoadError>;

/// Marker trait that specifies some struct as an "asset", meaning it
/// can be deserialized from a slice of bytes.
pub trait Asset: Any + Send + Sync + 'static {}

impl Asset for bf::material::Material {}
impl Asset for bf::mesh::Mesh {}
impl Asset for bf::image::Image {}

#[derive(Debug)]
pub enum AssetLoadError {
    FileNotFound,
    CannotReadFile(std::io::Error),
    SerializationError(bf::Error),
}

/// Loads asset from bytes and returns the loaded asset as a `Arc<dyn Any + Send + Sync`.
fn asset_from_bytes_dynamic(bytes: &[u8]) -> LoadResult<Arc<dyn Any + Send + Sync>> {
    Ok(
        match load_bf_from_bytes(bytes)
            .map_err(AssetLoadError::SerializationError)?
            .into_container()
        {
            Container::Image(t) => Arc::new(t),
            Container::Mesh(t) => Arc::new(t),
            Container::Material(t) => Arc::new(t),
        },
    )
}
