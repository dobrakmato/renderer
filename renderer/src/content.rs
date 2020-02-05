use crate::mesh::Mesh;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::hash_map::{Entry, RandomState};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::AtomicU8;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, TryLockError};
use std::thread::spawn;

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

trait Storage<T> {
    fn get_handle(&mut self, file_id: FileId) -> Arc<Resource<T>>;
}

// ------------------

struct Texture;

struct Sound;

struct ResourceStorage {
    textures: HashMap<FileId, Arc<Resource<Texture>>>,
    sounds: HashMap<FileId, Arc<Resource<Sound>>>,
}

impl Storage<Texture> for ResourceStorage {
    fn get_handle(&mut self, file_id: FileId) -> Arc<Resource<Texture>> {
        match self.textures.entry(file_id) {
            Entry::Occupied(t) => t.get().clone(),
            Entry::Vacant(t) => t
                .insert(Arc::new(Resource {
                    file_id,
                    data: RwLock::new(None),
                    state: AtomicU8::new(State::Unloaded as u8),
                }))
                .clone(),
        }
    }
}

impl Storage<Sound> for ResourceStorage {
    fn get_handle(&mut self, file_id: FileId) -> Arc<Resource<Sound>> {
        match self.sounds.entry(file_id) {
            Entry::Occupied(t) => t.get().clone(),
            Entry::Vacant(t) => t
                .insert(Arc::new(Resource {
                    file_id,
                    data: RwLock::new(None),
                    state: AtomicU8::new(State::Unloaded as u8),
                }))
                .clone(),
        }
    }
}

fn test(mut storage: ResourceStorage) {
    let mut texture_handle: Arc<Resource<Texture>> = storage.get_handle(FileId(0));
    let sound_handle: Arc<Resource<Sound>> = storage.get_handle(FileId(1));

    // get possibly null texture from handle
    let texture = texture_handle.data.try_read();
}

struct Resources {
    state: HashMap<FileId, State>,
}

impl Resources {
    fn get_state(file_id: FileId) {}
}
