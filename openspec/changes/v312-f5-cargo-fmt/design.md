# V312-F-5 Design: cargo fmt --all Fix

## Approach

The simplest, lowest-risk fix is to run `cargo fmt --all` once on the
workspace. This rewrites 183 files to the canonical Rust format. No
code behavior changes.

After applying, the gate's SGL-001 check will pass because:
1. `cargo fmt --check` exits 0 (no further diffs)
2. The git status hash before/after `cargo fmt --check` is identical
   (proving no silent auto-fix)

## Edge Cases

Macro-generated code may resist formatting. If any files fail after
`cargo fmt --all`, we apply `#[rustfmt::skip]` on those macro sites.
This is rare and unlikely given rustfmt's robustness in 2026-era Rust.

## Verification

Before: `cargo fmt --check` exit 1 (237 violations).
After: `cargo fmt --check` exit 0.