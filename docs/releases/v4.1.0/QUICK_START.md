# SQLRustGo v4.1.0 — Quick Start

> **Date**: 2026-10-23
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Purpose**: 5-minute quick start for developers and evaluators of v4.1.0.
> **Branch**: `develop/v4.1.0`

## 1. What you need

| Component | Requirement |
|---|---|
| Rust toolchain | 1.75+ (stable) |
| Cargo | 1.75+ |
| OS | Linux (x86_64), macOS (aarch64/x86_64), Windows (x86_64) |
| C compiler | gcc / cc (for `ring` crate, used by `rustls`) |
| Disk | ~2 GB for build artifacts (`target/`) |

## 2. Clone and build

```bash
# 1. Clone (pick your remote)
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout develop/v4.1.0

# 2. Build (all features)
cargo build --all-features

# 3. Run tests
cargo test --all-features

# 4. Run the binary
cargo run --all-features -- serve --port 3306
```

The server starts on port 3306 (MySQL wire protocol) by default.

## 3. Connect

```bash
# Using mysql client
mysql -h 127.0.0.1 -P 3306 -u root

# Or using the built-in CLI
cargo run --all-features -- cli "SELECT 1;"
```

## 4. Run a basic query

```sql
CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100), email VARCHAR(200));
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');
SELECT * FROM users WHERE id = 1;
-- Returns: (1, Alice, alice@example.com)
```

## 5. Run the gate checks

```bash
# Lint
cargo clippy --all-features -- -D warnings
cargo fmt --check --all

# Coverage
bash scripts/gate/check_coverage.sh

# Document completeness
bash scripts/gate/check_document_completeness.sh v4.1.0 /tmp/doccheck/

# Security scan
cargo audit --json
```

## 6. Run the SOAK test (1h)

```bash
bash scripts/soak/run_soak_loop.sh --duration 3600
```

This runs a 1-hour OLTP SOAK test with heartbeat monitoring.

## 7. What's new in v4.1.0 (vs v4.0.0)

