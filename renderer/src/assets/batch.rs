use crate::assets::Storage;
use bf::uuid::Uuid;
use std::iter::FromIterator;
use std::time::Duration;

#[derive(Default, Copy, Clone, Debug)]
pub struct BatchLoadResults {
    pub loaded: usize,
    pub unloaded: usize,
}

impl BatchLoadResults {
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.unloaded == 0
    }

    #[inline]
    pub fn percentage(&self) -> f64 {
        100.0 * self.loaded as f64 / (self.loaded + self.unloaded) as f64
    }
}

/// Represents a way to load multiple assets together and track
/// the progress of their loading.
pub struct BatchLoad<'a>(&'a Storage, Vec<Uuid>);

impl<'a> BatchLoad<'a> {
    /// Creates a new batch load and queues all items in batch load
    /// to be loaded into the specified storage.
    pub fn new<T: IntoIterator<Item = Uuid>>(storage: &'a Storage, items: T) -> Self {
        let batch = BatchLoad(storage, Vec::from_iter(items));

        for uuid in batch.1.iter() {
            batch.0.request_load(*uuid);
        }

        batch
    }

    /// Check the state of assets and returns `BatchLoadResults` struct.
    #[must_use]
    pub fn check_result(&self) -> BatchLoadResults {
        let mut results = BatchLoadResults::default();
        for uuid in self.1.iter() {
            if self.0.is_ready(uuid) {
                results.loaded += 1;
            } else {
                results.unloaded += 1;
            }
        }

        results
    }

    /// Blocks the current thread by sleep-looping with
    /// specified sleep duration while all assets in this batch
    /// are loaded.
    pub fn sleep_loop(self, sleep_duration: Duration) -> BatchLoad<'a> {
        loop {
            std::thread::sleep(sleep_duration);
            if self.check_result().is_complete() {
                break;
            }
        }

        self
    }

    pub fn into_items(self) -> Vec<Uuid> {
        self.1
    }
}
