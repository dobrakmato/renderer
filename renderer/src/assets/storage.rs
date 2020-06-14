use crate::assets::{asset_from_bytes_dynamic, Asset, AssetLoadError};
use bf::uuid::Uuid;
use log::error;
use std::any::Any;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, Weak};
use std::thread::spawn;

/// All possible states for assets in storage.
enum AssetState {
    /// This state means that the asset is currently not loaded.
    /// The meaning of this state is same as if there was no entry
    /// in the storage for this asset.
    Unloaded,
    /// The asset has not started loading but is already present
    /// in the load queue.
    Queued,
    /// Asset is currently being loaded by some worker and
    /// no further attempt to load should be made while the
    /// asset is in this state.
    Loading,
    /// An error occurred while the asset was being loaded
    /// this error is probably permanent and must be
    /// resolved by manual action taken by user. The request
    /// to load this asset might be executed again in the
    /// future but it will probably fail too.
    LoadError(AssetLoadError),
    /// The asset was successfully loaded to memory and it is
    /// ready to be used.
    Loaded(Arc<dyn Any + Send + Sync + 'static>),
    /// The asset was successfully loaded to memory and it was
    /// already used. It may or may not be currently present in
    /// memory.
    Tracked(Weak<dyn Any + Send + Sync + 'static>),
}

/// The storage is thread safe container for assets stored by their Uuid.
///
/// It is implemented as a `HashMap` protected by `RwLock`. Clients of this
/// struct should acquire the lock for the smallest time possible to avoid
/// blocking other threads.
pub struct Storage {
    storage: RwLock<HashMap<Uuid, AssetState>>,
    queue_send: crossbeam::Sender<Uuid>,
    queue_recv: crossbeam::Receiver<Uuid>,
    roots: Vec<PathBuf>,
}

impl Storage {
    /// Constructs a new `Storage` and starts a specified amount of worker
    /// threads.
    pub fn new(worker_count: usize) -> Arc<Self> {
        let (send, recv) = crossbeam::unbounded();

        let storage = Arc::new(Self {
            storage: RwLock::new(HashMap::new()),
            queue_send: send,
            queue_recv: recv,
            roots: vec![],
        });

        for _ in 0..worker_count {
            spawn_worker_thread(storage.queue_recv.clone(), storage.clone());
        }

        storage
    }

    /// Tries to find the path for specified asset file in one of the roots
    /// of the content.
    pub fn find_asset(&self, uuid: &Uuid) -> Option<PathBuf> {
        let mut file_name = String::with_capacity(36 + 3);

        // SAFETY: We are appending ASCII characters only (UUID)
        uuid.to_hyphenated()
            .encode_lower(unsafe { file_name.as_bytes_mut() });
        file_name.push_str(".bf");

        let path_file_name = PathBuf::from(file_name);

        for root in self.roots.iter() {
            let path = root.join(&path_file_name);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Function that checks whether the asset specified by UUID is currently
    /// loaded and present in memory. If the object is currently loaded
    /// it returns `Some` with a new `Arc` reference to it. If the asset is
    /// currently not loaded this function returns `None`.
    ///
    /// Warning: this function may block.
    pub fn get<T>(&self, uuid: &Uuid) -> Option<Arc<T>>
    where
        T: Asset,
    {
        // first we acquire read lock and try to solve this `get` request
        // only by reading the AssetState struct.
        match self.storage.read().unwrap().get(uuid) {
            None => return None,
            Some(state) => match state {
                AssetState::Unloaded => return None,
                AssetState::Loading => return None,
                AssetState::LoadError(e) => {
                    error!("Storage::get({:?}) operation failed because the underlying asset failed to load! {:?}", uuid.to_string(), e);
                    return None;
                }
                _ => {}
            },
        };

        // if we could not complete the task with read lock we need
        // to acquire write lock (perhaps we need to finish loading
        // and change the state of hashmap).
        match self.storage.write().unwrap().get_mut(uuid) {
            None => None,
            Some(state) => match state.deref() {
                AssetState::Loaded(t) => {
                    // here we need to move out the arc and convert the state
                    // from Loaded(owned) to Tracked(weak reference).
                    let strong = t.clone();
                    *state = AssetState::Tracked(Arc::downgrade(t));
                    Some(strong)
                }
                AssetState::Tracked(w) => w.upgrade(),
                _ => None,
            }
            .map(|a| Arc::downcast(a).unwrap()),
        }
    }

    /// This function appends the asset specified by its Uuid on the end of
    /// the queue. It returns a boolean indicating whether the request was
    /// added to the queue or not.
    ///
    /// Note: this function may block
    pub fn request_load(&self, uuid: Uuid) -> bool {
        // we acquire a read lock to determine whether we need
        // to add the specified asset to queue.
        let will_load = match self.storage.read().unwrap().get(&uuid) {
            // the asset uuid is not even present in the hashmap.
            // that means it was never loaded, we should definitely
            // append it to the load queue
            None => true,
            // we have some entry in the hashmap and the action we
            // take now depends on the value in the hashmap
            Some(state) => match state {
                AssetState::Unloaded => true,
                AssetState::Queued => false,
                AssetState::Loading => false,
                AssetState::LoadError(e) => {
                    error!("Requested re-load of asset that previously failed to load! {:?} Error: {:?}", uuid.to_hyphenated().to_string(), e);
                    true
                }
                AssetState::Loaded(_) => false,
                AssetState::Tracked(w) => w.strong_count() > 0,
            },
        };

        // now if we need to load the asset we acquire write lock and write to the hash
        // map that the asset is queued so it ends up only once in the queue.
        if will_load {
            self.storage
                .write()
                .unwrap()
                .insert(uuid, AssetState::Queued);
            self.queue_send
                .send(uuid)
                .expect("cannot push to load queue");
        }

        will_load
    }

    /// Acquires a write lock on the hashmap and updates the state for specified
    /// asset.
    ///
    /// # Panics
    ///
    /// This function panics in the state for specified asset is not present
    /// in hash-map.
    fn update_state(&self, uuid: &Uuid, state: AssetState) {
        match self.storage.write().unwrap().get_mut(&uuid) {
            Some(t) => *t = state,
            None => panic!(
                "asset with uuid {:?} is not present in storage",
                uuid.to_hyphenated().to_string()
            ),
        }
    }
}

/// Spawns a worker thread bound to specified load queue and target
/// storage to load assets to.
fn spawn_worker_thread(queue: crossbeam::Receiver<Uuid>, storage: Arc<Storage>) {
    // helper macro to skip processing current item in the loop
    // mark it as errored and move to next item in the queue
    macro_rules! give_up_with_error {
        ($uuid: expr, $err: expr) => {{
            storage.update_state($uuid, AssetState::LoadError($err));
            continue;
        }};
    }

    spawn(move || {
        for uuid in queue.iter() {
            // update state in storage to `Loading`
            storage.update_state(&uuid, AssetState::Loading);

            // read bytes from disk
            let bytes = match storage.find_asset(&uuid) {
                None => give_up_with_error!(&uuid, AssetLoadError::FileNotFound),
                Some(path) => match std::fs::read(path) {
                    Err(e) => give_up_with_error!(&uuid, AssetLoadError::CannotReadFile(e)),
                    Ok(t) => t,
                },
            };

            // decode the asset from bytes
            let asset = match asset_from_bytes_dynamic(bytes.as_slice()) {
                Err(e) => give_up_with_error!(&uuid, e),
                Ok(t) => t,
            };

            // place result into storage as `Loaded`
            storage.update_state(&uuid, AssetState::Loaded(asset))
        }
    });
}
