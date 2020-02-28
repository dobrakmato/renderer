use crossbeam::channel::{Receiver, Sender};
use log::info;
use std::path::Path;
use std::sync::Arc;
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

/// Type representing future (promise) that is possible unresolved (the
/// result of encapsulated computation is not yet computed). Each `Future`
/// consist of CPU future and a possible GPU one. Not every `Future` has
/// a GPU part associated with it.
pub struct Future<T> {
    recv: Receiver<Result<T>>,
}

impl<T> Future<T> {
    fn from_receiver(recv: Receiver<Result<T>>) -> Self {
        Future { recv }
    }

    pub fn wait_for(&mut self) -> Result<T> {
        self.recv.recv().expect("wait_for failed: cannot recv")
    }

    pub fn wait_for_then_unwrap(&mut self) -> Arc<T> {
        let (t, _) = self.wait_for();
        t
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
    pub fn load<T: 'static + Send + Sync + Load, P: PathLike>(&self, path: P) -> Future<T> {
        let path = path.to_path().to_path_buf();
        info!("submitting load request for {:?}", path);
        let queue = self.transfer_queue.clone();
        let (send, recv) = crossbeam::bounded(1);

        let work = move || {
            let bytes = std::fs::read(path).unwrap();
            let t = T::load(bytes.as_slice(), queue);
            send.send(t).unwrap();
        };

        self.worker.send(Box::new(work)).unwrap();

        Future::from_receiver(recv)
    }
}
