use crossbeam::channel::{Receiver, Sender};
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

pub enum LoadResult {
    Ready,
    GpuFuture(Box<dyn GpuFuture + Send + Sync>),
}

pub trait Load {
    fn load(bytes: &[u8], transfer_queue: Arc<Queue>) -> (Arc<Self>, LoadResult);
}

pub struct Content {
    transfer_queue: Arc<Queue>,
    worker: Sender<Box<dyn FnOnce() -> () + Send>>,
}

pub struct Future<T> {
    recv: Receiver<(Arc<T>, LoadResult)>,
}

impl<T> Future<T> {
    fn from_receiver(recv: Receiver<(Arc<T>, LoadResult)>) -> Self {
        Future { recv }
    }

    pub fn wait_for(&mut self) -> (Arc<T>, LoadResult) {
        self.recv.recv().expect("cannot recv")
    }

    pub fn wait_for_then_unwrap(&mut self) -> Arc<T> {
        let (t, _) = self.wait_for();
        t
    }

    pub fn wait_for_then_flush(&mut self) -> Arc<T> {
        let (t, r) = self.wait_for();
        match r {
            LoadResult::Ready => t,
            LoadResult::GpuFuture(f) => {
                f.then_signal_fence_and_flush().unwrap().wait(None).ok();
                t
            }
        }
    }
}

type Work = Box<dyn FnOnce() + Send>;

impl Content {
    pub fn new(transfer_queue: Arc<Queue>) -> Self {
        let (send, recv): (Sender<Work>, Receiver<Work>) = crossbeam::unbounded();

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

    pub fn load<T: 'static + Send + Sync + Load, P: PathLike>(&self, path: P) -> Future<T> {
        let path = path.to_path().to_path_buf();
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
