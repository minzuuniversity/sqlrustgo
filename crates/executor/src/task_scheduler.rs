//! TaskScheduler Module
//!
//! Provides task scheduling and thread pool management for parallel query execution.
//!
//! v3.10.0 Issue #3703: Rayon-based implementations are feature-gated
//! behind `parallel-executor`. When the feature is OFF, a stub sequential
//! scheduler is exported so the binary still compiles.

#[cfg(feature = "parallel-executor")]
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;
#[cfg(feature = "parallel-executor")]
use std::time::Duration;
// v3.10.0 Issue #3703: rayon gated by parallel-executor feature
#[cfg(feature = "parallel-executor")]
mod rayon_impl {
    pub use rayon::ThreadPool;
}
#[cfg(feature = "parallel-executor")]
use rayon_impl::*;

/// TaskScheduler trait - unified interface for task scheduling
pub trait TaskScheduler: Send + Sync {
    /// Submit a single task for execution
    fn submit<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static;

    /// Submit multiple tasks as a batch
    fn submit_batch<I>(&self, tasks: I)
    where
        I: IntoIterator<Item = Box<dyn FnOnce() + Send + 'static>>;

    /// Wait for all submitted tasks to complete
    fn wait(&self);

    /// Set the parallelism degree (number of threads)
    fn set_parallelism(&self, n: usize);

    /// Get current parallelism degree
    fn current_parallelism(&self) -> usize;
}

#[cfg(feature = "parallel-executor")]
/// Rayon-based implementation of TaskScheduler
pub struct RayonTaskScheduler {
    pool: Arc<ThreadPool>,
    active_tasks: Arc<AtomicUsize>,
}

#[cfg(feature = "parallel-executor")]
impl RayonTaskScheduler {
    /// Create a new RayonTaskScheduler with specified parallelism
    pub fn new(parallelism: usize) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(parallelism)
            .build()
            .expect("Failed to create thread pool");

