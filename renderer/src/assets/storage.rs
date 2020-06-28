//! Storage for assets, loading of asset, waiting for asset load and worker threads.

use crate::assets::{asset_from_bytes_dynamic, Asset, AssetLoadError, BatchLoad};
use bf::uuid::Uuid;
use core::notification::notification;
use log::{error, info, trace, warn};
use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, Weak};
use std::thread::spawn;
use std::time::{Duration, Instant};
use vulkano::device::Queue;

/// An integer that represent the version of asset that is currently
/// loaded. If an asset is reloaded this number is incremented.
type AssetRevision = u32;

/// All possible states for assets in storage.
enum AssetState {
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
    Loaded(Arc<dyn Any + Send + Sync + 'static>, Instant),
    /// The asset was successfully loaded to memory and it was
    /// already used. It may or may not be currently present in
    /// memory.
    Tracked(Weak<dyn Any + Send + Sync + 'static>),
}

/// Type used to receive load notifications from.
type NotificationRecv = core::notification::Receiver;

/// Type used to send load notifications from a worker thread.
type NotificationSend = core::notification::Sender;

/// The storage is thread-safe container for assets stored by their UUID.
///
/// It is implemented as a `HashMap` protected by `RwLock`. Clients of this
/// struct should acquire the lock for the smallest time possible to avoid
/// blocking other threads.
pub struct Storage {
    storage: RwLock<HashMap<Uuid, (AssetState, NotificationRecv)>>,
    load_queue: crossbeam::Sender<(Uuid, NotificationSend)>,
    revisions: RwLock<HashMap<Uuid, AssetRevision>>,
    roots: Vec<PathBuf>,
    pub transfer_queue: Arc<Queue>,
}

// todo: separate asset path resolving
// todo: separate automatic hot-reloading (watching)

impl Storage {
    /// Constructs a new `Storage` and starts a specified amount of worker
    /// threads.
    pub fn new(worker_count: usize, transfer_queue: Arc<Queue>) -> Arc<Self> {
        info!("Creating a Storage with {} worker threads.", worker_count);

        let (send, recv) = crossbeam::unbounded();

        let storage = Arc::new(Self {
            transfer_queue,
            storage: RwLock::new(HashMap::new()),
            revisions: RwLock::new(HashMap::new()),
            load_queue: send,
            roots: vec![Path::new("D:\\_MATS\\OUT\\").into()],
        });

        for _ in 0..worker_count {
            spawn_worker_thread(recv.clone(), storage.clone());
        }

        storage
    }

    /// Tries to find the asset file for asset specified by UUID in one of
    /// the roots.
    ///
    /// If asset file is not found in any of the configured roots
    /// this function returns `None`.
    pub fn find_asset(&self, uuid: &Uuid) -> Option<PathBuf> {
        let mut file_name = String::with_capacity(36 + 3);

        file_name.push_str(uuid.to_hyphenated().to_string().to_lowercase().as_str());
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

    /// Function that returns whether the asset specified by UUID is loaded
    /// and ready to be used at the time this function was called.
    ///
    /// **Warning**: you should use `get()` if you want to use the result as it can
    /// become unavailable in the time between `is_ready()` and `get()` calls. This
    /// is however unlikely to happen.
    pub fn is_ready(&self, uuid: &Uuid) -> bool {
        match self.storage.read().unwrap().get(uuid) {
            None => false,
            Some((state, _)) => match state {
                AssetState::Queued | AssetState::Loading | AssetState::LoadError(_) => false,
                AssetState::Loaded(_, _) => true,
                AssetState::Tracked(w) => w.strong_count() > 0,
            },
        }
    }

    /// Returns the current asset revision number. Each time the asset is
    /// reloaded this number is increased by one. When asset is loaded
    /// for first time, it is assigned revision number 1.
    ///
    /// Assets that are not loaded (but may be in load-queue) have revision
    /// number zero.
    pub fn revision(&self, uuid: &Uuid) -> AssetRevision {
        *self.revisions.read().unwrap().get(uuid).unwrap_or(&0)
    }

    /// Scans the whole storage and removes asset that were loaded but were never
    /// actually used. It measures the duration between current time and the time
    /// the asset was loaded and if the difference is bigger than provided threshold
    /// the asset will be dropped.
    ///
    /// Note: This function acquires the write lock on the storage
    /// for the whole operation. You should only call this function when you are
    /// sure you can wait for this operation to complete.
    pub fn gc(&self, leak_threshold: Duration) {
        let mut lock = self.storage.write().unwrap();

        // find all leaked assets
        let leaked = lock
            .keys()
            .filter(|k| match lock.get(k).unwrap() {
                (AssetState::Loaded(_, loaded_at), _) => loaded_at.elapsed() > leak_threshold,
                _ => false,
            })
            .copied()
            .collect::<Vec<_>>();

        if !leaked.is_empty() {
            warn!("GC-ed {} leaked assets! You should not rely on garbage collector to clean your mess.", leaked.len());
        }

        // remove all found assets
        for x in leaked {
            lock.remove(&x);
        }
    }

    /// Function that checks whether the asset specified by UUID is currently
    /// loaded and present in memory. If the asset is currently loaded
    /// it returns `Some` with an `Arc` reference to the asset. If the asset is
    /// currently not loaded this function returns `None`.
    ///
    /// **Warning**: this function may block.
    pub fn get<T>(&self, uuid: &Uuid) -> Option<Arc<T>>
    where
        T: Asset,
    {
        // first we acquire read lock and try to solve this `get` request
        // only by reading the AssetState struct.
        match self.storage.read().unwrap().get(uuid) {
            None => return None,
            Some((state, _)) => match state {
                AssetState::Queued => return None,
                AssetState::Loading => return None,
                AssetState::LoadError(e) => {
                    error!("Storage::get({:?}) operation failed because the underlying asset failed to load! {:?}", uuid.to_string(), e);
                    return None;
                }
                AssetState::Tracked(w) => return w.upgrade().map(|a| Arc::downcast(a).unwrap()),
                _ => {}
            },
        };

        // if we could not complete the task with read lock we need
        // to acquire write lock (perhaps we need to finish loading
        // and change the state of hashmap).
        match self.storage.write().unwrap().get_mut(uuid) {
            None => None,
            Some((state, _)) => match state.deref() {
                AssetState::Loaded(t, _) => {
                    // here we need to move out the arc and convert the state
                    // from Loaded(owned) to Tracked(weak reference).
                    let strong = t.clone();
                    *state = AssetState::Tracked(Arc::downgrade(t));
                    Some(strong)
                }
                _ => None,
            }
            .map(|a| Arc::downcast(a).unwrap()),
        }
    }

    /// This function appends the asset specified by its Uuid on the end of
    /// the queue.
    ///
    /// If the specified asset is already present in the queue or is currently
    /// being loaded this function has no effect.
    ///
    /// It returns a `LoadFuture` struct that can be used to block the thread
    /// until the requested asset is successfully loaded.
    ///
    /// Note: this function may block.
    pub fn request_load<T: Asset>(&self, uuid: Uuid) -> LoadFuture<T> {
        // we acquire a read lock to determine whether we need
        // to add the specified asset to queue.
        let (will_load, recv) = match self.storage.read().unwrap().get(&uuid) {
            // the asset uuid is not even present in the hashmap.
            // that means it was never loaded, we should definitely
            // append it to the load queue
            None => (true, None),
            // we have some entry in the hashmap and the action we
            // take now depends on the value in the hashmap
            Some((state, recv)) => (
                match state {
                    AssetState::Queued => false,
                    AssetState::Loading => false,
                    AssetState::LoadError(e) => {
                        error!("Requested re-load of asset that previously failed to load! {:?} Error: {:?}", uuid.to_hyphenated().to_string(), e);
                        true
                    }
                    AssetState::Loaded(_, _) => false,
                    AssetState::Tracked(w) => w.strong_count() > 0,
                },
                Some(recv.clone()),
            ),
        };

        // now if we need to load the asset we acquire write lock and write to the hash
        // map that the asset is queued so it ends up only once in the queue.
        if will_load {
            trace!(
                "Adding {:?} to load queue",
                uuid.to_hyphenated().to_string()
            );

            // create notification back from worker thread to notify
            // that the resource is ready
            let (send, recv) = notification();

            self.storage
                .write()
                .unwrap()
                .insert(uuid, (AssetState::Queued, recv.clone()));

            self.load_queue
                .send((uuid, send))
                .expect("cannot push to load queue");

            return LoadFuture(recv, &self, uuid, PhantomData);
        }

        LoadFuture(
            recv.expect("recv was supposed to be Some but was None"),
            &self,
            uuid,
            PhantomData,
        )
    }

    /// Works the same way as `request_load` method but forces the load
    /// from disk and deserialization even when the asset is present in
    /// memory and usable.
    ///
    /// The asset is first dropped from tracking by the `Storage` and
    /// then the load request is placed on the load-queue. If the asset
    /// is used by someone the asset will not be dropped as it is stored
    /// inside an `Arc`. The struct will continue to use the old revision
    /// of the asset until it decides to use the new version.
    ///
    /// The reloaded asset will be placed in this storage and all consumers
    /// must manually acquire the new revision of the asset from the
    /// storage by calling the `get()` method.
    ///
    /// You can verify the revision of asset that is currently loaded
    /// by calling the `revision()` method. If the `revision` number is
    /// bigger then it was the last time you acquired this asset, you
    /// can be sure that new version of this asset is ready to be used.
    ///
    /// Each call of this function will cause the asset to be loaded once.
    pub fn request_reload<T: Asset>(&self, uuid: Uuid) -> LoadFuture<T> {
        // if the asset is not yet loaded, treat this reload
        // request as normal load request
        if let None = self.storage.read().unwrap().get(&uuid) {
            return self.request_load::<T>(uuid);
        }

        // create notification back from worker thread to notify
        // that the resource is ready
        let (send, recv) = notification();

        // silently drop the old asset value regardless of
        // whether is was Arc<T> or Weak<T>
        self.storage
            .write()
            .unwrap()
            .insert(uuid, (AssetState::Queued, recv.clone()));

        self.load_queue
            .send((uuid, send))
            .expect("cannot push to load queue");

        LoadFuture(recv, &self, uuid, PhantomData)
    }

    /// Creates a new batch load for specified items.
    pub fn request_load_batch<T: Iterator<Item = Uuid>>(&self, items: T) -> BatchLoad {
        BatchLoad::new(&self, items)
    }

    /// Acquires a write lock on the internal HashMap and updates
    /// the state of specified asset.
    ///
    /// # Panics
    /// This function panics in the state for specified asset is not present
    /// in HashMap.
    fn update_asset_state(&self, uuid: &Uuid, state: AssetState) {
        match self.storage.write().unwrap().get_mut(&uuid) {
            Some((t, _)) => *t = state,
            None => panic!(
                "asset with uuid {:?} is not present in storage",
                uuid.to_hyphenated().to_string()
            ),
        }
    }
}

/// Future that can be blocked on until the asset is loaded.
pub struct LoadFuture<'a, T>(NotificationRecv, &'a Storage, Uuid, PhantomData<T>);

impl<'a, T: Asset> LoadFuture<'a, T> {
    /// This function blocks the calling thread until the asset is
    /// loaded and then returns an `Arc` reference to the loaded asset.
    pub fn wait(&self) -> Arc<T> {
        self.0.wait();
        self.1
            .get(&self.2)
            .expect("asset was loaded but not present in storage")
    }
}

/// Spawns a worker thread bound to specified load queue and target
/// storage to load assets to.
fn spawn_worker_thread(
    queue: crossbeam::Receiver<(Uuid, NotificationSend)>,
    storage: Arc<Storage>,
) {
    // helper macro to skip processing current item in the loop
    // mark it as errored and move to next item in the queue
    macro_rules! give_up_with_error {
        ($uuid: expr, $err: expr) => {{
            let _err = $err;
            error!(
                "Cannot load asset {:?} due to {:?}",
                $uuid.to_hyphenated().to_string(),
                &_err
            );
            storage.update_asset_state($uuid, AssetState::LoadError(_err));
            continue;
        }};
    }

    spawn(move || {
        for (uuid, send) in queue.iter() {
            trace!("Starting to load {:?}...", uuid.to_hyphenated().to_string());

            // update state in storage to `Loading`
            storage.update_asset_state(&uuid, AssetState::Loading);

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
            storage.update_asset_state(&uuid, AssetState::Loaded(asset, Instant::now()));

            // update the revision number by incrementing or set it to one
            match storage.revisions.write().unwrap().entry(uuid) {
                Entry::Occupied(mut t) => t.insert(t.get() + 1),
                Entry::Vacant(t) => *t.insert(1),
            };

            // notify potential threads that the asset is ready
            send.signal();
            trace!("Loaded asset {:?}!", uuid.to_hyphenated().to_string());
        }
    });
}
