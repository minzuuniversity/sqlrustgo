# scripts/gate/ scripts 状态索引 (audit 2026-06-04)

> 自动审计结果: 47 个脚本, 20 个真正 dead (0 调用 + 0 文档引用)。
> 本文件替代 DELETION — 通过 DEPRECATED header 保留 git history。

## 🟢 Active (被 ci.yml 或 D9 orchestrator 调用)

| Script | Caller | Status |
|--------|--------|--------|
| `check_alpha_v380.sh` | ci.yml | ✅ A1-A5 + A6-1~5 |
| `check_arch_sem_debt.sh` | D9 | ✅ D8 |
| `check_coverage.sh` | ci.yml | ✅ Coverage |
| `check_cross_version_debt.sh` | D9 | ✅ Cross-version |
| `check_evidence_binding.sh` | ci.yml | ✅ Evidence |
| `check_full_gate_verification.sh` | ci.yml | ✅ D9 orchestrator |
| `check_int_debt.sh` | D9 | ✅ D7 |
| `check_rc_ga_gate.sh` | ci.yml + D9 | ✅ D1-D5, D6a |
| `check_test_inventory.sh` | D9 | ✅ D6b |
| `audit_testing.sh` | ci.yml | ✅ D5.5 |
| `auto_env_blocker.sh` | ci.yml | ✅ SPEC-021 |

## 🟡 Internal (只被其他 gate 调)

| Script | Caller | Notes |
|--------|--------|-------|
| `audit_development.sh` | (内部) | audit framework, 未被调 |
| `audit_documentation.sh` | (内部) | docs/governance/ANTI_FABRICATION_POLICY.md 引用 |
| `audit_process.sh` | (内部) | self-reference (audit_*.sh required list) |
| `check_alpha.sh` | (内部) | 旧版 alpha, 已被 check_alpha_v380.sh 替代 |
| `check_arch_invariants.sh` | (内部) | C-ARCH-05 SSOT 引用源 |
| `check_attack_surface.sh` | (内部) | G-04 检查 |
| `check_beta_e2e.sh` | (内部) | Beta stage e2e wrapper |
| `check_beta_gate.sh` | (内部) | Beta stage gate |
| `check_docs.sh` | (内部) | 旧版 docs, 已被 check_docs_links.sh 替代 |
| `check_docs_consistency.sh` | (内部) | 文档一致性 |
| `check_docs_links.sh` | (内部) | AGENTS.md 引用 |
| `check_g_02_source_marker.sh` | (内部) | G-02 source marker |
| `check_g_06_freshness.sh` | (内部) | G-06 freshness |
| `check_integration_gate.sh` | (内部) | B-5 integration |
| `check_mainline.sh` | (内部) | mainline protection |
| `check_perf baseline.sh` | (内部) | perf baseline (注意空格) |
| `check_proof.sh` | (内部) | proof registry |
| `check_security.sh` | (内部) | security scan |
| `check_sql_compat.sh` | (内部) | SQL-92 compat |
| `check_validation_chain.sh` | (内部) | G-02 chain |
| `run_alpha_chain_test.sh` | (内部) | alpha chain runner |

## 🔴 Truly Dead (0 调用 + 0 文档引用)

| Script | 状态 |
|--------|------|
| `send_gate_alert.sh` | 🗑️ DEPRECATED — webhook 集成未启用 |
| `send_gate_failure_webhook.sh` | 🗑️ DEPRECATED — webhook 集成未启用 |
| `update_proof_registry.sh` | 🗑️ DEPRECATED — 已被 check_proof.sh 替代 |
| `pre-commit-env-blocker.sh` | 🗑️ DEPRECATED — pre-commit hook 已禁用 |
| `gate_webhook.sh` | 🗑️ DEPRECATED — webhook 集成未启用 |
| `log_gate.sh` | 🗑️ DEPRECATED — 日志改用 artifacts/gate/ |
| `collect_self_opt_metrics.sh` | 🗑️ DEPRECATED — 自我优化 phase 已结束 |
| `check_5_principles.sh` | 🗑️ DEPRECATED — 文档引用但未调用 (spec drift) |
| `check_10_principles.sh` | 🗑️ DEPRECATED — 同上 |
| `check_docs.sh` | 🗑️ DEPRECATED — 旧版 |
| `check_execution_boundary.sh` | 🗑️ DEPRECATED — 旧 EE 边界检查 |
| `check_g_02_source_marker.sh` | 🗑️ DEPRECATED — spec drift |
| `check_g_06_freshness.sh` | 🗑️ DEPRECATED — spec drift |
| `check_performance.sh` | 🗑️ DEPRECATED — 旧 perf |
| `check_plan_integrity.sh` | 🗑️ DEPRECATED — spec drift |
| `check_r1_r10_content.sh` | 🗑️ DEPRECATED — R-Gate 已退役 |
| `verify_beta_entry.sh` | 🗑️ DEPRECATED — Beta 已 GA |

> 上述 17 个脚本保留在仓库 (git history 完整), 但加 DEPRECATED header 防止新代码误用。
> 恢复方法: `git log --all --oneline -- scripts/gate/<name>.sh`
