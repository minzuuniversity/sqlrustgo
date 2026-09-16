//! Configuration parsing for the shard router.

use clap::Parser;
use std::path::PathBuf;

/// Configuration for the shard router binary.
///
/// The router is a stateless MySQL-protocol forwarder. The shard
/// topology is passed via the CLI; in production, this would come
/// from a service registry, but for v4.0.0 POC scope we accept
/// inline config.
#[derive(Debug, Clone, Parser)]
#[command(name = "sqlrustgo-shard-router", about = "MySQL-protocol shard router for sqlrustgo")]
pub struct ShardRouterConfig {
    /// Address to bind the router to (e.g. 0.0.0.0:3306).
    #[arg(long, env = "SHARD_ROUTER_LISTEN")]
    pub listen_addr: String,

    /// Comma-separated list of shard endpoints (host:port,host:port,...).
    /// The router hashes the partition key modulo len(shards).
    #[arg(long, env = "SHARD_ROUTER_SHARDS")]
    pub shards: String,

    /// Default schema (database) to use when clients don't specify
    /// one. Most deployments pin a single shard-per-schema.
    #[arg(long, env = "SHARD_ROUTER_DEFAULT_DB", default_value = "")]
    pub default_db: String,

    /// Default username for backend shard connections when client
    /// doesn't supply one.
    #[arg(long, env = "SHARD_ROUTER_DEFAULT_USER", default_value = "")]
    pub default_user: String,

    /// Optional file with shard rewrite map. Used by tests to inject
    /// deterministic shard assignments.
    #[arg(long, env = "SHARD_ROUTER_OVERRIDE_FILE")]
    pub override_file: Option<PathBuf>,

    /// Comma-separated list of broadcast-allowed tables (queries
    /// against these tables always fan out to every shard).
    /// Default: empty (= all non-PK queries broadcast).
    #[arg(long, env = "SHARD_ROUTER_BROADCAST_TABLES", default_value = "")]
    pub broadcast_tables: String,

    /// Logging filter (RUST_LOG-style).
    #[arg(long, env = "SHARD_ROUTER_LOG", default_value = "info")]
    pub log: String,
}
