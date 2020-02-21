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

impl Content {
    pub fn new(transfer_queue: Arc<Queue>) -> Self {
        Self { transfer_queue }
    }

    pub fn load<T, P: PathLike>(&self, path: P) -> Arc<T>
    where
        T: for<'a> Data<'a>,
    {
        let path = path.to_path();
        let bytes = std::fs::read(path).unwrap();

        // we need to extend the lifetime of bytes borrow, that's why we
        // have this weird unreachable unused variable here.
        let _ = {
            let bytes = bytes.as_slice();
            return match T::parse(bytes) {
                ParseResult::Done(t) => Arc::new(t),
                ParseResult::Upload(u) => {
                    let (t, f) = T::upload(u, self.transfer_queue.clone());
                    f.then_signal_fence_and_flush().ok();
                    t
                }
            };
            bytes
        };
        unreachable!();
    }
}
