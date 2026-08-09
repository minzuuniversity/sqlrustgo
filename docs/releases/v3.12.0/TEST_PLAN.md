# SQLRustGo v3.12.0 Test Plan

> **Version**: v3.12.0
> **Status**: PLANNED
> **Date**: 2026-08-09
> **Target**: GMP internal-audit retrieval production gate

## 1. Test Matrix

| Gate | Area | Method | Threshold |
|---|---|---|---|
| V312-G1 | Core SQL build/test | cargo build/test/fmt/clippy | exit 0, 0 warnings |
| V312-G2 | v3.11 carried blockers | stage + debt + TPC-H + coverage + sqllogictest evidence | no unresolved hidden P0 |
| V312-G3 | GMP schema | schema unit/integration tests | document/chunk/embedding/audit/relation tables valid |
| V312-G4 | Corpus ingestion | full `~/gmp-platform/gmp-md` import | 0 unclassified failures |
| V312-G5 | Vector retrieval | fixed retrieval fixture | deterministic top-k and rebuildable index |
| V312-G6 | Hybrid retrieval | GMP internal-audit question set | citation bundle on every result |
| V312-G7 | Graph projection | node/edge/path tests | deterministic counts and relation-filtered paths |
| V312-G8 | Compliance controls | ACL, e-signature hook, audit chain | tamper tests fail closed |
| V312-G9 | Backup/restore | restore full GMP/RAG/projection store | hash and count equality |
| V312-G10 | Mixed SOAK | 168h SQL + ingest + retrieval + audit | 0 crash, no audit-chain break |
| V312-G11 | SQLite SQLLogicTest oracle | `sqlrustgo_sqllogictest` runner + SQLite official/cached corpus | selected targets PASS or issue-linked exclusion |
| V312-G12 | TPC-H correctness close-out | SF=1 SQLRustGo vs SQLite/PostgreSQL/MySQL row-count + SHA256 | no unexplained zero-row or checksum mismatch |
| V312-G13 | MySQL wire protocol hardening | COM_QUERY/COM_STMT/error/reset/TLS/compression e2e | deterministic pass/fail artifact |
| V312-G14 | LOAD DATA / bulk import | SF=1/SF=10 import benchmark + memory cap | no OOM; row-count/hash equality |
| V312-G15 | Crash recovery and upgrade | kill -9, WAL replay, backup/restore, v3.10->v3.12 upgrade/downgrade | count/hash equality after recovery |

## 2. Required Test Assets

| Asset | Source | Use |
|---|---|---|
| Representative GMP corpus | `~/gmp-platform/gmp-md` | ingestion and retrieval |
| Internal-audit question set | new `docs/releases/v3.12.0/fixtures/gmp_audit_questions.yml` | retrieval quality |
| Embedding fixture | generated from fixed model/provider | deterministic vector tests |
| Graph relation fixture | generated from SOP/CAPA/deviation samples | graph projection |
| Tamper fixture | synthetic modified audit event | audit-chain fail-closed |
| SQLLogicTest smoke corpus | `crates/sqlrustgo_sqllogictest/testdata` | runner build/run and local compatibility gate |
| SQLite official SQLLogicTest corpus | SQLite upstream snapshot or reproducible local mirror/cache | broad SQL oracle testing |
| TPC-H SF=1 fixture | dbgen/BINT fixture with row-count manifest | correctness and performance close-out |
| Wire protocol fixture | prepared statement, error packet, reset, TLS/compression scripts | MySQL compatibility hardening |
| Recovery fixture | WAL/data snapshots across v3.10/v3.11/v3.12 | crash recovery and upgrade verification |

## 3. Evidence Requirements

Every PASS claim must include:

- command
- timestamp
- source agent
- source run
- evidence hash
- output location
- PASS/FAIL boundary

## 4. Rejection Rules

v3.12.0 must not enter GA if any of these are true:

