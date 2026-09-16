//! sqlrustgo shard router library.
//!
//! Lightweight MySQL-protocol-aware proxy that hashes the partition key
//! of incoming queries and forwards to one of N `sqlrustgo-mysql-server`
//! shards. Cross-shard queries broadcast to all shards. Single-master
//! per shard, no Raft/2PC.
//!
//! Design goals (v4.0.0 POC scope):
//! - Stateless TCP forwarder except for shard topology config
//! - PK-aware hash routing for `WHERE pk = ?` point queries (hot path)
//! - Per-statement broadcast fallback for `INSERT/UPDATE/DELETE/SELECT`
//!   without WHERE pk = ?
//! - Health-aware load balancing (per-shard counter, fails open)
//!
//! Out of scope (would be v5.0.0 work per PHASE_B_HORIZONTAL_SCALING_POC.md):
//! - Distributed transactions across shards
//! - Shard rebalancing / online schema changes
//! - Read-write splitting (already in crates/distributed)
//! - TLS termination (delegated to upstream load balancer)

use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::sync::CancellationToken;

pub mod config;
pub mod hash;
pub mod myproxy;
pub mod parser_ext;
pub mod shard;

pub use config::ShardRouterConfig;
pub use shard::{ShardEndpoint, ShardSet};

/// Run the router until the cancellation token fires.
///
/// Spawns one task per accepted connection. `local_addr` is populated
/// when the listener is bound (used for log output).
pub async fn run(
    config: ShardRouterConfig,
    cancel: CancellationToken,
) -> std::io::Result<std::net::SocketAddr> {
    let bind_addr: std::net::SocketAddr = config.listen_addr.parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("listen_addr parse: {e}"),
        )
    })?;
    let listener = TcpListener::bind(bind_addr).await?;
    let local_addr = listener.local_addr()?;
    let shards = Arc::new(ShardSet::from_config(&config)?);

    tracing::info!(
        target: "shard_router",
        listen = %local_addr,
        shards = shards.len(),
        "shard router accepting connections"
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                tracing::info!(target: "shard_router", "shutdown requested, ceasing accept");
                break;
            }
            accept = listener.accept() => {
                let (stream, peer) = match accept {
                    Ok(x) => x,
                    Err(e) => {
                        tracing::warn!(target: "shard_router", error = %e, "accept error, continuing");
                        continue;
                    }
                };
                let shards = Arc::clone(&shards);
                let conn_cancel = cancel.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, shards, conn_cancel).await {
                        tracing::debug!(target: "shard_router", peer = %peer, error = %e, "connection ended");
                    }
                });
            }
        }
    }

    Ok(local_addr)
}

async fn handle_connection(
    downstream: TcpStream,
    shards: Arc<ShardSet>,
    cancel: CancellationToken,
) -> std::io::Result<()> {
    let session = myproxy::Session::new(downstream, shards, cancel);
    session.run().await
}
