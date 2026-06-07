# Tasks: v390 Sprint 3 Operator Suite Merge

> **Change**: v390-sprint3-operator-suite-merge
> **PR**: #3298 (feat/v390-operator-regression-suite → develop/v3.9.0)
> **Issues**: #3277 (Multi-Join 0-row bug, auto-closed on merge), #3298 (PR tracking, manual close)
> **Effort**: ~20 min (validation + merge + verify)

## 1. Setup

- [ ] 1.1 Pull PR #3298 head branch via tarball (smart-HTTP intermittent broken)
- [ ] 1.2 Verify 8 changed files exist with expected content
- [ ] 1.3 Confirm `mergeable=True` on PR #3298

## 2. Validate

- [ ] 2.1 Inspect `src/engine_select.rs` (+22 -1): `lookup_qualified_column` helper present, called from `find_join_key_index` with qualifier prefix
- [ ] 2.2 Inspect `tests/operators/aggregate.rs` (+136): 9 tests, 2 corrected (sum_real_column, sum_empty_table_null)
- [ ] 2.3 Inspect `tests/operators/join.rs` (+413): 20 tests, includes 3-table chain, LEFT OUTER, NULL keys, 4-table
- [ ] 2.4 Inspect `tests/operators/exists.rs` (+78): 4 EXISTS tests (Sprint 4 protection)
- [ ] 2.5 Inspect `docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md` (+227): 5 root cause categories
- [ ] 2.6 Inspect `crates/bench/examples/tpch_data_gen.rs` (+8 -2): discount/tax fix
- [ ] 2.7 Inspect `Cargo.toml` (+12): 3 new `[[test]]` entries
- [ ] 2.8 Inspect `docs/releases/v3.9.0/incidents/GITEA_252_OUTAGE_20260607.md` (+36): incident record

## 3. Pre-merge checks

- [ ] 3.1 Confirm no other open PR conflicts with develop/v3.9.0
- [ ] 3.2 Confirm PR has `Closes #3277` in body (auto-close)
- [ ] 3.3 Confirm `force_merge=true` is acceptable (Gitea branch state)

## 4. Merge

- [ ] 4.1 Call `POST /pulls/3298/merge` with `{"do":"merge","force_merge:true"}`
- [ ] 4.2 Handle rate limit: sleep 8-30s, retry if "Please try again later"
- [ ] 4.3 Verify merge succeeded: `GET /pulls/3298` → `state=closed, merged=true`

## 5. Post-merge verification

- [ ] 5.1 Verify #3277 auto-closed: `GET /issues/3277` → `state=closed`
- [ ] 5.2 If #3298 not auto-closed, PATCH `state=closed` (Pitfall 47)
- [ ] 5.3 Spot-check develop/v3.9.0 head has the new commit: `git ls-remote origin develop/v3.9.0`

## 6. Archive

- [ ] 6.1 Run `openspec archive v390-sprint3-operator-suite-merge -y`
- [ ] 6.2 Verify archive moved to `openspec/changes/archive/2026-06-07-v390-sprint3-operator-suite-merge/`
- [ ] 6.3 Verify specs merged into `openspec/specs/{operator-regression-suite,multi-join-3-table-resolution,tpch-root-cause-board}/spec.md`