- **PK fast-path**: `scan_pk` forwarding in `WalStorage`/`ParallelWalStorage` (#5168)
  — primary key point lookups are now 19× faster (9.02ms → 0.467ms at 20K rows)
- **UPDATE/DELETE WHERE fast-path** (#5177) — 512× → 49× improvement
- **TLS read spin fix** (#5099) — server no longer busy-loops at 1334% CPU
- **Concurrent transaction fix** (#5099 partial) — row loss reduced from 40.4% to ~1%
- **5-remote sync tooling** — `scripts/sync/5remotes_sync.sh`
- **O(rows) delete allocation fix** (#5181 partial) — `Vec::retain` in-place compression

## 8. Known limitations

v4.1.0 is in **DRAFT** stage. The following are inherited from v4.0.0 and
not yet resolved:

- WP-C DDL/Integrity: 6 issues (#4682, #4652, #4672, #4669, #4709, #4703)
- WP-D Join/Subquery: 4 issues (#4668, #4656, #4649, #4636)
- WP-E Transaction semantics: 1 issue (#4626)
- WP-F Schema migration: 1 issue (#4848)
- WP-G Type/Comparison: 1 issue (#4846)
- WP-B completion: 6 issues (#4721, #4674, #4716, #4676, #4675, #4670)

See `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` for the full defect catalog.

## 9. Where to go next

- **Architecture**: `docs/releases/v4.1.0/ARCHITECTURE.md`
- **Development plan**: `docs/releases/v4.1.0/DEV_PLAN.md`
- **Test plan**: `docs/releases/v4.1.0/TEST_PLAN.md`
- **Issue catalog**: `docs/releases/v4.1.0/ISSUES_PLAN.md`
- **Release notes**: `docs/releases/v4.1.0/RELEASE_NOTES.md`
- **Feature matrix**: `docs/releases/v4.1.0/FEATURE_MATRIX.md`
- **Installation**: `docs/releases/v4.1.0/INSTALL.md`
- **Deployment**: `docs/releases/v4.1.0/DEPLOYMENT_GUIDE.md`
- **Migration**: `docs/releases/v4.1.0/MIGRATION_GUIDE.md`
- **Evaluation**: `docs/releases/v4.1.0/EVALUATION_REPORT.md`

## 10. Troubleshooting

### Build failures

| Symptom | Likely cause | Fix |
|---|---|---|
| `error: failed to compile ring` | Missing C compiler | Install `gcc` or `build-essential` |
| `error: linker not found` | Missing linker | Install `gcc` or `clang` |
| Out of disk space | `target/` too large | Run `cargo clean` or increase disk |

### Runtime failures

| Symptom | Likely cause | Fix |
|---|---|---|
| Port 3306 already in use | Another MySQL running | Change port: `--port 3307` |
| `Permission denied` on data dir | Wrong ownership | `chmod 755` the data directory |
| Server hangs on first query | TLS read spin (pre-#5099 fix) | Ensure you're on HEAD of develop/v4.1.0 |

### Test failures

| Symptom | Likely cause | Fix |
|---|---|---|
| `cargo test` timeout | SOAK test too long | Use `--lib` to skip integration tests |
| Coverage below 80% | New code not tested | Add tests for new modules |
| Gate check FAIL | Missing mandatory docs | See `docs/governance/DOC_CHECK_CORRECTION_RULES.md` |

## 11. Performance quick reference

| Metric | v4.0.0 (pre-fix) | v4.1.0 (post-fix) | Improvement |
|---|---:|---:|---:|
| PK point lookup (20K rows) | 9.02 ms | 0.467 ms | 19× |
| UPDATE/SELECT cost ratio (20K) | 512× | 49× | 10.4× |
| sysbench 8-client TPS | 13.89 | 38.53 | 2.8× |
| sysbench 8-client QPS | 277.79 | 770.52 | 2.8× |
| 8h SOAK TPS (avg) | — | 12.75 | baseline |
| 8h SOAK CPU (max) | — | 174% | no spin |

Data source: `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md`,
`evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md`,
`evidence/CONCURRENT_TX_ROLLBACK_DATA_LOSS_5099_2026-10-08.md`.

## 12. FAQ

**Q: Can I use v4.1.0 in production?**
A: No. v4.1.0 is in DRAFT stage. v4.0.0 GA was revoked on 2026-09-30.
Use v4.0.0 release branch for production until v4.1.0 reaches GA.

**Q: What's the difference between v4.1.0 and v4.0.0?**
A: v4.1.0 is the post-v4.0.0 maintenance continuation. It carries forward
v4.0.0's bugfixes, adds performance improvements (PK fast-path, UPDATE/DELETE
fast-path), and addresses WP-C..G deferred items. See
`docs/releases/v4.1.0/VERSION_PLAN.md` for the full delta.

**Q: How do I run the full gate suite?**
A: `bash gate/gate.sh v4.1.0` runs all L1 gates. Individual gates are in
`scripts/gate/`. See `docs/governance/GATE_CONDITIONS.md` for gate definitions.

**Q: How do I contribute?**
A: Create a branch from `develop/v4.1.0`, submit a failing test first, then
the fix. See `docs/releases/v4.1.0/DEV_PLAN.md` §4 for the development rules.

## 13. References

- `docs/releases/v4.1.0/STAGE.yaml` — stage SSOT
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` — defect catalog
- `docs/releases/v4.1.0/RELEASE_NOTES.md` — release notes
- `docs/releases/v4.1.0/CHANGELOG.md` — changelog
- `docs/releases/v4.1.0/ARCHITECTURE.md` — architecture
- `docs/governance/DOC_CHECK_CORRECTION_RULES.md` — doc completeness rules
- `docs/governance/GATE_CONDITIONS.md` — gate conditions
- `scripts/sync/README.md` — 5-remote sync tooling
