# 07 - Dependency Audit

## Major Dependencies (Workspace)

| Crate | Version | License | Source |
|-------|---------|---------|--------|
| `tokio` | 1.40 | MIT | LTS async runtime |
| `rustls` | 0.23 | Apache-2.0/ISC/MIT | TLS 1.3 |
| `serde` | 1.0 | Apache-2.0/MIT | Serialization |
| `clap` | 4.5 | Apache-2.0/MIT | CLI args |
| `chrono` | 0.4 | Apache-2.0/MIT | Time handling |
| `bincode` | 1.3 | MIT | Binary serialization |
| `crc32fast` | 1.4 | Apache-2.0/MIT | Checksums |
| `zstd` | 0.13 | MIT | Compression |
| `lz4` | 1.24 | Apache-2.0/MIT | Compression |
| `uuid` | 1.x | Apache-2.0/MIT | UUIDs |

## Workspace Members (39 crates)

All workspace members are first-party SQLRustGo code:
- `parser`, `lexer`, `types`, `storage`, `transaction`, `network`
- `executor`, `planner`, `optimizer`, `catalog`
- `vector`, `graph`, `columnar`, `wal`
- `mysql-server`, `server`, `bench`, `audit`
- ... (20+ more)

## Dependency Update Strategy

- **Major bumps**: tracked in `RELEASE_NOTES.md §9`
- **Security updates**: applied within 7 days of disclosure
- **License compliance**: all deps MIT/Apache-2.0 compatible
- **Abandoned deps**: 0 found

## Cargo.lock Status

- **Locked**: yes (`Cargo.lock` committed)
- **Total deps**: 591 (transitive)
- **Direct deps**: ~80
- **Last audit**: 2026-06-13
