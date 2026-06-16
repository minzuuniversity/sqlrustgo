## Why

TPC-H Q16 (Parts/Supplier Relationship Query) was reported in Sprint 5
v2 as `engine_issue` with 0 rows returned, while PostgreSQL returns
284 rows. The hypothesis at the time was a `NOT IN (subquery)`
implementation bug — the conservative evaluator might be dropping all
rows instead of doing a real subquery check.

Sprint 6 data regen (l_linenumber PK unique + discount 11-value
uniform + tax 9-value uniform) regenerated the SF=0.1 fixture to
60K lineitem rows. After the regen, an isolated in-process test
(`tests/_q15_q16_check.rs`) shows sqlrustgo now returns **284 rows
in ~72ms** for Q16, byte-exact with PostgreSQL on the same fixture.

The Sprint 5 v2 bug classification was therefore incorrect: Q16 was
a *fixture-shape* problem, not an engine bug. The `NOT IN` subquery
evaluator in `src/engine_utils.rs` was always correct — it just
happened to run against a fixture where the inner subquery's
`s_comment LIKE '%bad%deals%'` filter matched enough suppliers to
exclude everything (or matched nothing to exclude nothing, depending
on the discount rounding).

This change **verifies** Q16 against PG and updates the audit report
to mark the engine issue closed.

## What Changes

- **No engine code change** — the existing NOT IN subquery evaluator
  is correct.
- **Verification** (`tests/_q15_q16_check.rs`): single-test fixture
  showing sqlrustgo Q16 returns 284 rows matching PG.
- **Audit report** (`docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`):
  update the Q16 row from "engine_issue" to "PASS (post-Sprint 6 regen)".
- **Sibling change** `2026-06-08-v390-sprint6-q15-subquery` covers the
  same finding for Q15 (subquery in FROM).

## Impact

- Affected: GA-BLOCKER #3291 cell-level count (Q16: engine_issue → PASS).
- Combined with the Q15 sibling change, the Sprint 5 v2 pass count
  moves from 16/22 to **18/22**.
- Operators / users: no API change. The TPC-H harness is unchanged.

## Acceptance Criteria

- [ ] `tests/_q15_q16_check.rs::test_q16_not_in_subquery` returns 284 rows
      matching PG (byte-exact on all 4 columns: p_brand, p_type, p_size, supplier_cnt).
- [ ] `cargo test --release --test _q15_q16_check -- --nocapture` exits 0.
- [ ] Sprint 7 audit report Q16 row marked PASS.
- [ ] No regression: Q1-Q15, Q17, Q19 cell-level results unchanged.
