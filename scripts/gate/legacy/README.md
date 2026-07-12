# scripts/gate/legacy/

This directory contains **deprecated gate scripts** that are no longer
maintained or invoked by STAGE_CONFIG.yaml. They are preserved for:

1. **Historical reference** — understanding why specific decisions were made
2. **Audit trail** — documenting the gate framework evolution
3. **Bug postmortem** — providing context for completed issue investigations

## Why they're in legacy/ (not deleted)

Deleting these scripts would break historical PRs and postmortem documents
that reference them by name. Per the DeepSeek review feedback
(local://attachment-1) and the "anti-fabrication" gate
(`check_anti_fabrication.sh`), deletion without explicit reason risks
silent regressions.

## Each legacy script header declares its own deprecation

All scripts in this directory have the marker comment:

```bash
# STATUS: DEPRECATED — see scripts/gate/README.md
# Reason: ...
# Action: do not add new callers; restore via git history if needed
```

The 4 attributes (Status / Reason / Action / original purpose) are present
in every script header so a reader can understand intent without external
docs.

## Current legacy scripts (as of 2026-07-13)

| Script | Origin | Reason for archive |
|--------|--------|---------------------|
| `check_alpha.sh` | v2.9.0 (early 2026) | Hardcoded v2.9.0 paths; superseded by `check_alpha_v3.10.0.sh` |
| `check_5_principles.sh` | v3.0.0 | Replaced by `check_principles.sh` (consolidated 5+10) |
| `check_10_principles.sh` | v3.0.0 | Replaced by `check_principles.sh` (consolidated 5+10) |
| `check_docs.sh` | v2.9.0 | Hardcoded v2.9.0; replaced by `check_docs_links.sh` |
| `check_execution_boundary.sh` | v2.9.0 | Pre-EXPLAIN ANALYZE; not needed for v3.10.0 |
| `check_g_02_source_marker.sh` | v2.9.0 | G-02 obsolete per v3.10.0 evidence model |
| `check_g_06_freshness.sh` | v2.9.0 | G-06 obsolete per v3.10.0 evidence model |
| `check_performance.sh` | v2.9.0 | Hardcoded v2.9.0; replaced by `check_g15_perf_report.sh` |
| `check_plan_integrity.sh` | v2.9.0 | Pre-Stage Control Framework; replaced by `check_stage.sh` |
| `check_r1_r10_content.sh` | v2.9.0 | Pre-5-principles consolidation; replaced by `check_principles.sh` |
| `collect_self_opt_metrics.sh` | v2.9.0 | Not in active gate flow |
| `gate_webhook.sh` | v2.9.0 | Superseded by `send_gate_alert.sh` (still in legacy) |
| `log_gate.sh` | v2.9.0 | Logging via `gmp_progress.log`; not in active gate flow |
| `pre-commit-env-blocker.sh` | v2.9.0 | Hardcoded v2.9.0 env names; not in active gate flow |
| `send_gate_alert.sh` | v2.9.0 | Not in active alert flow; pending replacement by `gate_webhook.sh` (or v3.11+) |
| `send_gate_failure_webhook.sh` | v2.9.0 | Hardcoded HTTP endpoint; pending review |
| `update_proof_registry.sh` | v2.9.0 | Pre-evidence-binding model; replaced by `check_evidence_binding.sh` |
| `verify_beta_entry.sh` | v2.9.0 | Pre-Stage Control Framework; replaced by `check_stage.sh` |

## Reference

- Per-version audit: see `docs/releases/v3.10.0/V310_TEST_BINARY_GATE_AUDIT_REPORT.md`
- DeepSeek review: `local://attachment-1` §阶段 4
- Phase 4 commit: see `d7a56702e6` (refactor(v3.10.0): Phase 1b — add tools/* to workspace members)
- Phase 4 commit: see `3e4cd4accc` (refactor(v3.10.0): Phase 2 — migrate 224 root tests/ files to subdirs)

---

*Last updated: 2026-07-13 by Claude Code (hermes-agent) per DeepSeek review*
