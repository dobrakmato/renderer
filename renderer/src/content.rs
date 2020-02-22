use crate::io::{Data, ParseResult};
use std::path::Path;
use std::sync::Arc;
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

pub struct Content {
    transfer_queue: Arc<Queue>,
}

pub enum Future<T> {
    Cpu(T),
    Gpu(T, Box<dyn GpuFuture>),
}

impl<T> Future<T> {
    pub fn wait_for(self) -> T {
        match self {
            Future::Cpu(t) => t,
            Future::Gpu(t, f) => {
                f.then_signal_fence_and_flush().ok();
                t
            }
        }
    }
}

impl Content {
    pub fn new(transfer_queue: Arc<Queue>) -> Self {
        Self { transfer_queue }
    }

    pub fn load<T, P: PathLike>(&self, path: P) -> Future<Arc<T>>
    where
        T: for<'a> Data<'a>,
    {
        let path = path.to_path();
        let bytes = std::fs::read(path).unwrap();
        let bytes = bytes.as_slice();

        return match T::parse(bytes) {
            ParseResult::Done(t) => Future::Cpu(Arc::new(t)),
            ParseResult::Upload(u) => {
                let (t, f) = T::upload(u, self.transfer_queue.clone());
                Future::Gpu(t, f)
            }
        };
    }
}
