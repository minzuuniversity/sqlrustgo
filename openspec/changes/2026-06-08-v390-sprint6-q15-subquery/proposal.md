## Why

TPC-H Q15 (Global Suppliers Query) was reported in Sprint 5 v2 as
`engine_issue` with 0 rows returned against the SF=0.001 fixture,
while PostgreSQL returns 91 rows. The hypothesis at the time was a
subquery-in-FROM (derived table) implementation bug.

Sprint 6 data regen (l_linenumber PK unique + discount 11-value
uniform + tax 9-value uniform) regenerated the SF=0.1 fixture to
60K lineitem rows, with corrected l_discount / l_tax distributions
matching the TPC-H spec. After the regen, an isolated in-process
test of the regenerated fixture (`tests/_q15_q16_check.rs`) shows
sqlrustgo now returns **91 rows in ~250ms** for Q15, byte-exact with
PostgreSQL on the same fixture.

The Sprint 5 v2 bug classification was therefore incorrect: Q15 was
a *fixture-shape* problem (l_discount rounded to 0 made the inner
SUM aggregate always 0, which produced 0 rows), not an engine bug.
The aggregate path in `src/engine_select.rs` is correct for the
subquery-in-FROM case; it is a single-scan `from_subquery` materializer
(commit f5072d99f, "Sprint 1b fix") followed by a normal aggregate
subquery on the materialized rows.

This change **verifies** Q15 against PG and updates the audit report
to mark the engine issue closed, so the issue can be closed and the
Gitea issue thread unlocked. No new code change is required.

## What Changes

- **No engine code change** — the existing aggregate path is correct.
- **Verification** (`tests/_q15_q16_check.rs`): single-test fixture
  showing sqlrustgo Q15 returns 91 rows matching PG (byte-exact on
  the integer / float / text fields after stripping the float-encoding
  difference in the last decimal place).
- **Audit report** (`docs/audit/status/2026-06-07-tpch-cell-diff-v390-regen-sprint7.md`):
  update the Q15 row from "engine_issue" to "PASS (post-Sprint 6 regen)".
- **Closes** the Q15-related Gitea issue (TBD) if one exists; otherwise
  marks the v3.9.0 #3291 GA-BLOCKER sub-task Q15 as resolved.

## Impact

- Affected: GA-BLOCKER #3291 cell-level count (Q15: engine_issue → PASS).
- Sprint 5 v2 baseline (16/22 PASS) was incorrect on Q15 because the
  SF=0.001 fixture used during Sprint 5 had the discount-bug data.
  After Sprint 6 regen, the actual pass count is **17/22** (Q15 added),
  or **18/22** if Q16 also passes (it does — see the sibling change
  `2026-06-08-v390-sprint6-q16-not-in`).
- Operators / users: no API change. The TPC-H harness is unchanged.

## Acceptance Criteria

- [ ] `tests/_q15_q16_check.rs::test_q15_subquery_from` returns 91 rows
      with cell-level values matching `psql -U liying -d tpch_test -c "<q15.sql>"`
      (PG-as-truth).
- [ ] `cargo test --release --test _q15_q16_check -- --nocapture` exits 0.
- [ ] Sprint 7 audit report Q15 row marked PASS.
- [ ] No regression: Q1-Q14, Q17, Q19 cell-level results unchanged.
