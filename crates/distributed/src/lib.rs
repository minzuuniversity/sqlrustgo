//! SQLRustGo distributed database support.
//!
//! This crate provides the read/write query classifier (`read_write_splitter`)
//! used by callers (e.g. server-side query routing) to decide whether a
//! SQL statement should be sent to a read replica or to the write primary.
//!
//! # Scope (v312-60)
//!
//! Only the `read_write_splitter` module is wired in this drop. The full
//! shard router, Raft consensus, 2PC coordinator, and gRPC server from
//! `archive/v3.11/deleted-crates/distributed/` are out of scope for v3.12.0
//! and will be reintroduced in the v3.13+ distributed-routing track.
//!
//! The module-level doc on `read_write_splitter` notes the SEM-1 / #3172
//! classification rule for `SAVEPOINT` / `ROLLBACK TO SAVEPOINT` /
//! `RELEASE SAVEPOINT` (treated as **Write**) and links the rationale.

pub mod read_write_splitter;
