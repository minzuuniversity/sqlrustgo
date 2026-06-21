# v3.9.0 72h Soak Test Report (REVISED 2026-06-21)

> **2026-06-21 audit correction**: This report was originally written
> for the in-process 1,440x compressed-time smoke. That form is no
> longer the G7 gate. The current G7 Soak Test gate is a wired E2E
> run of `sqlrustgo-mysql-server` driven by `sysbench oltp_read_write`.
> See `docs/openspec/3175-soak-test.md` and
> `scripts/gate/check_p13_soak_test.sh`.
>
> The original report content below is preserved for historical
> reference; it documents the harness invariants that were verified
> at beta cut, **not** a production-realistic 72h run.

## Executive Summary (REVISED)

As of 2026-06-21, v3.9.0 G7 Soak Test gate:

- **Mode**: WIRED E2E (real `sqlrustgo-mysql-server` + `sysbench oltp_read_write`)
- **CI default**: 5 minutes wall-clock (override via `SOAK_MINUTES`)
- **Long-run forms**: 24h / 72h / 168h via `scripts/stability/run_wired_soak.sh`
- **In-process simulation**: decommissioned; `tests/soak_test.rs` all `#[ignore]`
- **Latest gate run**: see `test_results/g7_soak_*/STABILITY_REPORT.md` (per CI run)

The historical 1,440x compression (60s/180s/420s) is retained as a
harness-level invariant suite for ad-hoc local debugging only:
`cargo test --test soak_test -- --ignored`.

---

## Original Report (2026-06-05, HISTORICAL)

> **Author**: Hermes C (Auto-generated)
> **Date**: 2026-06-05
> **Branch**: `develop/v3.9.0` @ `c71b609f`
> **Tag candidate**: `v3.9.0-beta`
> **Gate (at the time)**: G7 24h Soak, per ALPHA_GATE_CONTRACT.md §1.7
> **Mode (at the time)**: in-process compressed-time smoke, NOT production-realistic

### Historical Executive Summary (PRESERVED FOR REFERENCE)

v3.9.0 72h soak test **PASS** in compressed-time smoke (180s wall-clock,
1,440x compression). All 10 in-process soak tests passed at all 3 levels
(24h/72h/168h). G7 gate (the in-process form): **7/7 PASS**.

### Historical Test Results (SAMPLE)

| Test | Result | Notes |
|------|--------|-------|
| test_soak_72h_smoke_p1_3 | PASS | 180s, 900 queries, no alert |
| test_soak_72h_smoke_no_fd_leak_p1_3 | PASS | FD count stable |
| test_soak_24h_smoke_p1_3 | PASS | 60s, 300 queries |
| test_soak_24h_smoke_memory_growth_within_threshold_p1_3 | PASS | < 10% |
| test_soak_24h_smoke_p99_latency_bounded_p1_3 | PASS | < 5ms |
| test_soak_168h_smoke_p1_3 | PASS | 420s, 2100 queries |
| test_soak_168h_smoke_no_lock_leak_proxy_p1_3 | PASS | FD growth <= 5 |
| test_soak_alert_message_when_exceeds_threshold_p1_3 | PASS | harness invariant |
| test_soak_memory_baseline_invariant_p1_3 | PASS | no-query == baseline |
| test_soak_p50_p99_ordering_p1_3 | PASS | p99 >= p50 |

**Limitation acknowledged**: The above was an in-process simulation. It
validated harness invariants but did NOT exercise the wire protocol,
buffer pool, connection manager, WAL, or lock manager. Real 24h/72h/168h
wall-clock was deferred to RC phase. See the REVISED section above for
the current wired-E2E gate.
