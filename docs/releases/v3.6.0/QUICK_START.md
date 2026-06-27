# v3.6.0 Quick Start

## Prerequisites
- Rust toolchain (stable)
- cargo-llvm-cov (for coverage)

## Build
```bash
cargo build --release --workspace
```

## Test
```bash
cargo test --lib --workspace --exclude sqlrustgo-mysql-server
```

## Lint
```bash
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check
```

## Coverage
```bash
# L1 8 crates average
for crate in sqlrustgo-types sqlrustgo-parser sqlrustgo-planner \
  sqlrustgo-optimizer sqlrustgo-executor sqlrustgo-storage \
  sqlrustgo-transaction sqlrustgo-catalog; do
  cargo llvm-cov test --package $crate --all-features --lib
done
```

## Current Status
Alpha Gate: CONDITIONAL PASS (81.97%)
Branch: develop/v3.6.0
