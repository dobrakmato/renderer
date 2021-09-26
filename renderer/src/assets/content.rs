//! Storage for assets, loading of asset, waiting for asset load and worker threads.

use crate::assets::Asset as BfAsset;
use bf::uuid::Uuid;
use bf::{load_bf_from_bytes, Container};
use crossbeam::channel::{bounded, unbounded, Receiver, Sender, TryRecvError};
use log::{error, info, trace};
use once_cell::sync::Lazy;
use parking_lot::lock_api::MappedRwLockReadGuard;
use parking_lot::{RawRwLock, RwLock, RwLockReadGuard};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use vulkano::device::Queue;

// some helper types
type Map<A> = HashMap<Uuid, AssetSlot<A>>;
type Storage<A> = RwLock<Map<A>>;
type BoxedAsset = Box<dyn BfAsset>;

type SignalRx = Receiver<()>;
type SignalTx = Sender<()>;

type LoadTx = Sender<Load>;
type LoadRx = Receiver<Load>;

/// State of single asset in the storage internal structure.
pub struct AssetSlot<A> {
    /// Possibly loaded asset.
    asset: Option<A>,
    revision: u64,
    rx: Option<SignalRx>,
}

impl<A> AssetSlot<A> {
    pub fn new_empty(rx: SignalRx) -> Self {
        Self {
            asset: Option::None,
            revision: 0,
            rx: Some(rx),
        }
    }
}

// note: maybe we can refactor Load to contain a reference to
// a storage that the asset should be loaded, then we can get
// rid of `static` from the storage. this way we can simply
// replace the whole storage before loading another scene.

/// Request to load an asset.
struct Load {
    uuid: Uuid,
    path: PathBuf,
    tx: SignalTx,
}

/// Actual internal storage.
static STORAGE: Lazy<Storage<BoxedAsset>> = Lazy::new(|| RwLock::new(HashMap::new()));
static WORKER_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Function that drives single worker thread.
fn spawn_worker_thread(rx: LoadRx) {
    std::thread::Builder::new()
        .name(format!(
            "ContentWorker-{}",
            WORKER_COUNTER.fetch_add(1, Ordering::SeqCst)
        ))
        .spawn(move || {
            loop {
                let item = match rx.recv() {
                    Ok(t) => t,
                    Err(_) => break,
                };

                load(item);
            }
            info!("Worker thread exited!");
        })
        .expect("cannot start worker thread");
}

/// Function that actually loads an asset into storage.
fn load(work: Load) {
    // helper macro to skip processing current item in the loop
    // mark it as errored and move to next item in the queue
    macro_rules! give_up_with_error {
        ($err: expr) => {{
            let _err = $err;
            error!(
                "Cannot load asset {:?} due to {:?}",
                work.uuid.to_hyphenated().to_string(),
                &_err
            );
            work.tx.send(()).ok();
            return;
        }};
    }

    let start = Instant::now();
    trace!(" Loading file {:?} as asset {:?}", work.path, work.uuid);

    let bytes = match std::fs::read(work.path) {
        Err(e) => give_up_with_error!(e),
        Ok(t) => t,
    };

    let bf_file = match load_bf_from_bytes(&bytes) {
        Err(e) => give_up_with_error!(e),
        Ok(t) => t,
    };

    let asset: BoxedAsset = match bf_file.into_container() {
        Container::Image(t) => Box::new(t),
        Container::Mesh(t) => Box::new(t),
        Container::Material(t) => Box::new(t),
        Container::Tree(t) => Box::new(t),
    };

    // update the storage
    {
        trace!(" Updating the storage of {:?}", work.uuid);
        trace!(
            "[{:?}] Acquiring WRITE lock to store loaded asset",
            std::thread::current().name()
        );
        let mut guard = STORAGE.write();
        match guard.get_mut(&work.uuid) {
            None => panic!("loaded asset that was not found in storage map"),
            Some(slot) => {
                slot.revision += 1;
                slot.asset = Some(asset);
            }
        }
        trace!("[{:?}] Dropping WRITE lock", std::thread::current().name())
    }

    trace!(
        " Asset {:?} completely loaded in {}ms! ",
        work.uuid,
        start.elapsed().as_millis()
    );
    // send notification (we don't care if it arrives)
    work.tx.send(()).ok();
}

