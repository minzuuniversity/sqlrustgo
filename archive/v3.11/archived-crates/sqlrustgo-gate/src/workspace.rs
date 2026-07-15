//! Shared workspace root resolution for all checks.
//!
//! At compile time: uses CARGO_MANIFEST_DIR (set by cargo build/test).
//! At runtime:   resolves via current_exe() relative to the binary location.
//!
//! The binary lives at: target/release/sqlrustgo-gate
//! Two levels up from the binary = workspace root.

use std::path::PathBuf;

/// Returns the sqlrustgo workspace root.
pub fn workspace_root() -> PathBuf {
    // First try: CARGO_MANIFEST_DIR (compile-time, most reliable)
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let resolved = PathBuf::from(&manifest)
            .join("../..")
            .canonicalize()
            .expect("workspace root not found");
        if resolved.join("Cargo.toml").exists() {
            return resolved;
        }
    }

    // Fallback: resolve from binary location
    // Binary at: target/release/sqlrustgo-gate
    // Binary's parent.parent = workspace root
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if let Ok(resolved) = parent.join("../..").canonicalize() {
                if resolved.join("Cargo.toml").exists() {
                    return resolved;
                }
            }
        }
    }

    panic!("workspace root not found (tried CARGO_MANIFEST_DIR and current_exe)")
}
