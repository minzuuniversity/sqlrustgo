# v3.10.0 Gate Scripts Audit

> **Date**: 2026-07-13
> **Method**: Compare `scripts/gate/*.sh` against `docs/governance/STAGE_CONFIG.yaml`
> **Status**: 49 of 75 scripts are NOT in STAGE_CONFIG (Phase 4 cleanup target)

## Summary

| Category | Count | % |
|----------|------:|--:|
| **Active (referenced in STAGE_CONFIG)** | 26 | 35% |
| **Active (referenced in STAGE_CONFIG via path-only)** | 3 | 4% |
| **UNUSED in STAGE_CONFIG** | 49 | 65% |
| **TOTAL** | 75 | 100% |

## 1. Active Gate Scripts (29 in STAGE_CONFIG)

### Universal (per stage)
1. `check_arch_invariants.sh` — C-ARCH-01~05
2. `check_arch3_no_bypass.sh` — VTU enforcement
3. `check_arch_sem_debt.sh` — D8
4. `check_cross_version_debt.sh` — cross-version debt
5. `check_int_debt.sh` — INT debt
6. `check_anti_fabrication.sh` — anti-fake
7. `check_full_gate_verification.sh` — full gate
8. `check_drift_not_pass.sh` — no drift
9. `check_integration_gate.sh` — integration
10. `check_docs_links.sh` — DRAFT
11. `check_coverage.sh` — coverage (per-stage optional)
12. `check_architecture_freeze.sh` — RC+ only
13. `check_gate_self_verification.sh` — GA
14. `check_gate_test_integrity.sh` — GA

### Per-stage (template references)
15. `check_alpha.sh` (v2.9.0, DEPRECATED, in `legacy/`)
16. `check_alpha_v{VER}.sh` (per-version template, e.g. `check_alpha_v3.10.0.sh`)
17. `check_beta_gate.sh` (version-agnostic, our commit a6784b2)
18. `check_rc_ga_gate.sh` (RC + GA)

### G1-G16 specific (individual gates)
19-29. `check_g1_tpch_22_22.sh`, `check_g1_tpch_baseline.sh`, `check_g11_qps.sh`, `check_g12_sysbench.sh`, `check_g13_stability.sh`, `check_g14_real_crash.sh`, `check_g15_perf_report.sh`, `check_g16_compatibility.sh`, `check_g1_tpch_baseline.sh`, `check_p13_soak.sh` (per G*), `check_soak_gate.sh` (per G*), `check_backup_restore.sh` (G6)

## 2. UNUSED Gate Scripts (49 — cleanup candidates)

### 2.1 Audit / Process (5) — informational only
- `audit_development.sh`
- `audit_documentation.sh`
- `audit_process.sh`
- `audit_testing.sh`
- `auto_env_blocker.sh`

### 2.2 Per-version / Per-isolated (7) — kept for historical reference
- `check_alpha_v3.10.0.sh` (superseded by check_beta_gate.sh v3.10.0)
- `check_alpha_v380.sh` (v3.8.0 specific)
- `check_beta_e2e.sh` (v3.8.0/v3.9.0 specific)
- `check_beta_v3.10.0.sh` (v3.10.0 per-version, BETA specific)
- `check_g_all.sh` (v3.9.0 RC gate, deprecated)
- `check_g_correctness_v390.sh` (v3.9.0 specific)
- `legacy/check_alpha.sh` (v2.9.0, moved earlier)

### 2.3 Update / Collect / Send / Log / Webhook / Verify / Metric (10) — workflow tools
- `check_p12_crash_test.sh`, `check_p13_soak_test.sh`, `check_p14_upgrade_test.sh`
- `check_p21_audit_log.sh`, `check_p22_time_travel.sh`, `check_p23_hash_chain.sh`
- `update_changelog.sh`, `update_proof_registry.sh`
- `gate_webhook.sh`, `send_gate_alert.sh`, `send_gate_failure_webhook.sh`
- `log_gate.sh`, `collect_self_opt_metrics.sh`
- `verify_beta_entry.sh`, `pre-commit-env-blocker.sh`

