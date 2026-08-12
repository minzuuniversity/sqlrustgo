# #3905 SF=10 / Sysbench / Observability baseline close-out

## Issue body (recap)
- #3905 V312-18: SF=10 + Sysbench + Observability baseline
- #4018 V312-18a: TPC-H SF=10 baseline
- #4019 V312-18b: Sysbench OLTP baseline metrics
- #4020 V312-18c: Bulk-load SF=10 benchmark
- #4021 V312-18d: Prometheus /metrics endpoint

## Current infrastructure (no code changes in this PR)
- benches/tpch_bench.rs — TPC-H benchmark harness with scale-factor
  switching and per-query timing summary
- benches/qps_bench.rs — Sysbench-like mixed OLTP benchmark
- benches/dataset_generator.rs — dataset generation utility
- benches/postgres_config.rs — Postgres baseline config
- benches/bench_*.rs — additional micro-benchmarks

## Why this is a status-only PR
The single-turn budget cannot cover the SF=10 data generation,
Prometheus endpoint design + implementation, or Sysbench
baseline write-up. This PR records the infrastructure inventory.

## Source / agent
- source_agent: sisyphus
- source_run: v313-3905-benchmark-closeout / Issue #3905
- timestamp: 2026-08-11

## Status of sub-issues
- #3905: parent, status update only
- #4018: TPC-H SF=10 baseline — pending data generation
- #4019: Sysbench baseline — pending run + write-up
- #4020: Bulk-load SF=10 — pending data generation
- #4021: Prometheus endpoint — pending crate + integration
