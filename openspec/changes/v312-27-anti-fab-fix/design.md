# V312-27 Anti-Fab Fix — Design (Retroactive)

**Status**: RECONSTRUCTED 2026-08-10

## Anti-Fabrication Principle

Every `#[ignore]` must link to a real issue or a phased removal plan.
Bare `#[ignore]` without rationale is an anti-fabrication violation.

## Registry Drift Detection

Every `#[ignore]` in source must appear in `tests/baseline/ignore_registry.json`
as either ACTIVE, MARKER, or RETIRED. The CI workflow detects drift by
grep-diff at PR time.

## Exit Criterion

The 5 boundary conditions in `V312-27_anti_fab_fix_report.md` must all PASS:
1. VtuGuard test passes (replaces `#[ignore]` with active assertion)
2. tpch_wire_smoke rows.len() > 0 assertion passes
3. E2E tests run (early-return blocks removed)
4. ignore_registry.json has 37 fewer stale entries
5. Closure report exists and is referenced
