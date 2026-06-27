# 08 - License Compliance

## License Summary

| Component | License | Compatible |
|-----------|---------|------------|
| SQLRustGo source code | Apache-2.0 OR MIT | ✅ (permissive) |
| All workspace crates | Apache-2.0 OR MIT | ✅ |
| All dependencies (top 50) | MIT/Apache-2.0/BSD/ISC | ✅ |
| No GPL/AGPL/LGPL deps | n/a | ✅ |

## License Verification

```bash
# Verify no copyleft licenses in deps
cargo deny check license
# Result: 0 violations
```

## Source Files

All first-party code in this repository:
- Dual-licensed under Apache-2.0 OR MIT
- Contributors retain copyright
- See `LICENSE-APACHE` and `LICENSE-MIT` for full text

## Dependencies (sample top 10)

| Crate | License |
|-------|---------|
| tokio | MIT |
| rustls | Apache-2.0/ISC/MIT |
| serde | Apache-2.0/MIT |
| clap | Apache-2.0/MIT |
| chrono | Apache-2.0/MIT |
| bincode | MIT |
| crc32fast | Apache-2.0/MIT |
| zstd | MIT |
| lz4 | Apache-2.0/MIT |
| uuid | Apache-2.0/MIT |

All MIT/Apache-2.0/BSD-compatible. No GPL contamination.

## Third-party Code

No third-party code copied into source. All functionality implemented from scratch.

## Trademark

"SQLRustGo" is a project name, not a registered trademark. No trademark concerns.
