//! CancellationToken - per-query cancellation support for parallel execution
//!
//! v3.10.0 Issue #3703 Phase 4: Cancellation support
//!
//! This module provides a CancellationToken that can be used to cancel
//! long-running parallel queries. It integrates with the PipelineExecutor
//! to provide graceful shutdown of parallel tasks.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

/// CancellationToken - signals cancellation of a parallel query
///
/// Each parallel query gets its own CancellationToken. The token can be
/// checked by worker tasks to detect cancellation, and the parent can
/// call `cancel()` to request shutdown.
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync>>>>,
}

impl CancellationToken {
    /// Create a new CancellationToken
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            callbacks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Cancel the operation
    pub fn cancel(&self) {
        if !self.cancelled.swap(true, Ordering::SeqCst) {
            debug!("Cancellation requested");
            // Invoke all registered callbacks
            let callbacks = self.callbacks.lock();
            for callback in callbacks.iter() {
                callback();
            }
        }
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Register a callback to be invoked on cancellation
    pub fn on_cancel(&self, callback: Box<dyn Fn() + Send + Sync>) {
        let mut callbacks = self.callbacks.lock();
        if self.is_cancelled() {
            // Already cancelled, invoke immediately
            callback();
        } else {
            callbacks.push(callback);
        }
    }

    /// Wait for cancellation with a timeout
    ///
    /// Returns true if cancellation occurred, false if timeout elapsed.
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        let start = std::time::Instant::now();
        while !self.is_cancelled() {
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        true
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// ScopedCancellation - automatically cancels on drop (RAII pattern)
pub struct ScopedCancellation {
    token: CancellationToken,
}

impl ScopedCancellation {
    /// Create a new ScopedCancellation
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Get the cancellation token
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl Drop for ScopedCancellation {
    fn drop(&mut self) {
        if !self.token.is_cancelled() {
            warn!("ScopedCancellation dropped without explicit cancel - cancelling");
            self.token.cancel();
        }
    }
}

impl Default for ScopedCancellation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_basic_cancellation() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());

        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancel_callbacks() {
        let token = CancellationToken::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();
        token.on_cancel(Box::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        assert_eq!(counter.load(Ordering::SeqCst), 0);
        token.cancel();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_cancel_after_callback_registered() {
        let token = CancellationToken::new();
        let counter = Arc::new(AtomicUsize::new(0));

        token.cancel(); // Cancel first

        let counter_clone = counter.clone();
        token.on_cancel(Box::new(move || {
            // Should be invoked immediately
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_scoped_cancellation() {
        let counter = Arc::new(AtomicUsize::new(0));
        let token_holder;

        {
            let scoped = ScopedCancellation::new();
            let token = scoped.token();
            token_holder = token.clone();

            let counter_clone = counter.clone();
            token.on_cancel(Box::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }));

            assert_eq!(counter.load(Ordering::SeqCst), 0);
        } // Drop here should trigger cancel

        assert!(token_holder.is_cancelled());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_wait_timeout() {
        let token = CancellationToken::new();
        let result = token.wait_timeout(Duration::from_millis(10));
        assert!(!result, "Should timeout when not cancelled");

        token.cancel();
        let result = token.wait_timeout(Duration::from_millis(10));
        assert!(result, "Should detect cancellation immediately");
    }

    #[test]
    fn test_multiple_cancellations_idempotent() {
        let token = CancellationToken::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();
        token.on_cancel(Box::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));

        token.cancel();
        token.cancel(); // Second cancel should be no-op
        token.cancel();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
