use std::time::{Instant, Duration};

pub struct Stopwatch<'a> {
    runs: u64,
    total_time: u64,
    last_time: u64,
    name: &'a str,
    last_start: Option<Instant>,
}

impl<'a> Stopwatch<'a> {
    /// Creates a new Stopwatch with specified name.
    pub fn new(name: &'a str) -> Self {
        Stopwatch {
            runs: 0,
            total_time: 0,
            last_time: 0,
            name,
            last_start: None,
        }
    }

    /// Creates a new Stopwatch with specified name that is already start()-ed.
    pub fn new_started(name: &'a str) -> Self {
        let mut watch = Self::new(name);
        watch.start();
        watch
    }

    /// If the stopwatch is currently stopped, starts the stopwatch. If the stopwatch
    /// is currently running panics with error. Stopwatch cannot be started when it is
    /// already running.
    ///
    /// # Panics
    /// Calling this method panics if this Stopwatch is already in started state.
    pub fn start(&mut self) {
        match self.last_start {
            Some(_) => panic!("Stopwatch must end() before start()-ing again!"),
            None => self.last_start = Some(Instant::now()),
        }
    }

    /// If the stopwatch is currently running, stops the stopwatch and records the result. If
    /// the stopwatch is already stopped, panics. Stopwatch cannot be stopped before it is
    /// started again.
    ///
    /// # Panics
    /// Calling this method panics if this Stopwatch is already in stopped state.
    pub fn end(&mut self) {
        match self.last_start {
            None => panic!("Stopwatch must be start()-end before end()-ing again!"),
            Some(s) => {
                let elapsed = Instant::now() - s;
                self.last_time = elapsed.as_micros() as u64;
                self.total_time += self.last_time;
                self.runs += 1;
                self.last_start = None;
            }
        }
    }

    #[inline]
    pub fn name(&self) -> &'a str {
        self.name
    }

    #[inline]
    pub fn runs(&self) -> u64 {
        self.runs
    }

    #[inline]
    pub fn total_time(&self) -> Duration {
        Duration::from_micros(self.total_time)
    }

    #[inline]
    pub fn last_time(&self) -> u64 {
        self.last_time
    }

    #[inline]
    pub fn avg_time(&self) -> f64 {
        if self.runs == 0 { return 0.0; }
        self.total_time as f64 / self.runs as f64
    }
}

#[cfg(test)]
mod tests {
    use crate::perf::Stopwatch;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn stopwatch_creates() {
        let root = Stopwatch::new("root");

        assert_eq!(root.runs(), 0);
        assert_eq!(root.total_time().as_micros(), 0);
        assert_eq!(root.avg_time(), 0.0);
        assert_eq!(root.last_time(), 0);
        assert_eq!(root.name(), "root");
    }

    #[test]
    fn stopwatch_counts_time() {
        let mut root = Stopwatch::new("root");

        root.start();
        sleep(Duration::from_millis(10));
        root.end();

        assert_eq!(root.runs(), 1);
        assert_ne!(root.total_time().as_micros(), 0);
        assert_ne!(root.avg_time(), 0.0);
        assert_ne!(root.last_time(), 0);
        assert_eq!(root.last_time(), root.total_time().as_micros() as u64);
        assert_eq!(root.last_time() as f64, root.avg_time());
    }

    #[test]
    #[should_panic]
    fn stopwatch_panics_when_multiple_starts() {
        let mut stopwatch = Stopwatch::new("root");

        stopwatch.start();
        stopwatch.start();
    }

    #[test]
    #[should_panic]
    fn new_started_stopwatch_panics_when_multiple_starts() {
        let mut stopwatch = Stopwatch::new_started("root");
        stopwatch.start();
    }

    #[test]
    #[should_panic]
    fn stopwatch_panics_when_multiple_ends() {
        let mut stopwatch = Stopwatch::new("root");

        stopwatch.start();
        stopwatch.end();
        stopwatch.end();
    }
}