### 2.4 Per-feature / Per-issue (10) — issue-specific scripts
- `check_arch2_no_bypass.sh` (ARCH-2 closed in v3.8.0)
- `check_attack_surface.sh` (v3.6.0 era)
- `check_docs_consistency.sh` (in v3.8.0 docs)
- `check_document_completeness.sh` (in v3.8.0 docs)
- `check_evidence_binding.sh` (v3.6.0 era)
- `check_execution_semantics.sh` (v3.6.0 era)
- `check_g16_ddl_syntax.sh` (v3.10.0 WIP, kept for reference)
- `check_g6_set_ops_syntax.sh` (v3.10.0 WIP, kept for reference)
- `check_ignore_count.sh` (v3.9.0 era)
- `check_int2_no_orphan.sh`, `check_int3_single_expr.sh` (INT-2/INT-3 specific)
- `check_mainline.sh` (v3.8.0 era)
- `check_oracle_present.sh` (v3.8.0 era)

### 2.5 Per-version Gates (already covered)
- `check_alpha_v3.10.0.sh` (in 2.2)
- `check_beta_v3.10.0.sh` (in 2.2)
- `check_rc_gate_v3.10.0.sh` (created in Phase 3+4, not yet in STAGE_CONFIG)
- `check_principles.sh` (consolidates 5+10 principles, not in STAGE_CONFIG)

## 3. Recommendations

### 3.1 Keep (high priority)
- 26 active scripts in STAGE_CONFIG
- 3 per-version scripts: `check_alpha_v3.10.0.sh`, `check_beta_v3.10.0.sh`, `check_rc_gate_v3.10.0.sh` (recommended for v3.10.0 RC/GA)
- 5 audit scripts (historical reference)
- 1 principles consolidator: `check_principles.sh`

### 3.2 Move to `legacy/` (low priority, v3.11+ cleanup)
- 6 v3.6.0/v3.7.0/v3.8.0 era scripts: `check_attack_surface`, `check_evidence_binding`, `check_execution_semantics`, `check_mainline`, `check_oracle_present`, `check_ignore_count`
- 4 INT-2/INT-3 specific: `check_int2_no_orphan`, `check_int3_single_expr`, `check_arch2_no_bypass`
- 2 G*-specific v3.10.0 WIP: `check_g16_ddl_syntax`, `check_g6_set_ops_syntax`
- 2 docs checks: `check_docs_consistency`, `check_document_completeness`

### 3.3 Workflow tools (keep at root, not gate)
- 10 update/collect/send/log/webhook/verify/metric scripts
- 5 p12-p23 specific scripts
- 2 v3.9.0 era: `check_g_all.sh`, `check_g_correctness_v390.sh`
- 2 v3.8.0/v3.9.0 era: `check_alpha_v380.sh`, `check_beta_e2e.sh`

### 3.4 Final target
| Metric | Current | Target (v3.11+) |
|--------|--------:|----------------:|
| Total gate scripts | 75 | 30 |
| Active in STAGE_CONFIG | 29 | 30 |
| `legacy/` archive | 1 | 20 |
| Workflow tools (root) | 25 | 25 (separate category) |

## 4. Already Done (Phase 0/3/4)

- `legacy/check_alpha.sh` — already moved (commit `ecb9280`)
- `check_principles.sh` — already created as 5+10 consolidator (commit `785ca6772c`)

## 5. Action Items

| Priority | Action | Effort |
|----------|--------|--------|
| LOW | Move 12 deprecated scripts to `legacy/` | 1h |
| LOW | Add `check_rc_gate_v3.10.0.sh` to STAGE_CONFIG RC section | 30min |
| LOW | Add `check_principles.sh` to STAGE_CONFIG documentation | 30min |
| LOW | Add `check_beta_v3.10.0.sh` to STAGE_CONFIG BETA section | 30min |
| DEFER | 252 恢复后 push 全套 changes to 252 | TBD |

## References

- `docs/governance/STAGE_CONFIG.yaml` (5 阶段 gate definitions SSOT)
- `scripts/gate/` (89 → 75 actual .sh files)
- Phase 0 commit `a6784b2` (check_beta_gate.sh)
- Phase 3+4 commit `e1fd7779` (check_rc_gate_v3.10.0.sh)
- Phase 4 commit `785ca6772c` (check_principles.sh)
- DeepSeek review (local://attachment-1) §阶段 4
