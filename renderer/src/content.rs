use std::sync::atomic::AtomicU8;
use std::sync::RwLock;

/// Internal unique identifier of a loadable asset.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub struct FileId(usize);

/// Defines state of the resource. Each resource is either queued
/// for load, currently loading, loaded and ready for use or unloaded.
#[derive(PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    Unloaded,
    Queued,
    Loading,
    Loaded,
}

/// Type that owns the possibly loaded data `T`.
pub struct Resource<T> {
    file_id: FileId,
    data: RwLock<Option<T>>,
    state: AtomicU8,
}

impl<T> Resource<T> {}
