# V400-09 168h Multi-Model SOAK — FINAL REPORT (Deferral Note)

> **Date**: 2026-09-29 (this report)
> **Status**: 🟡 **DEFERRED** — kickoff issued 2026-09-19, but the
>  168h SOAK process never ran to completion and no per-run
>  monitoring log / STABILITY_REPORT_<TS>.md exists.
> **Original kickoff**: `docs/releases/v4.0.0/SOAK_168H_KICKOFF_2026-09-19.md`
> **Closure trigger**: GA gate verdict (per V400-09 in
>  `CLAIM_DOWNGRADE_MANIFEST.md §3.2 GA-claim downgrade boundaries`).
> **Architectural owner**: openclaw / CI / Z6G4

## 1. Executive Summary

The V400-09 168h multi-model SOAK was kicked off on 2026-09-19
(kickoff PIDs: bash wrapper 41637, sqlrustgo-mysql-server 41654,
driver ALIVE per the kickoff report §1). However:

- **No `STABILITY_REPORT_<TS>.md` was produced** at the 168h completion
  timestamp (target: 2026-09-26 22:11:53Z + 168h).
- **No per-run `metrics.csv` / `driver.log` / `server.log` artifacts exist**
  in `docs/releases/v4.0.0/SOAK_*` matching the kickoff configuration
  (port 3411, WAL_SYNC=batch:100, dataset sbtest1 10k rows).
- **No Z6G4 runner telemetry** was found in the GitHub Actions or
  Gitea CI runners' known artifact stores.

In short, the kickoff report was authored and the SOAK process
**claimed to launch**, but the 168h sustained run did not occur.

This is **consistent with the earlier 8h SOAK finding** (per
`docs/releases/v4.0.0/SOAK_BASELINE_8H_2026-09-17.md`):

> "**Conclusion**: The 8h SOAK confirms the 1h SOAK finding: the v4.0.0
> perf stack (MVCC GC + WAL group commit) is healthy and stable, but
> the working-set peak exceeds the macOS per-process RSS cap. This is
> a **deployment-tuning follow-up**, not a code defect.
>
> **V400-09 (168h multi-model SOAK) is not yet ready to start.** The
> follow-up to lower the working-set peak is a precondition."

The kickoff on 2026-09-19 was therefore issued against the same RSS
ceiling risk, without the deployment-tuning follow-up landing first.

## 2. Status of the v4.0.0 self-claimed GA in light of this deferral

The V400-09 168h SOAK was a **RC-to-GA gate requirement** per
`docs/releases/v4.0.0/STAGE.yaml#required_gactions_for_ga`:

> - "168h multi-model SOAK PASS"

The v4.0.0 `GA_GATE_REPORT.md` (2026-09-20) recorded this as a
"CONDITIONAL PASS" with `CLAIM_DOWNGRADE_MANIFEST.md §3.2` carving
out:

> "v4.0.0 SOAK: 5-min baseline PASS, 168h deferred to v4.0.1"

So the deferral is **already** an officially-claimed boundary in the
v4.0.0 GA release. This report closes out the open question of
"what happened to the 168h run" with a definitive answer: it never
ran to completion, and the deferral was correctly captured in
CLAIM_DOWNGRADE_MANIFEST.md at GA time.

## 3. 168h SOAK prerequisites that were not met at kickoff

Per the 8h SOAK follow-up list (`SOAK_BASELINE_8H_2026-09-17.md §5`):

1. **Lower working-set peak** before kicking off V400-09 (168h SOAK):
   - RSS grows to >1.5 GB before GC fires on macOS arm64
   - macOS jetsam cap (≈1.7 GB) was hit at 3112s in the 8h run
   - **Status as of 2026-09-29**: tuning follow-up not landed
     (no commit since 2026-09-19 changed `gc_lag`, `BufferPool`
     sizing, or related parameters)
2. **Instrument `task_info(TASK_VM_INFO).phys_size`** at server startup
   - **Status**: not landed
3. **Verify the 1.7 GB cap** is per-process or per-host
   - **Status**: not landed
4. **V400-09 follow-up items** (RSS budget per crate, automatic
   working-set reduction, per-macOS-machine tuning)
   - **Status**: none landed

All 4 prerequisites were OPEN at kickoff time. The 168h SOAK was
issued as a **best-effort kickoff** to give the CI/Z6G4 runner an
opportunity to run, but the prereqs were not in place to expect
completion.

## 4. Decision: DEFER V400-09 to v4.1.0 scope

Given:

- The 168h SOAK was never observably running past the kickoff timestamp
- The 4 prerequisites from 8h SOAK are still open
- v4.0.0 GA already carved out the 168h SOAK as a v4.0.1 boundary
- v4.1.0 is the active continuation branch with WP-C..G deferred items

**The correct decision is**: V400-09 168h SOAK is **scoped to v4.1.0**
rather than v4.0.0. This aligns with the WP-H triage outcome recorded
in `CLAIM_DOWNGRADE_MANIFEST.md §2`:

> WP-H | v3.13/defer #4717 #4707 #4701 #4692 #4699 #4688 #4671 #4639 |
>         7/8 in-scope DONE; #4639 defer-to-v4.1

The 168h SOAK is effectively the same category of deferral: it's
infrastructure-tuning + capacity-validation work that depends on RSS
budget work, which itself depends on the working-set-peak tuning
that has not landed.

## 5. v4.1.0 scope (action items)

For v4.1.0 to close V400-09, the following must happen:

1. **Land working-set-peak tuning** (8h SOAK follow-up item #1):
   - Reduce `BufferPool` default size by 50%
   - Or shorten `gc_lag` from 256 to 64
   - Verify RSS peak < 1.0 GB under 1h SOAK sustained load
2. **Instrument `task_info(TASK_VM_INFO).phys_size`** at server startup
3. **Verify the 1.7 GB cap** is per-process (run
   `sysctl vm.jetsam_threshold` on the target hardware)
4. **Re-launch 168h SOAK** with the new RSS budget on Z6G4 runner
5. **Write `V400_09_168H_SOAK_FINAL_REPORT_v4.1.0.md`** when complete
   (modeled on `docs/releases/v3.9.0/SOAK_168H_MACMINI_REPORT.md`)

These action items are tracked in `docs/releases/v4.1.0/PHASE_1_SCOPE.md`
§2.3.

## 6. References

- `docs/releases/v4.0.0/SOAK_168H_KICKOFF_2026-09-19.md` — kickoff report
- `docs/releases/v4.0.0/SOAK_BASELINE_8H_2026-09-17.md` — 8h SOAK
  (crashed at 3112s with RSS ceiling; V400-09 NOT YET READY verdict)
- `docs/releases/v4.0.0/SOAK_BASELINE_5MIN_2026-09-19.md` — 5min pre-flight PASS
- `docs/releases/v4.0.0/SOAK_BASELINE_1H_2026-09-16.md` — 1h SOAK
- `docs/releases/v4.0.0/PHASE_B_WAL_BATCH.md` — WAL group commit trade-off
- `docs/releases/v4.0.0/PHASE_B_MVCC_GC.md` — MVCC GC tuning
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` §3.2 — v4.0.0 GA
  5-min PASS, 168h deferred
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` — CONDITIONAL PASS
- `docs/releases/v3.9.0/SOAK_168H_MACMINI_REPORT.md` — template for
  PASS-class 168h reports
- `docs/releases/v4.1.0/STAGE.yaml` §open_issues / §next_action — V400-09
  carry-forward
- `docs/releases/v4.1.0/PHASE_1_SCOPE.md` §2.3 — P1 V400-09 action items