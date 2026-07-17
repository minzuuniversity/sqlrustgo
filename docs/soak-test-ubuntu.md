# SOAK Test - Ubuntu Linux x86_64 (HP Z440)

## Build
```bash
cargo build --release --package sqlrustgo-mysql-server
cargo build --release --package sqlrustgo-mysql-client --example hybrid_soak
```

## Run
Follow docs/soak-test-plan.md for execution instructions.

## Expected Results
| Metric | Expected |
|--------|----------|
| QPS | > 50/sec |
| Error Rate | < 1% |
| Memory | Stable (< 10GB RSS) |