pub struct Content {
    // todo: remove transfer queue from content
    pub transfer_queue: Arc<Queue>,
    roots: Vec<PathBuf>,
    load_queue: LoadTx,
}

impl Content {
    /// Constructs a new `Content` and starts a specified amount of worker (loading)
    /// threads.
    pub fn new(worker_count: usize, transfer_queue: Arc<Queue>, roots: Vec<PathBuf>) -> Self {
        info!("Creating a Content with {} worker threads.", worker_count);
        info!("Using following content roots: ");

        roots.iter().for_each(|x| info!(" - {:?}", x));

        let (tx, rx) = unbounded();

        let content = Self {
            load_queue: tx,
            transfer_queue,
            roots,
        };

        for _ in 0..worker_count {
            spawn_worker_thread(rx.clone());
        }

        content
    }

    fn find_asset(&self, uuid: &Uuid) -> Option<PathBuf> {
        let mut file_name = String::with_capacity(36 + 3);

        file_name.push_str(uuid.to_hyphenated().to_string().to_lowercase().as_str());
        file_name.push_str(".bf");

        let path_file_name = PathBuf::from(&file_name);

        for root in self.roots.iter() {
            let path = root.join(&path_file_name);
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    pub fn request_load(&self, uuid: Uuid) -> LoadRequest {
        let path = self
            .find_asset(&uuid)
            .expect("Asset not found in any root!");
        let (tx, rx) = bounded(1);
        let load = Load { uuid, path, tx };

        trace!("Load request {:?}...", uuid.to_hyphenated().to_string());

        // create initial entry or update existing entry in the storage
        {
            trace!(
                "[{:?}] Acquiring WRITE lock to request load",
                std::thread::current().name()
            );
            let mut guard = STORAGE.write();
            match guard.entry(uuid) {
                Entry::Occupied(mut t) => t.get_mut().rx = Some(rx.clone()),
                Entry::Vacant(t) => {
                    t.insert(AssetSlot::new_empty(rx.clone()));
                }
            }
            trace!("[{:?}] Dropping WRITE lock", std::thread::current().name())
        }

        // push item to the load queue (we don't care if it fails)
        self.load_queue.send(load).ok();

        LoadRequest {
            content: &self,
            uuid,
        }
    }

    pub fn get<A: BfAsset>(&self, uuid: &Uuid) -> Option<MappedRwLockReadGuard<RawRwLock, A>> {
        trace!(
            "[{:?}] Acquiring READ lock to read asset",
            std::thread::current().name()
        );
        let guard = STORAGE.read();

        if guard.contains_key(uuid) && guard.get(uuid).unwrap().asset.is_some() {
            return Some(RwLockReadGuard::map(guard, |g| {
                // we can safely unwrap as we verified that both options
                // are `Some(t)` and we still hold a lock to storage
                let x = g.get(uuid).unwrap().asset.as_ref().unwrap();

                assert!(x.is::<A>());
                x.downcast_ref::<A>().unwrap()
            }));
        }
        trace!("[{:?}] Dropping READ lock", std::thread::current().name());

        None
    }

    pub fn get_blocking<A: BfAsset>(&self, uuid: &Uuid) -> MappedRwLockReadGuard<RawRwLock, A> {
        let rx = {
            trace!(
                "[{:?}] Acquiring READ lock to wait for asset",
                std::thread::current().name()
            );
            let guard = STORAGE.read();
            let x = match guard.get(uuid) {
                None => None,
                Some(slot) => match slot.rx {
                    None => None, // nothing to do, asset is already loaded
                    Some(ref rx) => match rx.try_recv() {
                        Ok(_) => None, // item is loaded, but recv was never called
                        Err(e) => match e {
                            TryRecvError::Empty => Some(rx.clone()), // item is not yet loaded, wait
                            TryRecvError::Disconnected => None, // item is loaded and recv was called
                        },
                    },
                },
            };
            trace!("[{:?}] Dropping READ lock", std::thread::current().name());
            x
        };

        if let Some(rx) = rx {
            rx.recv().ok();
        }

        self.get(uuid).expect("Asset was not found in storage!")
    }

    // todo: add hot-reloading
}

pub struct LoadRequest<'a> {
    uuid: Uuid,
    content: &'a Content,
}

impl<'a> LoadRequest<'a> {
    pub fn wait<A: BfAsset>(&self) -> MappedRwLockReadGuard<RawRwLock, A> {
        self.content.get_blocking(&self.uuid)
    }
}
