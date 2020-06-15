//! Functionality for cross-thread notifying.
//!
//! Notification is similar to Java's `CountDownLatch` except
//! it can be counted-down only once. Other threads can block
//! on the notification. All waiting threads are resumed when
//! the notification is signaled.

use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};

/// Sender part capable of signaling the notification.
pub struct Sender(Arc<(Mutex<bool>, Condvar)>);

/// Receiver part of notification capable of blocking the current thread
/// until the notification is signaled.
#[derive(Clone)]
pub struct Receiver(Arc<(Mutex<bool>, Condvar)>);

impl Sender {
    /// Signals the notification and resumes all threads that
    /// are blocked on a `wait()` call.
    #[inline]
    pub fn signal(&self) {
        let (mutex, condvar) = self.0.deref();
        let mut ready = mutex.lock().unwrap();
        *ready = true;
        condvar.notify_all()
    }
}

impl Receiver {
    /// Blocks current thread until this notification becomes
    /// signaled.
    #[inline]
    pub fn wait(&self) {
        let (mutex, condvar) = self.0.deref();
        let mut ready = mutex.lock().unwrap();
        while !*ready {
            ready = condvar.wait(ready).unwrap();
        }
    }
}

/// Creates a new notification. Returns a `Sender` and `Receiver`
/// structs. `Sender` can be used to signal the notification and
/// `Receiver` struct can be used to block the thread until the
/// notification becomes signaled.
///
/// Notification is similar to Java's `CountDownLatch` except
/// it can be counted-down only once. Other threads can block
/// on the notification. All waiting threads are resumed when
/// the notification is signaled.
#[allow(clippy::mutex_atomic)] // need mutex for CondVar
pub fn notification() -> (Sender, Receiver) {
    let arc = Arc::new((Mutex::new(false), Condvar::new()));
    (Sender(arc.clone()), Receiver(arc))
}
