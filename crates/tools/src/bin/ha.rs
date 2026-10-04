//! HA CLI entry point.
//!
//! The command surface lives in `sqlrustgo_tools::ha`; this file only
//! supplies `main`, matching the pattern `physical_backup` already uses.
//! `ha.rs` was not declared in `lib.rs` at all, so neither the module
//! nor its three tests had ever been compiled (#4939).

fn main() -> anyhow::Result<()> {
    sqlrustgo_tools::ha::run()
}
