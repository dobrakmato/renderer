use crossbeam::channel::Sender;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, RwLock};
use std::thread::spawn;
use vulkano::format::Format;
use vulkano::image::ImmutableImage;

/// Represents that state of a resource.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum State {
    /// This means that the resource is not currently loaded in the memory.
    Unloaded,
    /// This means that the resource was loaded from the disk drive and in the
    /// next step it will be uploaded (or is being) uploaded to device (GPU).
    Loaded,
    /// This means that the resource was loaded from disk drive, uploaded to
    /// target device (GPU) and is ready to be used.
    Ready,
}

/// Internal structure used to store all data about a resource.
#[derive(Debug)]
pub struct Resource<T> {
    state: AtomicU8,
    data: RwLock<Option<T>>,
}

impl<T> Default for Resource<T> {
    fn default() -> Self {
        Resource {
            state: AtomicU8::new(State::Unloaded as u8),
            data: RwLock::new(None),
        }
    }
}

pub struct Storage<K, T> {
    resources: HashMap<K, Arc<Resource<T>>>,
}

impl<K: Hash + Eq, T> Default for Storage<K, T> {
    fn default() -> Self {
        Storage {
            resources: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq, T> Storage<K, T> {
    pub fn handle(&mut self, key: K) -> Arc<Resource<T>> {
        match self.resources.entry(key) {
            Entry::Occupied(t) => t.get().clone(),
            Entry::Vacant(t) => t.insert(Arc::new(Resource::default())).clone(),
        }
    }
}

pub struct Content {
    textures: Storage<String, ImmutableImage<Format>>,
    io_loader: IoLoader<String>,
    gpu_uploader: GpuUploader<String>,
}

struct LoadRequest<K> {
    file: K,
    data: Vec<u8>,
    state: State,
}

impl Content {
    pub fn new() -> Self {
        Content {
            textures: Storage::default(),
            io_loader: IoLoader::new(),
            gpu_uploader: GpuUploader::new(),
        }
    }
}

struct IoLoader<K> {
    work: Sender<LoadRequest<K>>,
}

impl<K: Send + 'static> IoLoader<K> {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();
        spawn(move || loop {
            if let Ok(t) = receiver.recv() {}
        });
        Self { work: sender }
    }
}

struct GpuUploader<K> {
    work: Sender<LoadRequest<K>>,
}

impl<K: Send + 'static> GpuUploader<K> {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam::channel::bounded(5);
        spawn(move || loop {
            if let Ok(t) = receiver.recv() {}
        });
        Self { work: sender }
    }
}
