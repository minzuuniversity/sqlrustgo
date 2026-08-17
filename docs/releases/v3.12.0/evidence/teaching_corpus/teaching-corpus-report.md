# SQLRustGo v3.12 Teaching Corpus Structural Report

> **provenance:** generated_by=check_teaching_corpus_v312.sh, generated_at=2026-08-17T11:46:47+08:00, commit=1e0f5018ad4649a8d3aa26e8c3db45dcfbea85cb, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, evidence_hash=c3e3b85ad14c7160fc2411260ae23b9ea440dce8c27bb32672f47a1e64a38356, log=/Users/liying/workspace/dev/yinglichina163/sqlrustgo/docs/releases/v3.12.0/logs/teaching_corpus_1e0f5018ad_20260817_114647.log

| Field | Value |
|---|---|
| source_agent | local |
| source_run | check_teaching_corpus_v312 |
| timestamp | 2026-08-17T11:46:48+08:00 |
| commit | 1e0f5018ad4649a8d3aa26e8c3db45dcfbea85cb |
| log | /Users/liying/workspace/dev/yinglichina163/sqlrustgo/docs/releases/v3.12.0/logs/teaching_corpus_1e0f5018ad_20260817_114647.log |
| evidence_hash | c3e3b85ad14c7160fc2411260ae23b9ea440dce8c27bb32672f47a1e64a38356 |
| gate_status | PASS |
| pass | 13 |
| fail | 0 |
| warn | 1 |

## Scope

This is the V312-56B structural gate. It enforces the manifest contract:

1. Every file in `/Users/liying/workspace/dev/yinglichina163/sqlrustgo/tests/compat/teaching_sql_v3_12` is listed in `manifest.yml` (and vice-versa)
2. Every `.sql` file declares `# name:` and `# expect:` markers
3. FAIL/SKIP entries carry `issue_link` / `owner` / `expiry` in
   `manifest.yml`
4. The oracles block declares at least SQLite (MySQL/PostgreSQL optional)
5. No silent `# ignore` markers

The actual SQLite-vs-SQLRustGo row-by-row oracle comparison is delegated
to `cargo test --test teaching_corpus_test` (manifest integrity) and
to the Round-21 / #4218 follow-up (row-set parity).

## Boundary

- This report covers the teaching corpus only. The smoke corpus
  (`crates/sqlrustgo_sqllogictest/testdata`) and the SQLite official
  corpus are tracked under `check_sqllogictest_v312.sh` / V312-11 / V312-24.
