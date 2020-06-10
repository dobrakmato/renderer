use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct CPUProfiler<'a> {
    runs: u64,
    total_time: u64,
    last_time: u64,
    name: &'a str,
    last_start: Option<Instant>,
}

impl<'a> CPUProfiler<'a> {
    /// Creates a new profiler with specified name.
    pub fn new(name: &'a str) -> Self {
        CPUProfiler {
            runs: 0,
            total_time: 0,
            last_time: 0,
            name,
            last_start: None,
        }
    }

    /// Creates a new profiler with specified name that is already start()-ed.
    pub fn new_started(name: &'a str) -> Self {
        let mut watch = Self::new(name);
        watch.start();
        watch
    }

    /// If the profiler is currently stopped, starts the stopwatch. If the stopwatch
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

    /// If the profiler is currently running, stops the stopwatch and records the result. If
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
    pub fn last_time(&self) -> Duration {
        Duration::from_micros(self.last_time)
    }

    #[inline]
    pub fn avg_time(&self) -> Duration {
        if self.runs == 0 {
            return Duration::new(0, 0);
        }
        Duration::from_micros((self.total_time as f64 / self.runs as f64) as u64)
    }
}

/// This macro generates a struct containing CPUProfiler objects with
/// specified names. It also implements a `Default` trait for it so it
/// can be easily initialized.
///
/// You can prefix the name of generated struct with `pub` modifier to
/// generate a pub struct. If you only specify the name, the generated
/// struct will not have the `pub` access modifier.
///
/// # Example
///
/// The following invocation
///
/// ```rust
/// use core::impl_stats_struct;
///
/// impl_stats_struct!(pub Statistics; item1, item2);
/// ```
/// expands to
///
/// ```rust
/// use core::perf::CPUProfiler;
///
/// #[derive(Debug)]
/// pub struct Statistics<'a> {
///     pub item1: CPUProfiler<'a>,
///     pub item2: CPUProfiler<'a>,
/// }
/// impl<'a> Default for Statistics<'a> {
///     fn default() -> Self {
///         Statistics {
///             item1: CPUProfiler::new("item1"),
///             item2: CPUProfiler::new("item2"),
///         }
///     }
/// }
/// ```
///
#[macro_export]
macro_rules! impl_stats_struct {
    (pub $name: ident; $($it: ident),+) => {
        #[derive(Debug)]
        pub struct $name<'a> {
            $(pub $it: core::perf::CPUProfiler<'a>,)+
        }

        impl<'a> Default for $name<'a> {
            fn default() -> Self {
                $name {
                    $($it: core::perf::CPUProfiler::new(stringify!($it)),)+
                }
            }
        }
    };
    ($name: ident; $($it: ident),+) => {
        #[derive(Debug)]
        struct $name<'a> {
            $($it: core::perf::CPUProfiler<'a>,)+
        }

        impl<'a> Default for $name<'a> {
            fn default() -> Self {
                $name {
                    $($it: core::perf::CPUProfiler::new(stringify!($it)),)+
                }
            }
        }
    };
}

/// This macro automatically inserts `start()` and `end()` calls with
/// specified `CPUProfiler` at the start and end of the current scope.
///
/// The `start()` call is placed at macro invocation site while `end()`
/// is automatically called with the help of `Drop` trait.
///
///
#[macro_export]
macro_rules! measure_scope {
    ($profiler: expr) => {
        struct ScopedMeasure<'a, 'b>(&'b mut core::perf::CPUProfiler<'a>);
        impl<'a, 'b> ScopedMeasure<'a, 'b> {
            fn start_with_drop_guard(item: &'b mut core::perf::CPUProfiler<'a>) -> Self {
                item.start();
                return Self(item);
            }
        }
        impl<'a, 'b> Drop for ScopedMeasure<'a, 'b> {
            fn drop(&mut self) {
                self.0.end();
            }
        }
        #[allow(unused)]
        let scoped = ScopedMeasure::start_with_drop_guard(&mut $profiler);
    };
}

#[cfg(test)]
mod tests {
    use crate::perf::CPUProfiler;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn stopwatch_creates() {
        let root = CPUProfiler::new("root");

        assert_eq!(root.runs(), 0);
        assert_eq!(root.total_time().as_micros(), 0);
        assert!(root.avg_time().as_secs_f64() <= 0.0);
        assert_eq!(root.last_time().as_micros(), 0);
        assert_eq!(root.name(), "root");
    }

    #[test]
    fn stopwatch_counts_time() {
        let mut root = CPUProfiler::new("root");

        root.start();
        sleep(Duration::from_millis(10));
        root.end();

        assert_eq!(root.runs(), 1);
        assert_ne!(root.total_time().as_micros(), 0);
        assert!(root.avg_time().as_secs_f64() > 0.0);
        assert!(root.last_time().as_secs_f64() > 0.0);
        assert_eq!(root.last_time().as_micros(), root.total_time().as_micros());
        assert!((root.last_time() - root.avg_time()).as_secs_f64() < std::f64::EPSILON);
    }

    #[test]
    #[should_panic]
    fn stopwatch_panics_when_multiple_starts() {
        let mut stopwatch = CPUProfiler::new("root");

        stopwatch.start();
        stopwatch.start();
    }

    #[test]
    #[should_panic]
    fn new_started_stopwatch_panics_when_multiple_starts() {
        let mut stopwatch = CPUProfiler::new_started("root");
        stopwatch.start();
    }

    #[test]
    #[should_panic]
    fn stopwatch_panics_when_multiple_ends() {
        let mut stopwatch = CPUProfiler::new("root");

        stopwatch.start();
        stopwatch.end();
        stopwatch.end();
    }
}
