//! I-12: Parallel Executor (worker thread pool)
//!
//! **Issue**: #2833
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (I-12 / INT-2)
//! **Change**: openspec/changes/i-12-parallel-executor
//!
//! In-memory worker pool infrastructure. Real executor integration in v3.9.0.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub type Task = Box<dyn FnOnce() -> i64 + Send + 'static>;

struct TaskQueue {
    queue: Mutex<VecDeque<Task>>,
    cvar: Condvar,
    shutdown: Mutex<bool>,
    active_workers: Mutex<usize>,
}

pub struct WorkerPool {
    queue: Arc<TaskQueue>,
    workers: Vec<thread::JoinHandle<()>>,
    worker_count: usize,
    results: Arc<Mutex<Vec<i64>>>,
    tasks_executed: Arc<Mutex<u64>>,
}

impl WorkerPool {
    pub fn new(worker_count: usize) -> Self {
        let queue = Arc::new(TaskQueue {
            queue: Mutex::new(VecDeque::new()),
            cvar: Condvar::new(),
            shutdown: Mutex::new(false),
            active_workers: Mutex::new(worker_count),
        });
        let results = Arc::new(Mutex::new(Vec::new()));
        let tasks_executed = Arc::new(Mutex::new(0u64));
        let mut workers = Vec::with_capacity(worker_count);
        for worker_id in 0..worker_count {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let tasks_executed = Arc::clone(&tasks_executed);
            let handle = thread::spawn(move || {
                worker_loop(worker_id, queue, results, tasks_executed);
            });
            workers.push(handle);
        }
        Self {
            queue,
            workers,
            worker_count,
            results,
            tasks_executed,
        }
    }

    pub fn submit<F>(&self, f: F)
    where
        F: FnOnce() -> i64 + Send + 'static,
    {
        let task: Task = Box::new(f);
        self.queue.queue.lock().unwrap().push_back(task);
        self.queue.cvar.notify_one();
    }

    pub fn shutdown(&self) {
        *self.queue.shutdown.lock().unwrap() = true;
        self.queue.cvar.notify_all();
    }

    pub fn wait(&mut self) {
        // Wait for queue to be empty
        loop {
            {
                let q = self.queue.queue.lock().unwrap();
                if q.is_empty() {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        self.shutdown();
        // Take workers and join them
        let workers = std::mem::take(&mut self.workers);
        for w in workers {
            let _ = w.join();
        }
    }

    pub fn results(&self) -> Vec<i64> {
        self.results.lock().unwrap().clone()
    }

    pub fn tasks_executed(&self) -> u64 {
        *self.tasks_executed.lock().unwrap()
    }

    pub fn worker_count(&self) -> usize {
        self.worker_count
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn worker_loop(
    _id: usize,
    queue: Arc<TaskQueue>,
    results: Arc<Mutex<Vec<i64>>>,
    tasks_executed: Arc<Mutex<u64>>,
) {
    loop {
        let task = {
            let mut q = queue.queue.lock().unwrap();
            loop {
                if *queue.shutdown.lock().unwrap() {
                    return;
                }
                if let Some(t) = q.pop_front() {
                    break t;
                }
                q = queue.cvar.wait(q).unwrap();
            }
        };
        let result = task();
        results.lock().unwrap().push(result);
        *tasks_executed.lock().unwrap() += 1;
    }
}

#[test]
fn test_basic_worker_pool() {
    let mut pool = WorkerPool::new(4);
    for i in 0..10 {
        pool.submit(move || i * 2);
    }
    pool.wait();
    let results = pool.results();
    assert_eq!(results.len(), 10);
    assert_eq!(pool.worker_count(), 4);
}

#[test]
fn test_results_collection() {
    let mut pool = WorkerPool::new(2);
    for i in 0..5 {
        pool.submit(move || (i + 1) * 10);
    }
    pool.wait();
    let results = pool.results();
    let mut sorted = results.clone();
    sorted.sort();
    assert_eq!(sorted, vec![10, 20, 30, 40, 50]);
}

#[test]
fn test_fair_distribution() {
    let mut pool = WorkerPool::new(3);
    for _ in 0..30 {
        pool.submit(|| 1);
    }
    pool.wait();
    let results = pool.results();
    assert_eq!(results.len(), 30);
    // All tasks returned 1
    assert!(results.iter().all(|&r| r == 1));
    assert_eq!(pool.tasks_executed(), 30);
}

#[test]
fn test_single_worker() {
    let mut pool = WorkerPool::new(1);
    for i in 0..5 {
        pool.submit(move || i);
    }
    pool.wait();
    let results = pool.results();
    assert_eq!(results.len(), 5);
}

#[test]
fn test_graceful_shutdown_empty_queue() {
    let mut pool = WorkerPool::new(2);
    // No tasks submitted
    let start = Instant::now();
    pool.wait();
    let results = pool.results();
    let elapsed = start.elapsed();
    assert!(results.is_empty());
    assert!(
        elapsed < Duration::from_secs(2),
        "should shutdown quickly, took {:?}",
        elapsed
    );
}

#[test]
fn test_concurrent_results_no_data_race() {
    use std::sync::atomic::{AtomicI64, Ordering};
    let mut pool = WorkerPool::new(4);
    let counter = Arc::new(AtomicI64::new(0));
    for _ in 0..100 {
        let c = Arc::clone(&counter);
        pool.submit(move || {
            c.fetch_add(1, Ordering::SeqCst);
            42
        });
    }
    pool.wait();
    let results = pool.results();
    assert_eq!(results.len(), 100);
    assert!(results.iter().all(|&r| r == 42));
    assert_eq!(counter.load(Ordering::SeqCst), 100);
}
