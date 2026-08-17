# SQLRustGo v3.12 Teaching Corpus Structural Report

> **provenance:** generated_by=check_teaching_corpus_v312.sh, generated_at=2026-08-17T18:26:18+08:00, commit=acac41762cc0d00e2e3b571ef91c28345ded3783, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, evidence_hash=4a87a986e453b9851bad253f497be074e0b16094e048a4eacbdbeca69f12f5c3, log=/Users/liying/workspace/dev/yinglichina163/sqlrustgo/docs/releases/v3.12.0/logs/teaching_corpus_acac41762c_20260817_182617.log

| Field | Value |
|---|---|
| source_agent | local |
| source_run | check_teaching_corpus_v312 |
| timestamp | 2026-08-17T18:26:18+08:00 |
| commit | acac41762cc0d00e2e3b571ef91c28345ded3783 |
| log | /Users/liying/workspace/dev/yinglichina163/sqlrustgo/docs/releases/v3.12.0/logs/teaching_corpus_acac41762c_20260817_182617.log |
| evidence_hash | 4a87a986e453b9851bad253f497be074e0b16094e048a4eacbdbeca69f12f5c3 |
| gate_status | FAIL |
| pass | 14 |
| fail | 1 |
| warn | 1 |
| with_oracle | 1 |
| oracle_runs | 20 |
| oracle_pass | 0 |
| oracle_fail | 1 |
| oracle_skipped | 8 |
| oracle_diff_dir | /Users/liying/workspace/dev/yinglichina163/sqlrustgo/docs/releases/v3.12.0/evidence/teaching_corpus/oracle_diffs |

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
to `cargo test --test teaching_corpus_oracle_test` (per-file row-set
parity vs SQLite goldens under `docs/releases/v3.12.0/evidence/teaching_corpus/golden/`).
The oracle test runs only when this gate is invoked with
`--with-oracle`; pass/fail counts are then folded into the JSON
summary and oracle diffs land in `docs/releases/v3.12.0/evidence/teaching_corpus/oracle_diffs/`.

## Boundary

- This report covers the teaching corpus only. The smoke corpus
  (`crates/sqlrustgo_sqllogictest/testdata`) and the SQLite official
  corpus are tracked under `check_sqllogictest_v312.sh` / V312-11 / V312-24.
