# 无执行点门禁的逐个分类（61 道）—— 处置提案

- 日期：2026-11-08
- 对应看板行：`P0-4`（`docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl`）
- 口径：`gov_probe.py metrics` 的 `dead_list`（`check_n=168`，其中 61 道**没有任何执行点**）

## 0. 先纠正一个说法：这 61 道不是「无人引用」

现场核查：**61/61 都有文档引用**（`docs/`、`audit/`、`scripts/`、CI 配置里都能搜到门禁名）。
⇒ 所以「死门禁」的准确含义是：**有文档引用、但没有任何执行点**（不在 `STAGE_CONFIG` 的阶段清单里，
也没有 CI job 跑它）。这不是「没人记得它」，而是**文档在引用它的结论，却没有任何东西复现那些结论**。
⇒ 处置的第一原则因此改变：**不能直接删**（删了文档里的引用就悬空），
要么恢复执行点，要么把文档里的引用改成「历史记录 + 已退役」。

## 1. 分类结果（逐个依据：同族新门禁存在 / 标题指向旧阶段 / 主题仍在）

### 1.1 有同族新版可取代：**10 道**

| 门禁 | 标题 | 同族新版 |
|---|---|---|
| `check_4021_prometheus_metrics.sh` | check_4021_prometheus_metrics.sh — V312-26 Issue #4021 Prometheus /metrics Gate | `check_prometheus_metrics.sh` |
| `check_5_principles_v310.sh` | check_5_principles_v310.sh — Truthfulness Framework G-01~G-06 for v3.10.0 | `check_principles.sh`, `check_10_principles_v310.sh` |
| `check_alpha_v3.10.0.sh` | v3.10.0 Alpha Gate — 进入 Alpha 阶段必须通过 | `check_alpha_v3.11.0.sh`, `check_alpha_v3.12.0.sh`, `check_alpha_v410.sh`, `check_alpha_v380.sh` |
| `check_arch2_no_bypass.sh` | check_arch2_no_bypass.sh — v3.8.0 ARCH-2 Bypass Gate | `check_arch3_no_bypass.sh` |
| `check_beta_v3.10.0.sh` | check_beta_v3.10.0.sh -- v3.10.0 BETA Governance Gate (B6-B8) | `check_beta_v3.12.0.sh`, `check_beta_v3.11.0.sh` |
| `check_cli_binary_names.py` | """Static analyzer behind `scripts/gate/check_cli_binary_names.sh` — #5405. | `check_cli_binary_names.sh` |
| `check_coverage.sh` | DEPRECATED 2026-08-10: use check_coverage_v312.sh (G17) | `check_coverage_v312.sh` |
| `check_historical_backlog_disposition.sh` | check_historical_backlog_disposition.sh — wrapper for V312-20 / #3907 | `check_historical_backlog_disposition.py` |
| `check_principles.sh` | check_principles.sh — Consolidated 5 + 10 Principles Gate (Phase 4) | `check_5_principles_v310.sh`, `check_10_principles_v310.sh` |
| `check_tpch_sf10.sh` | check_tpch_sf10.sh — GA-P1 TPC-H SF=10 Gate | `check_tpch_sf1.sh` |

### 1.2 旧阶段（v3.x / V312-xx）门禁：**28 道**

| 门禁 | 标题 |
|---|---|
| `check_4019_sysbench_baseline.sh` | check_4019_sysbench_baseline.sh — V312-26 Issue #4019 Sysbench OLTP Baseline Gate |
| `check_4020_bulk_load_sf10.sh` | check_4020_bulk_load_sf10.sh — V312-26 Issue #4020 Bulk-load SF=10 Gate |
| `check_4022_slow_query_log.sh` | check_4022_slow_query_log.sh — V312-26 Issue #4022 Slow Query Log Gate |
| `check_beta_e2e.sh` | check_beta_e2e.sh — BETA Gate E2E Functional Coverage (SPEC-023) |
| `check_bustubx_b_track_v312.sh` | RC-B1 - V312-RC-GA: BustubX-EDU B-track oracle corpus gate |
| `check_bustubx_edu_cli_smoke_v312.sh` | check_bustubx_edu_cli_smoke_v312.sh -- V312-57 BustubX-EDU sqlite-style |
| `check_g11_qps.sh` | check_g11_qps.sh - G11 QPS/TPS 门禁 |
| `check_g12_sysbench.sh` | check_g12_sysbench.sh - G12 Sysbench OLTP 门禁 |
| `check_g15_perf_report.sh` | check_g15_perf_report.sh - G15 Performance Report 门禁 |
| `check_g16_ddl_syntax.sh` | check_g16_ddl_syntax.sh - G16 DDL Syntax Gate (v3.10.0 PR1) |
| `check_g1_tpch_22_22.sh` | check_g1_tpch_22_22.sh - G1 (v3.9.0) 22/22 TPC-H 保持 gate |
| `check_g_correctness_v390.sh` | check_g_correctness_v390.sh - Gate-C (v3.9.0) Substantive Correctness Gate |
| `check_ga_only_v312.sh` | check_ga_only_v312.sh — v3.12.0 GA-1..GA-8 standalone verifier (V312-59-D) |
| `check_plan_integrity_v310.sh` | check_plan_integrity_v310.sh — v3.10.0 Plan Document Integrity Check |
| `check_pr_dag.sh` | check_pr_dag.sh — alias for verify_pr_dag.sh (issue #5457) |
| `check_rc_gate_v3.10.0.sh` | check_rc_gate_v3.10.0.sh — v3.10.0 RC Stage Gate |
| `check_rc_v3.11.0.sh` | check_rc_v3.11.0.sh — v3.11.0 RC Stage Gate |
| `check_stage_template.sh` | check_stage_template.sh — Stage Gate Template (Phase 4) |
| `check_storage_index_wal.sh` | check_storage_index_wal.sh — V312-23 / #3910 gate |
| `check_teaching_corpus_v312.sh` | V312-56B / ISSUE #4252 — Teaching SQL corpus structural gate |
| `check_test_inventory.sh` | check_test_inventory.sh — v3.8.0 D6-Test-Integration Gate |
| `check_v312_01_blocker_closed.sh` | check_v312_01_blocker_closed.sh |
| `check_v312_dml_integrity.sh` | RC-B6 - V312-RC-GA: DML/integrity correctness gate |
| `check_v312_join_subquery_semantics.sh` | RC-B5 - V312-RC-GA: Join/subquery correctness gate |
| `check_v312_no_silent_success.sh` | RC-B3 - V312-RC-GA: Semantic no-op guard gate |
| `check_v312_parser_real_scripts.sh` | RC-B2 - V312-RC-GA: Parser real-script gate |
| `check_v312_promotion_to_rc.sh` | check_v312_promotion_to_rc.sh |
| `check_v312_type_function_semantics.sh` | RC-B4 - V312-RC-GA: Type/function conformance gate |

### 1.3 主题仍在、只是没人跑：**23 道**

| 门禁 | 标题 |
|---|---|
| `check_a04_evidence_reproducible.sh` | check_a04_evidence_reproducible.sh |
| `check_attack_surface.sh` | set -e |
| `check_backup_restore_closed_loop.sh` | scripts/gate/check_backup_restore_closed_loop.sh |
| `check_execution_semantics.sh` | scripts/gate/check_execution_semantics.sh |
| `check_g6_set_ops_syntax.sh` | G6 — V310-06 PR2 / Issue #3723 C-2 (INTERSECT / EXCEPT / UNION ORDER BY) |
| `check_gate_script_refs.sh` | check_gate_script_refs.sh — Gate Criterion Script Reference Existence Gate |
| `check_gmp_consumer.sh` | GMP-Platform consumer compile/smoke gate — closes V400-10 / #4873 / #4943. |
| `check_main_freshness.sh` | scripts/gate/check_main_freshness.sh — anti-divergence gate for the `main` branch. |
| `check_mainline.sh` | set -euo pipefail |
| `check_no_log_tbl_json.sh` | V400-00: File Governance Gate |
| `check_oracle_present.sh` | scripts/gate/check_oracle_present.sh |
| `check_orphan_sources.sh` | Orphan source detector for #4939. |
| `check_p32_statistics.sh` | check_p32_statistics.sh - P3-2 (#3181) Statistics (ANALYZE TABLE) G10 gate |
| `check_p33_cost_optimizer.sh` | check_p33_cost_optimizer.sh - P3-3 (#3182) Cost Optimizer G10 gate |
| `check_p34_parallel_executor.sh` | check_p34_parallel_executor.sh - P3-4 (#3183) ParallelExecutor 优化 G10 gate |
| `check_p35_simd.sh` | check_p35_simd.sh - P3-5 (#3184) SIMD 集成 G10 gate |
| `check_prepared_stmt.sh` | check_prepared_stmt.sh - P3-1 (#3180) Prepared Statement Cache gate |
| `check_proof.sh` | set -e |
| `check_release_binary.sh` | check_release_binary.sh — GA-P0 Release Binary Gate |
| `check_sql_compat.sh` | set -e |
| `check_ssot_duplicate.py` | """ |
| `check_test_count_monotonic.sh` | scripts/gate/check_test_count_monotonic.sh |
| `check_validation_chain.sh` | check_validation_chain.sh — G-01 Validation Chain Enforcement |

