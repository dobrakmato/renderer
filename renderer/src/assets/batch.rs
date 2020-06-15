//! Loading multiple assets in one batch and tracking the load progress
//! of the whole batch.

use crate::assets::{Asset, Storage};
use bf::uuid::Uuid;
use std::iter::FromIterator;
use std::time::Duration;

/// Immediate status (results) of a `BatchLoad`.
#[derive(Default, Copy, Clone, Debug)]
pub struct BatchLoadResults {
    /// Number of already loaded assets from the batch.
    pub loaded: usize,
    /// Number of not yet loaded assets from the batch.
    pub unloaded: usize,
}

impl BatchLoadResults {
    /// Returns whether the batch is complete - all assets are loaded.
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.unloaded == 0
    }

    /// Returns the percentage of loaded assets as `f64`.
    #[inline]
    pub fn percentage(&self) -> f64 {
        100.0 * self.loaded as f64 / (self.loaded + self.unloaded) as f64
    }
}

/// A way to load multiple assets together and track
/// the progress of their loading.
///
/// You should not create this struct manually but use a
/// `request_load_batch` function of a `Storage`.
pub struct BatchLoad<'a>(&'a Storage, Vec<Uuid>);

impl<'a> BatchLoad<'a> {
    /// Creates a new batch load and queues all items in batch load
    /// to be loaded into the specified storage.
    pub fn new<T: IntoIterator<Item = Uuid>>(storage: &'a Storage, items: T) -> Self {
        let batch = BatchLoad(storage, Vec::from_iter(items));

        for uuid in batch.1.iter() {
            batch.0.request_load::<Dummy>(*uuid);
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

    /// Transform this struct into a `Vec<Uuid>` containing the UUIDs
    /// of assets present in this batch.
    pub fn into_items(self) -> Vec<Uuid> {
        self.1
    }
}

#[doc(hidden)]
struct Dummy;
impl Asset for Dummy {}
