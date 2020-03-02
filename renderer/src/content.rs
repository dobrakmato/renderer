use crossbeam::channel::{Receiver, Sender};
use log::info;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use std::thread::spawn;
use vulkano::device::Queue;
use vulkano::sync::GpuFuture;

/// Trait representing type that can be transformed into IO path.
pub trait PathLike {
    fn to_path(&self) -> &Path;
}

impl<'a> PathLike for &'a str {
    fn to_path(&self) -> &Path {
        Path::new(self)
    }
}

/// Represents request dispatched to worker IO thread.
type Request = Box<dyn FnOnce() + Send>;

/// Represents result of Load::load() function.
pub type Result<T> = (Arc<T>, Option<Box<dyn GpuFuture + Send>>);

pub trait Load {
    fn load(bytes: &[u8], transfer_queue: Arc<Queue>) -> Result<Self>;
}

pub trait Storage: Sized {
    fn lookup(k: &PathBuf) -> Option<Arc<Future<Self>>>;
    fn store(k: PathBuf, v: Weak<Future<Self>>);
}

macro_rules! cache_storage_impl {
    ($t:ty) => {
        static STORAGE: once_cell::sync::OnceCell<
            std::sync::Mutex<
                std::collections::HashMap<
                    std::path::PathBuf,
                    std::sync::Weak<crate::content::Future<$t>>,
                >,
            >,
        > = once_cell::sync::OnceCell::new();

        #[inline]
        fn internal_storage_init() {
            if STORAGE.get().is_none() {
                STORAGE
                    .set(std::sync::Mutex::new(std::collections::HashMap::new()))
                    .map_err(|_| panic!("cannot initialize STORAGE"))
                    .unwrap();
            }
        }

        impl crate::content::Storage for $t {
            fn lookup(
                k: &std::path::PathBuf,
            ) -> Option<std::sync::Arc<crate::content::Future<$t>>> {
                internal_storage_init();

                let m = STORAGE.get().unwrap();
                let h = m.lock().unwrap();

                h.get(k).map(|x| x.upgrade()).flatten()
            }

            fn store(k: std::path::PathBuf, v: std::sync::Weak<crate::content::Future<$t>>) {
                internal_storage_init();

                let m = STORAGE.get().unwrap();
                let mut h = m.lock().unwrap();

                h.insert(k, v);
            }
        }
    };
}

/// Type representing future (promise) that is possible unresolved (the
/// result of encapsulated computation is not yet computed). Each `Future`
/// consist of CPU future and a possible GPU one. Not every `Future` has
/// a GPU part associated with it.
pub enum Future<T> {
    Receiver(Receiver<Result<T>>),
    Resolved(Arc<T>),
}

impl<T> Future<T> {
    pub fn wait_for_then_unwrap(&self) -> Arc<T> {
        match self {
            Future::Receiver(r) => r.recv().expect("wait_for failed: cannot recv").0.clone(),
            Future::Resolved(t) => t.clone(),
        }
    }
}

pub struct Content {
    transfer_queue: Arc<Queue>,
    worker: Sender<Request>,
}

impl Content {
    pub fn new(transfer_queue: Arc<Queue>) -> Self {
        let (send, recv): (Sender<Request>, Receiver<Request>) = crossbeam::unbounded();

        /* we start the worker thread */
        spawn(move || loop {
            if let Ok(t) = recv.recv() {
                t();
            }
        });

        Self {
            transfer_queue,
            worker: send,
        }
    }

    /// This function will enqueue the request to load a resource specified by `path`
    /// to a worker queue. This queue is FIFO. When the next worker thread is available
    /// it will perform the actual load code.
    ///
    /// This function returns `Future` which can be used to check if the resource is
    /// already loaded or not. It can be also used to block calling thread until the
    /// resource is loaded however this is usually not what you want.
    ///
    /// If the resource specified by `path` is already loaded in memory, the existing
    /// resource will be returned inside already resolved `Future`.  
    pub fn load<T: Debug + 'static + Send + Sync + Load + Storage, P: PathLike>(
        &self,
        path: P,
    ) -> Arc<Future<T>> {
        let path = path.to_path().to_path_buf();
        let id = path.file_name().unwrap().to_os_string();

        info!("[{:?}] load requested!", id);

        if let Some(t) = T::lookup(&path) {
            info!("[{:?}] returned existing future.", id);
            return t;
        }

        let queue = self.transfer_queue.clone();
        let (send, recv) = crossbeam::bounded(1);
        let future = Arc::new(Future::Receiver(recv));
        T::store(path.clone(), Arc::downgrade(&future));

        let work = move || {
            info!("[{:?}] starting loading...", id);
            let bytes = std::fs::read(&path).unwrap();
            let t = T::load(bytes.as_slice(), queue);

            info!("[{:?}] done", id);
            send.send(t).unwrap();
        };

        self.worker.send(Box::new(work)).unwrap();

        future
    }
}