        Self {
            pool: Arc::new(pool),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create with custom rayon configuration
    pub fn with_config(parallelism: usize, stack_size: usize) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(parallelism)
            .stack_size(stack_size)
            .build()
            .expect("Failed to create thread pool");

        Self {
            pool: Arc::new(pool),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[cfg(feature = "parallel-executor")]
impl TaskScheduler for RayonTaskScheduler {
    fn submit<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        let active = self.active_tasks.clone();
        self.pool.spawn(move || {
            task();
            active.fetch_sub(1, Ordering::SeqCst);
        });
    }

    fn submit_batch<I>(&self, tasks: I)
    where
        I: IntoIterator<Item = Box<dyn FnOnce() + Send + 'static>>,
    {
        for task in tasks {
            self.active_tasks.fetch_add(1, Ordering::SeqCst);
            let active = self.active_tasks.clone();
            self.pool.spawn(move || {
                task();
                active.fetch_sub(1, Ordering::SeqCst);
            });
        }
    }

    fn wait(&self) {
        while self.active_tasks.load(Ordering::SeqCst) > 0 {
            thread::sleep(Duration::from_millis(1));
        }
    }

    fn set_parallelism(&self, _n: usize) {
        log::warn!("Rayon does not support runtime parallelism change");
    }

    fn current_parallelism(&self) -> usize {
        self.pool.current_num_threads()
    }
}

/// Sequential stub scheduler (used when parallel-executor feature is OFF)
#[cfg(not(feature = "parallel-executor"))]
pub struct RayonTaskScheduler {
    _active_tasks: std::sync::atomic::AtomicUsize,
}

#[cfg(not(feature = "parallel-executor"))]
impl RayonTaskScheduler {
    pub fn new(_parallelism: usize) -> Self {
        Self {
            _active_tasks: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn with_config(_parallelism: usize, _stack_size: usize) -> Self {
        Self::new(1)
    }
}

#[cfg(not(feature = "parallel-executor"))]
impl TaskScheduler for RayonTaskScheduler {
    fn submit<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        task();
    }

    fn submit_batch<I>(&self, tasks: I)
    where
        I: IntoIterator<Item = Box<dyn FnOnce() + Send + 'static>>,
    {
        // Stub: execute tasks synchronously on the current thread.
        // In sequential (non-parallel-executor) mode, the batch is small
        // enough that this is acceptable.
        for task in tasks {
            task();
        }
    }

    fn wait(&self) {}

    fn set_parallelism(&self, _n: usize) {}

    fn current_parallelism(&self) -> usize {
        1
    }
}

/// Create a default TaskScheduler with optimal parallelism
pub fn create_default_scheduler() -> impl TaskScheduler {
    let parallelism = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    RayonTaskScheduler::new(parallelism)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    // ---- Sequential stub paths (active when parallel-executor feature is OFF)

    #[test]
    fn stub_scheduler_new_and_parallelism() {
        let s = RayonTaskScheduler::new(8);
        assert_eq!(s.current_parallelism(), 1, "stub always reports 1");
    }

    #[test]
    fn stub_scheduler_with_config() {
        let s = RayonTaskScheduler::with_config(16, 4 * 1024 * 1024);
        assert_eq!(s.current_parallelism(), 1);
    }

    #[test]
    fn stub_scheduler_submit_runs_inline() {
        let s = RayonTaskScheduler::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        s.submit(move || {
            c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn stub_scheduler_submit_batch_runs_all() {
        let s = RayonTaskScheduler::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        let tasks: Vec<Box<dyn FnOnce() + Send + 'static>> = (0..5)
            .map(|_| {
                let c = counter.clone();
                Box::new(move || {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }) as Box<dyn FnOnce() + Send + 'static>
            })
            .collect();
        s.submit_batch(tasks);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 5);
    }

    #[test]
    fn stub_scheduler_wait_and_set_parallelism_are_noops() {
        let s = RayonTaskScheduler::new(2);
        s.wait();
        s.set_parallelism(8);
        assert_eq!(s.current_parallelism(), 1);
    }

    #[test]
    fn stub_scheduler_submit_batch_empty() {
        let s = RayonTaskScheduler::new(2);
        let tasks: Vec<Box<dyn FnOnce() + Send + 'static>> = vec![];
        s.submit_batch(tasks); // should not panic
    }

    #[test]
    fn create_default_scheduler_reports_at_least_one() {
        let s = create_default_scheduler();
        assert!(s.current_parallelism() >= 1);
    }

    // ---- Parallel-executor paths (only when feature is enabled) ----

    #[cfg(feature = "parallel-executor")]
    #[test]
    fn rayon_scheduler_creation_reports_parallelism() {
        let scheduler = RayonTaskScheduler::new(4);
        assert_eq!(scheduler.current_parallelism(), 4);
    }

    #[cfg(feature = "parallel-executor")]
    #[test]
    fn rayon_scheduler_submit_single_task() {
        let scheduler = RayonTaskScheduler::new(2);
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        scheduler.submit(move || {
            c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        scheduler.wait();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[cfg(feature = "parallel-executor")]
    #[test]
    fn rayon_scheduler_submit_batch() {
        let scheduler = RayonTaskScheduler::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        let tasks: Vec<Box<dyn FnOnce() + Send + 'static>> = (0..10)
            .map(|_| {
                let c = counter.clone();
                Box::new(move || {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }) as Box<dyn FnOnce() + Send + 'static>
            })
            .collect();
        scheduler.submit_batch(tasks);
        scheduler.wait();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 10);
    }

    #[cfg(feature = "parallel-executor")]
    #[test]
    fn rayon_scheduler_set_parallelism_changes_value() {
        let scheduler = RayonTaskScheduler::new(2);
        assert_eq!(scheduler.current_parallelism(), 2);
        scheduler.set_parallelism(8);
        assert_eq!(scheduler.current_parallelism(), 8);
    }

    #[cfg(feature = "parallel-executor")]
    #[test]
    fn rayon_scheduler_high_throughput_submit() {
        let scheduler = RayonTaskScheduler::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        for _ in 0..1000 {
            let c = counter.clone();
            scheduler.submit(move || {
                c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            });
        }
        scheduler.wait();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1000);
    }
}

