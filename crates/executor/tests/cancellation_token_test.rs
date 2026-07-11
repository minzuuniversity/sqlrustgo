//! CancellationToken integration tests
//!
//! v3.10.0 Issue #3703 Phase 4: Cancellation support

use sqlrustgo_executor::cancellation_token::{CancellationToken, ScopedCancellation};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

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

#[test]
fn test_token_clones_share_state() {
    let token1 = CancellationToken::new();
    let token2 = token1.clone();

    assert!(!token1.is_cancelled());
    assert!(!token2.is_cancelled());

    token1.cancel();

    assert!(token1.is_cancelled());
    assert!(token2.is_cancelled(), "Clone should see cancellation");
}
