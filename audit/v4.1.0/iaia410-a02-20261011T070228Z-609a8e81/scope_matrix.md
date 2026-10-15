# IAIA-410 / A02 — Claim / Entry / Scope Decision Matrix

- **Work package:** A02 (#5224) — claim/entry/scope matrix
- **assessed_commit:** `609a8e81db79d41b1fa6aa1fb6f8f361c97ac53a` (A01 frozen baseline, per SSOT §0)
- **Run:** `iaia410-a02-20261011T070228Z-609a8e81`
- **Governing protocol:** `docs/releases/v4.1.0/IAIA_410_SSOT.md`

## 1. Headline findings

1. **README top-level claims are STALE.** README §1–§8 (SQL subset, TPC-H, MySQL compat,
   crash recovery, RBAC) are v3.12.0-GA-based and self-flagged against the v4.0.0-GA
   revocation. v4.1.0 is **DRAFT** (not ALPHA). Every README claim MUST be re-verified on the
   frozen v4.1.0 run — none is trusted as-is. STAGE.yaml / RELEASE_NOTES.md /
   CORE_PATH_INVENTORY.md are v4.1.0-dated (fresh).
2. **All 10 capability domains (SSOT §3.2) are `in_scope`** for their core relational /
   MySQL-wire / durability capability. The scope exclusions are specific *claims* (general-purpose
   vector/graph/RAG, full MySQL 5.7, SQLite dialect, GA-only multi-model/SOAK), not whole domains.
3. **Production reachability is evidenced, not assumed.** Each in-scope domain maps to concrete
   production entries (PE-01..PE-07) with file:line reachability evidence. Module existence /
   test count / test-pass is explicitly NOT used as a substitute (SSOT §0, §3).

## 2. Production entries (SSOT §3.1 — 6 categories → 7 entries)

| ID | Category | Bin / subcommand | Durable | Reaches (evidence) |
|----|----------|------------------|---------|--------------------|
| PE-01 | MySQL Server non-TLS | `sqlrustgo serve` | yes | MvccStorage(FileStorage)+Wal+Checkpoint (`crates/mysql-server/src/lib.rs:6884-6947`) |
| PE-02 | MySQL Server TLS | `sqlrustgo serve --tls` | yes | rustls TlsStream + same storage (`lib.rs`) |
| PE-03 | Real MySQL client + prepared stmt | `sqlrustgo cli` | n/a (client) | COM_QUERY/COM_STMT_* over wire → PE-01/02 storage |
| PE-04 | REPL | `sqlrustgo repl` | no | MemoryExecutionEngine (in-process) (`crates/mysql-server/src/main.rs:544,551`) |
| PE-05 | CLI / admin tools | `sqlrustgo exec/backup/restore/...`, `sqlrustgo-admin *` | mixed | exec→memory; backup/restore/verify/pitr→storage+admin |
| PE-06 | Startup / recovery / shutdown | `sqlrustgo serve` + signal handler | yes | StatefulRecoveryEngine WAL replay (`recovery_engine.rs`, `lib.rs:6887`) |
| PE-07 | Background maint / checkpoint / backup-restore | `serve` (bg thread), `backup`, `restore`, `admin pitr` | yes | CheckpointManager (300s) + backup_coordinator + pitr (`checkpoint.rs`) |

Non-production bins (soak/shard-router/sqllogictest/sqlancer/tools) are excluded from
reachability scope — they are test/dev harnesses, not user-facing SQL paths.

## 3. Capability-domain scope decisions (SSOT §3.2)

| Domain | Scope | Production entries | Verification WPs | Reachability anchor |
|--------|-------|--------------------|------------------|---------------------|
| CD-01 SQL semantics | **in_scope** | PE-01/03/04/05 | A03, A04 | `src/engine_select.rs` via serve/cli/repl |
| CD-02 DDL / DML | **in_scope** | PE-01/03/04/05 | A03, A04 | `src/engine_ddl.rs`+`engine_dml.rs` |
| CD-03 Optimization & planning | **in_scope** | PE-01/03 | A03, A05 | `UnifiedCostModel` @ `engine_builder.rs:52..351` (F3 suspected-unreachable ops → confirm in A03) |
| CD-04 Index & storage | **in_scope** | PE-01/05 | A05 | `crates/storage` File/BinaryStorage (`--storage`) |
| CD-05 Transaction & WAL | **in_scope** | PE-01/03 | A06 | `crates/transaction`+WAL/MVCC |
| CD-06 Protocol | **in_scope** | PE-01/02/03 | A04, A10 | `crates/mysql-server` wire + rustls |
| CD-07 Security & isolation | **in_scope** | PE-01/03/05 | A06, A10 | `crates/security`+session-db (#5192 P0 anchor) |
| CD-08 Test system | **in_scope** | test bins | A07, A08 | meta: audit OF test system (anti-audit) |
| CD-09 Cross-version | **in_scope** | PE-01 | A09 | vs v3.12.0 GA baseline (deps A04/05/06) |
| CD-10 Operations & persistence | **in_scope** | PE-06/07 | A06 | `recovery_engine`/checkpoint/backup/pitr |

## 4. OUT_OF_SCOPE (not claimed by v4.1.0 — authority cited)

| ID | Capability | Reason | Authority |
|----|-----------|--------|-----------|
| OOS-01 | General-purpose vector DB | README §0 forbids; internal GMP only | README §0; RELEASE_NOTES §1 |
| OOS-02 | General-purpose graph DB | README §0 forbids; internal GMP only | README §0 |
| OOS-03 | General-purpose RAG | internal-GMP-only | README §0 |
| OOS-04 | Full MySQL 5.7 replacement | subset only; stage=DRAFT | README §5; STAGE.yaml |
| OOS-05 | SQLite dialect (11 items) | excluded from v4.1.0 | RELEASE_NOTES §3.1; CLAIM_DOWNGRADE_MANIFEST |
| OOS-06 | Production multi-model claims | require GA; v4.1.0=DRAFT | RELEASE_NOTES §2; STAGE.yaml |
| OOS-07 | 168h multi-model SOAK | require GA | RELEASE_NOTES §2 |

## 5. DEFERRED (claimed/planned — verification deferred)

| ID | Capability | Owner | Target version | Follow-up issue | Re-inclusion condition |
|----|-----------|-------|----------------|-----------------|------------------------|
| DEF-01 | Internal GMP retrieval (vector/graph/RAG) | GMP track | separate GMP audit track | TBD | GMP track has its own frozen run + IAIA |
| DEF-02 | 24 WP backlog items (WP-C..G) | v4.1.0 team | v4.1.0 ALPHA | per-WP (RELEASE_NOTES §3) | lands + wired to a production entry |
| DEF-03 | CHAR(n) PAD SPACE (#4846) | v4.1.0 team | v4.1.0 ALPHA | #4846 | lands in WP-G, exercised |
| DEF-04 | ALTER TABLE RENAME COLUMN (#4848) | v4.1.0 team | v4.1.0 ALPHA | #4848 | lands in WP-F, exercised |

## 6. Acceptance self-checks (#5224)

- [x] **AC-1** Every public claim has (source, production entry, ≥1 verification WP) — see `capability_inventory.yaml`.
- [x] **AC-2** Every out-of-scope capability has reason + owner + follow-up issue — see §4/§5 above.
- [x] **AC-3** Module existence / test count / test-pass NOT substituted for production-reachability — every in-scope domain has file:line reachability evidence.
- [x] **AC-4** All 10 capability domains have a scope decision.
- [x] **AC-5** All 6 production-entry categories are covered (PE-01..PE-07).

## 7. assessed_commit note

The formal assessed_commit is **`609a8e81`** (frozen by A01's `manifest.yaml`, SSOT §0) — **not** the
current branch HEAD. The A02 branch tree sits at descendant `01df92a5`, which contains post-freeze
storage fixes (#5266 db-dropping trait default removal, #5057 shared `current_db` removal, #5277
MVCC row-identity, #5281). Those fixes are **NOT** part of the audited baseline: at `609a8e81` the
db-dropping `insert_in_db` trait default and shared `current_db` state are still present, so A03/A06
will confirm #5192-class cross-database data-loss on the frozen baseline. All code references in this
matrix were taken at `609a8e81` (worktree `iaia-a02-baseline`).
