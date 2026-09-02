# SQLRustGo v3.12.0 Performance Report

> **provenance:** generated_by=codex-cli, generated_at=2026-09-02T21:20:00+08:00, source_repo=openclaw/sqlrustgo, remote_head=`b14ad8df0306891e217af19648b210e81d5d2be0`, policy=Anti-Fabrication-Policy-v1.0
> **stage:** RC / GA candidate preparation

## Summary

The v3.12.0 performance evidence is strong enough for a GA-candidate review,
but not yet sufficient for final GA promotion because the canonical Linux/Docker
SOAK 5691 re-validation is still pending.

## Evidence Matrix

| Area | Verdict | Evidence |
|---|---|---|
| TPC-H SF=1 correctness/perf | PASS-PREVIOUS | `evidence/v312-59/GA5_TPCH_SF1_REPORT.md`, `evidence/v312-58/Q17_SF1_CELLDIFF.json` |
| Q17 SF=1 | PASS-PREVIOUS | Q17 elapsed 61.6s, row_count=1, FP64 value within tolerance. |
| Mixed SOAK smoke/1h | PASS-PREVIOUS | `evidence/v312-59/GA2_MIXED_SOAK_DEMO_REPORT.md` and `soak/mixed_workload_1h_demo_v2.json`. |
| SOAK V5 local 8h | PASS-LOCAL | `evidence/v312-59/SOAK_V5_FINDINGS.md`, `soak/soak_v5_summary_20260902_195919.txt`. |
| SOAK 168h Linux/Docker | PENDING | Required for final GA-2 closure unless formally reclassified. |
| Wire/recovery/upgrade | PASS-PREVIOUS | `evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md`. |

## TPC-H SF=1

`GA5_TPCH_SF1_REPORT.md` records sqlrustgo as 22/22 PASS after PR #4550.
The previously weak Q17 path was re-verified at approximately 61.6 seconds
with oracle-matching row count and floating-point value within tolerance.

This evidence supports the GA-5 requirement:

> TPC-H SF=1 correctness has no unexplained zero-row/checksum mismatch.

## Mixed SOAK

### 1h Demo

The 1h demo evidence records 5-class mixed workload execution with 0% failure
at the authored time. This remains useful as infrastructure proof, but it is
not a substitute for the final SOAK evidence boundary if the release requires
Linux/Docker long-run validation.

### V5 Local 8h Counterfactual

`SOAK_V5_FINDINGS.md` records the latest local performance result:

| Metric | Value |
|---|---|
| Workload | sysbench `oltp_read_write` |
| Duration | 8 h |
| Threads | 4 |
| Rate | 4 |
| Transactions | 63,092 |
| QPS | 43.81 |
| Ignored errors | 0 |
| Reconnects | 0 |
| RSS range | 188-230 MB across 16 workload snapshots |

The result supports the claim that Fix B + Fix C remove the observed
O(N)-clone-on-DML leak class on the local macOS development binary.

### Remaining SOAK Gap

The same V5 report explicitly limits its scope: Linux dirty-page retention
behavior from SOAK 5691 cannot be reproduced on macOS. A Linux/Docker
re-validation remains the canonical GA-2 evidence gap.

## Release Claim Boundary

Allowed now:

- v3.12.0 is an RC with strong GA-candidate performance evidence.
- TPC-H SF=1 and local SOAK V5 evidence are available and linked.

Not allowed yet:

- v3.12.0 GA performance gate is fully passed.
- 168h mixed SOAK is complete.
- Linux/Docker SOAK 5691 behavior is revalidated.
