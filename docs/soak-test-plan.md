# SQLRustGo 168h SOAK Test Plan

## Overview
Long-running stability test (168 hours / 7 days) for SQLRustGo v3.11.0 using TPC-H SF=0.1 dataset.

## Test Configuration

### Environment
- OS: macOS (Darwin) / Ubuntu Linux x86_64 (HP Z440)
- Rust Version: 2024 edition with Tokio async runtime
- Build Target: Release (`--release`)

### Binary Configuration
- Server Binary: `target/release/sqlrustgo-mysql-server`
- Test Client: `target/release/examples/hybrid_soak`
- Storage: Binary format (no WAL)
- Server Threads: 8
- Test Port: 3396

## Test Parameters
| Parameter | Value |
|-----------|-------|
| Duration | 604800 seconds (168 hours) |
| Target QPS | 50 per thread (200 total) |
| Threads | 4 |
| OLTP Ratio | 0.32 |
| Report Interval | 300 seconds |

## Prerequisites
```bash
cargo build --release --package sqlrustgo-mysql-server
cargo build --release --package sqlrustgo-mysql-client --example hybrid_soak
```

## Execution
```bash
# Server
./target/release/sqlrustgo-mysql-server serve --port 3396 --data-dir /tmp/sqlrustgo-v311-soak --storage binary --server-threads 8 --log-level error

# Client
./target/release/examples/hybrid_soak --host 127.0.0.1 --port 3396 --threads 4 --duration 604800 --target-qps 50 --report-interval 300
```

## Success Criteria
- Uptime: > 168h
- Error Rate: < 1%
- QPS: > 25/sec

## Related Issues
- Issue #3500: V311-21: 168h SOAK v3.11.0
