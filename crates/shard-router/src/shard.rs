//! Shard endpoint representation.
//!
//! A `ShardSet` owns the list of `ShardEndpoint`s and provides health
//! round-robin selection per shard. Routing decisions are made by the
//! parser layer (`parser_ext`), not here.

use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::config::ShardRouterConfig;

/// Backend shard endpoint. `id` is the zero-based shard index used by
/// hash routing; `addr` is the host:port of the upstream
/// `sqlrustgo-mysql-server`.
#[derive(Debug, Clone)]
pub struct ShardEndpoint {
    pub id: usize,
    pub addr: String,
    pub in_flight: Arc<AtomicU64>,
}

impl ShardEndpoint {
    pub fn new(id: usize, addr: impl Into<String>) -> Self {
        Self {
            id,
            addr: addr.into(),
            in_flight: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn in_flight(&self) -> u64 {
        self.in_flight.load(Ordering::Relaxed)
    }
}

/// Owned by `run()` and shared per-connection. Cloning is cheap
/// (internally `Arc<[ShardEndpoint]>`).
#[derive(Clone)]
pub struct ShardSet {
    shards: Arc<[ShardEndpoint]>,
}

impl ShardSet {
    pub fn from_config(config: &ShardRouterConfig) -> io::Result<Self> {
        let shards: Vec<ShardEndpoint> = config
            .shards
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(|(i, addr)| ShardEndpoint::new(i, addr))
            .collect();

        if shards.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no shards configured (--shards host:port,host:port)",
            ));
        }

        Ok(Self {
            shards: Arc::from(shards.into_boxed_slice()),
        })
    }

    pub fn len(&self) -> usize {
        self.shards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shards.is_empty()
    }

    pub fn get(&self, id: usize) -> Option<&ShardEndpoint> {
        self.shards.get(id)
    }

    /// Iterate over all shards (used by broadcast fallback).
    pub fn iter(&self) -> impl Iterator<Item = &ShardEndpoint> {
        self.shards.iter()
    }
}
