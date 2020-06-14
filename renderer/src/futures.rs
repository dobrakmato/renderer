use std::ops::Deref;
use std::sync::{Arc, Condvar, Mutex};

pub struct Sender(Arc<(Mutex<bool>, Condvar)>);

#[derive(Clone)]
pub struct Receiver(Arc<(Mutex<bool>, Condvar)>);

impl Sender {
    #[inline]
    pub fn notify_all(&self) {
        let (mutex, condvar) = self.0.deref();
        let mut ready = mutex.lock().unwrap();
        *ready = true;
        condvar.notify_all()
    }

    #[inline]
    pub fn notify_one(&self) {
        let (mutex, condvar) = self.0.deref();
        let mut ready = mutex.lock().unwrap();
        *ready = true;
        condvar.notify_one()
    }
}

impl Receiver {
    #[inline]
    pub fn wait(&self) {
        let (mutex, condvar) = self.0.deref();
        let mut ready = mutex.lock().unwrap();
        while !*ready {
            ready = condvar.wait(ready).unwrap();
        }
    }
}

/// Creates a new notification.
pub fn notification() -> (Sender, Receiver) {
    let arc = Arc::new((Mutex::new(false), Condvar::new()));
    (Sender(arc.clone()), Receiver(arc))
}

macro_rules! wait_all {
    ($($recv: expr),+) => {
        $($recv.wait());+
    };
}