- v3.11.0 G3/G4/SOAK blockers are silently ignored.
- SQLLogicTest remains only a document/TBD item and is not integrated into a real gate.
- The SLT runner path remains ambiguous between `crates/sqllogictest` and `crates/sqlrustgo_sqllogictest`.
- SQLite official/cached corpus acquisition has no manifest, hash, file count, or exclusion policy.
- GMP corpus import has unclassified failures.
- Search result lacks source path, version, chunk hash, or citation text.
- Audit hash-chain tamper test does not fail.
- Vector index cannot rebuild from SQLRustGo-managed data.
- Graph projection requires archived `graph` crate as an unvalidated production dependency.
- 168h mixed SOAK is not completed or is described as PASS without evidence.
- TPC-H SF=1 zero-row or checksum mismatch is explained only by text, without cross-engine output.
- `LOAD DATA` or backup/restore claims are made without count/hash evidence.

## 5. SQLLogicTest / SQLite Oracle Gate

The SQLite automatic testing framework is a carried-forward P0 item from v3.10.0. It must become executable in v3.12.0.

| Stage | Required SLT evidence |
|---|---|
| Alpha | `cargo build -p sqlrustgo_sqllogictest` succeeds; runner `--help` prints usable options |
| Beta | Local smoke corpus under `crates/sqlrustgo_sqllogictest/testdata` runs and writes a report |
| RC | Curated SQLite-compatible subset runs with PASS/FAIL/SKIP classification and issue-linked exclusions |
| GA | All selected SLT targets pass, or every skipped/failed group has an issue, owner, expiry, and rationale |

Required commands:

```bash
cargo build -p sqlrustgo_sqllogictest
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata
bash scripts/gate/check_sqllogictest_v312.sh
```

Current baseline captured on 2026-08-09:

| Command | Observed result | Test-plan implication |
|---|---|---|
| `cargo build -p sqlrustgo_sqllogictest` | Build completes; warnings are still emitted from dependent crates | Acceptable only as initial Alpha build evidence, not as clippy/warning-free evidence |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | Runner completes with 6/16 files passing and 27.3% pass rate | v3.12.0 must triage failures, classify expected incompatibilities, and raise the smoke gate before Beta/RC |
| `bash scripts/gate/check_sqllogictest_v312.sh` | Script not yet present in the current plan baseline | Must be implemented before the SQLLogicTest gate can be called integrated |

Required artifacts:

| Artifact | Path |
|---|---|
| SLT smoke report | `docs/releases/v3.12.0/sqllogictest-baseline/smoke-report.md` |
| SQLite corpus manifest | `docs/releases/v3.12.0/sqllogictest-baseline/sqlite-corpus-manifest.json` |
| Exclusion registry | `docs/releases/v3.12.0/sqllogictest-baseline/exclusions.yml` |
| Gate output | `docs/releases/v3.12.0/logs/sqllogictest_<commit>_<timestamp>.log` |

## 6. v3.11 Weak-Point Regression Plan

| Weak point from v3.11.0 assessment | v3.12 regression test |
|---|---|
| TPC-H zero-row query correctness | Cross-engine row-count and SHA256 compare for all 22 SF=1 queries |
| G3 coverage口径漂移 | Single canonical coverage command, stored output, no mixed PASS claims |
| MySQL wire protocol低覆盖 | COM_QUERY, COM_STMT_PREPARE/EXECUTE/CLOSE, error packet, reset, TLS/compression e2e |
| LOAD DATA未验证 | SF=1/SF=10 import, row counts, hashes, memory/time cap |
| Crash recovery证据不足 | kill -9, WAL replay, dirty page recovery, backup/restore checksum |
| Upgrade/downgrade证据不足 | v3.10.0/v3.11.0 fixture upgrade to v3.12.0 and rollback verification |
| GMP/RAG/Vector/Graph生产缺口 | GMP corpus, retrieval quality, vector rebuild, graph projection, ACL and audit-chain gates |
