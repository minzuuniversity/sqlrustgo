# CI Gate Contract — v3.12.0 GA Aggregator

> **Version**: 1.0
> **Date**: 2026-08-27
> **Issue**: #4536 (fast-path GA gate)
> **Owner**: maintainers

This document captures the **invocation contract** between CI workflows
and the v3.12.0 GA aggregator gate script
(`scripts/gate/check_ga_v3.12.0.sh`). It exists because Issue #4536
introduced a `--fast-path` / `--full` mode split, and CI parity must be
preserved while local-dev ergonomics improve.

## Status as of 2026-08-27

| Workflow (`.gitea/workflows/*.yml`) | Calls aggregator? | Passes `--fast-path`? |
|------------------------------------|-------------------|------------------------|
| `ci.yml`                           | No                | n/a                    |
| `ci-test.md`                       | No                | n/a                    |
| `gate.yml`                         | No                | n/a                    |
| `reconciliation.yml`               | No                | n/a                    |
| `soak_168h.yml`                    | No                | n/a                    |
| `soak_probe.yml`                   | No                | n/a                    |

**Result**: aggregator is invoked **manually by maintainers** during GA
promotion reviews, not by automated CI. This is intentional — the
aggregator is a **governance summary tool**, not a CI gate. Per-stage
verification (B1 build/clippy/fmt, B2 lib tests, GA-2 168h soak, etc.)
is enforced by separate dedicated workflows/scripts that produce their
own `evidence_hash` and feed the aggregator's report.

## Contract

If the aggregator is **ever** invoked from CI in the future, the
following contract applies:

### MUST NOT pass `--fast-path`

- `--fast-path` is **local-dev only**. It performs a `bash -n` syntax
  check on `check_beta_v3.12.0.sh` and skips the heavy
  `cargo build + clippy + fmt` cascade. CI parity requires the heavy
  BETA stage to actually execute.
- A CI invocation that **incorrectly** passes `--fast-path` will be
  flagged by the JSON `mode` field (see below) and rejected by reviewers.

### MUST use default invocation (no flag) or `--full`

- Default `MODE="full"` (see `check_ga_v3.12.0.sh` line 64) preserves
  backward compatibility.
- Explicit `--full` is equivalent to default and equally CI-safe.

### JSON report `mode` field MUST be `"full"` for CI

- The JSON top-level `mode` field is `"full"` for heavy runs and
  `"fast-path"` for local-dev runs (see `check_ga_v3.12.0.sh`
  line 464).
- Any CI-generated `ga_gate_report.json` with `"mode": "fast-path"`
  is a **misconfiguration** and must be rejected before promotion.

## Cross-references

- Script: [`scripts/gate/check_ga_v3.12.0.sh`](../../scripts/gate/check_ga_v3.12.0.sh)
- Proposal: `openspec/changes/issue-4536-fast-path-ga-gate/proposal.md`
- Design: `openspec/changes/issue-4536-fast-path-ga-gate/design.md`
- Spec: `openspec/changes/issue-4536-fast-path-ga-gate/specs/ga-aggregator/spec.md`
- Evidence: `docs/releases/v3.12.0/evidence/v312-59/issue-4536-fast-path-bench.txt`
- Evidence: `docs/releases/v3.12.0/evidence/v312-59/issue-4536-fast-path-report.json`
- Evidence: `docs/releases/v3.12.0/evidence/v312-59/issue-4536-default-behavior.txt`
- Issue: #4536

## Provenance (ADR-014 5 evidence fields)

| 字段 | 值 |
|------|---|
| source_agent | claude-sonnet (Claude Code) |
| source_run | issue-4536-fast-path-ga-gate-20260827 |
| timestamp | 2026-08-27T19:15:00+08:00 |
| evidence_hash | local-git:`14f638d09` (post-merge of PR #4524 docs-unify) |
| conflict_resolution | N/A — single AI scope |