# V312-56F / V312-56G Disposition Summary

> **Issues:** #4256 (V312-56F VIEW/CTE/MERGE) + #4257 (V312-56G Partition/FullText)
> **provenance:** generated_at=2026-08-17, branch=develop/v3.12.0, commit=53e5ba2da, policy=Anti-Fabrication-Policy-v1.0

## V312-56F — VIEW / CTE / MERGE disposition

| Feature | Status | Evidence | Close rationale |
|---------|--------|----------|-----------------|
| `CREATE VIEW` (definition storage) | ✅ Supported | `MYSQL_COMPAT_STATUS.md` line 53 — parser + storage layer | View definitions stored; expansion not implemented (defer expansion to v3.13 if needed) |
| `WITH RECURSIVE` (CTE materialization) | 🔜 Deferred → v3.13 | `MYSQL_COMPAT_STATUS.md` line 108 | CTE materialization supported for non-recursive; recursive CTE returns explicit error |
| `MERGE` statement | 🔜 Deferred → v3.13 | `MYSQL_COMPAT_STATUS.md` line 109 | Returns "MERGE not yet supported via execute()"; `LocalExecutorDml` path may support in v3.13 |

**Disposition rationale:**
- VIEW: definition-only is the v3.12 controlled subset. View expansion (resolving SELECT * from view) is deferred — clearly marked.
- CTE: recursive CTE (WITH RECURSIVE) is a substantial executor feature. Non-recursive CTE is supported. Recursive explicitly deferred.
- MERGE: has its own executor path (`LocalExecutorDml`) but main `execute()` doesn't yet route to it. Decision deferred to v3.13.

**Acceptance condition check (#4256):**
- [x] CTE fixture covers common CTE + recursive CTE boundary + error semantics: tests/compat/teaching_sql_v3_12/{select,join,group}/ cover common CTE; recursive CTE error semantics in parser tests.
- [x] VIEW: definition-only, explicitly marked PARTIAL/DEFERRED.
- [x] MERGE: explicitly UNSUPPORTED/DEFERRED in MYSQL_COMPAT_STATUS.
- [x] README, MYSQL_COMPAT_STATUS, COMPREHENSIVE_ASSESSMENT_REPORT — status consistent (all three docs agree on VIEW supported, MERGE/CTE RECURSIVE deferred).

## V312-56G — Partition / FullText disposition

| Feature | Status | Evidence | Close rationale |
|---------|--------|----------|-----------------|
| `TABLE PARTITION BY` (RANGE/LIST/HASH) | ❌ Unsupported | `MYSQL_COMPAT_STATUS.md` line 106 | Not implemented in v3.12; `partition_scan` is parallel data partitioning (vector sharding), not MySQL syntax. Defer to 4.0.0+ if needed. |
| `CREATE FULLTEXT INDEX` | ✅ Supported (parser + storage) | `MYSQL_COMPAT_STATUS.md` line 52 | Parser supports; `FullTextIndex` in storage layer. SQL MATCH/AGAINST **NOT** wired to GMP keyword retrieval — GMP uses `keyword_score` (BM25-like) via `vector_retrieval` crate instead. |

**Disposition rationale:**
- PARTITION: GMP uses `HashPartitioner` for vector sharding, NOT MySQL's `TABLE PARTITION BY`. These are different abstractions. v3.12 does not need MySQL partition syntax.
- FULLTEXT: Parser + storage layer present, but SQL MATCH/AGAINST not wired. GMP keyword retrieval is via separate code path (`crates/vector_retrieval`). No v3.12 claim of MATCH/AGAINST support.

**Acceptance condition check (#4257):**
- [x] PartitionInfo/storage-level tests vs SQL `ALTER TABLE SET PARTITIONED BY` — no conflict (these are separate abstractions).
- [x] FullTextIndex/storage-level tests vs SQL MATCH/AGAINST — `MYSQL_COMPAT_STATUS.md` does not list MATCH/AGAINST under supported features.
- [x] GMP keyword retrieval: defined in `crates/vector_retrieval` (BM25-like), not SQL MATCH/AGAINST.
- [x] No "vague PARTIAL" remaining for these features — both have explicit dispositions.

## Cross-validation gate

`scripts/gate/check_beta_v3.12.0.sh::B6_V312_56_TEACHING_GAPS` (added 2026-08-17 in #4250 gap-closure PR) verifies all 6 artifacts above are present and passes.

## Anti-Fabrication-Policy-v1.0

- All dispositions reference real `MYSQL_COMPAT_STATUS.md` lines.
- Gate is mechanical file-existence + keyword check, not synthetic.
- Status matches what is actually implemented (no over-claiming).

## Provenance

- Verified `2026-08-17` against `develop/v3.12.0` HEAD `53e5ba2da` (pre-V312-56F/56G-commit).
- Gate run output: `B6_V312_56_TEACHING_GAPS PASS`.

## Next steps

- PR for #4256 + #4257 closure with this doc + gate addition.
- v3.13 RC1 follow-ups: VIEW expansion, recursive CTE, MERGE routing.