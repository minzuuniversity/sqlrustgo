# #3943 R2.4 SEM-4 coverage measurement gap close-out evidence

## State documented at close-out (no code changes)
- SEM-4 (Coverage measurement gap: Z6G4 82% vs Z440 32%) tracked as
  `IN_PROGRESS` in `docs/governance/debt/debt-registry.yaml`
  with `progress: 30%` and the existing progress_metric describing
  v3.10.0 average ~67% and the deferred V310-10 ≥80% target.
- This PR captures the close-out evidence for the V312-19-followup
  follow-up issue. It does NOT mark SEM-4 as CLOSED — SEM-4
  remains IN_PROGRESS until the underlying measurement-gap work
  (unified cargo-llvm-cov / tarpaulin runner) is finished.

## Source / agent
- source_agent: sisyphus
- source_run: v313-3943-closeout / Issue #3943
- timestamp: 2026-08-11

## Re-verification
- `grep -B 1 -A 8 'SEM-4' docs/governance/debt/debt-registry.yaml`
  shows the entry still present with the same IN_PROGRESS state and
  30% progress; this is the evidence the issue body describes and
  the V312-19 strict-close condition #4 expectation of
  PASS-WITH-DRIFT is met by the existing debt-registry entry.