## 2. 为什么 1.3 这一类最要紧

本轮实测里有两个 1.3 类的门禁**当场跑通**：

- `check_attack_surface.sh` —— 2026-11-08 修掉它内部 2 处 `grep -c … || echo "0"` 的多行计数 bug 后 rc=0；
  它是「攻击面文档覆盖率」门禁（AV1–AV10 + CRITICAL 计数）。
- `check_a04_evidence_reproducible.sh` —— 2026-11-08 为 IL-10 的反向变异实跑（两臂对照），改前判红、归档最小重录后转绿。

⇒ 这类门禁**是能用的**，只是没人跑 ⇒ 它们的结论若被文档引用，就是**无人复现的证据**
（与 A04 归档漂移、claims 假声明同一类问题）。

## 3. 仓里已有的两个处置先例（供属主批量确认时对齐口径）

1. **`scripts/gate/legacy/` 目录**：`check_10_principles.sh`、`check_r1_r10_content.sh` 等历史实现留在那里，
   并在 `scripts/gate/coverage_measurement_allowlist.txt` 里以 role=`legacy` + 理由登记（不计入 168）。
2. **允许表的 role 字段**：`the-implementation` / `reporting` / `report-generator` / `historical` / `legacy`
   ⇒ 「历史门禁」与「仍在但没人跑」在仓里已经有各自的表达方式，不必新造机制。

## 4. 请属主裁定的三件事（我不自行改门禁清单，属 AGENTS 2.1/3.2）

1. **1.1 的 10 道**：退役（移入 `scripts/gate/legacy/` + 以 `legacy` 角色登记），还是恢复执行点？
2. **1.2 的 28 道**：统一标为 `historical`（文档引用改成历史记录），还是逐个恢复？
3. **1.3 的 23 道**：挂到既有 job（不新增 `check_*`、分母 168 不动）还是退役？其中已证明能跑的至少包括 `check_attack_surface.sh` 与 `check_a04_evidence_reproducible.sh`。

> 每道门禁的处置都需要一次明确决定（reason 或退役），我不能替属主批量确认 —— 这正是本行验收第 ② 条的要求。
