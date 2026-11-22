# P0-4 处置调研：83 道「无执行点」门禁的登记、分类与实跑史

**日期**：2026-11-17　**性质**：**只读聚合**，产出处置**输入**，**不代替裁决**。

## 0. 一句话结论

- 门禁分母 **168**；其中 **stage_required 34 + ci_reachable 51 = 85** 有执行点，**83** 道没有（`dead 61` + `orphan_referenced 22`）。
- **在 6 个归档轮次、共 172 条实跑落账里，只出现过 31 个门禁名** ⇒ **137/168 个门禁在归档证据里从未被实跑过**（83 道无执行点的**全部**在内）。
- 「83 道从未实跑」是**已验证事实**，不是推断：自检过工具本身对**已知跑过**的门禁能取到历史（`check_rc_ga_gate.sh` 7 次、`check_g1_tpch_baseline.sh` 7 次且末次 = FAIL）。
- **处置方向**（外部专家已定，见 `expert-review-20261117.md` 第 4 条）：**红门禁应阻断 GA，而不是通过断开执行入口来维持绿色** ⇒ 先让门禁能跑 → 再修红 → 最后挂执行点。

## 1. 分布

| 维度 | 分布 |
|---|---|
| 类别 | `{'ci_reachable': 51, 'dead': 61, 'orphan_referenced': 22, 'stage_required': 34}` |
| 处置 | `{'needs-external-host': 12, 'archive-candidate': 32, 'needs-execution-point': 25, 'superseded': 2, 'keep-rewire': 12}` |

## 2. 83 道逐条明细

| 门禁 | 类别 | 分类文档 | 登记处置 | 实跑史 | 落点动作 |
|---|---|---|---|---|---|
| `scripts/gate/check_5_principles_v310.sh` | dead | 1.1 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_alpha_v3.10.0.sh` | dead | 1.1 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_alpha_v3.11.0.sh` | orphan_referenced | — | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_arch2_no_bypass.sh` | dead | 1.1 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_beta_v3.10.0.sh` | dead | 1.1 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_beta_v3.11.0.sh` | orphan_referenced | — | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_bustubx_edu_cli_smoke_v312.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_g12_sysbench.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_g15_perf_report.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_g16_ddl_syntax.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_g1_tpch_22_22.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_g6_set_ops_syntax.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_g_correctness_v390.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_ga_v3.12.0.sh` | orphan_referenced | — | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_ga_wire_recovery_upgrade.sh` | orphan_referenced | — | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_gate_script_refs.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_historical_backlog_disposition.py` | orphan_referenced | — | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_historical_backlog_disposition.sh` | dead | 1.1 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_p32_statistics.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_p33_cost_optimizer.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_p34_parallel_executor.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_p35_simd.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_perf_baseline.sh` | orphan_referenced | — | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_plan_integrity_v310.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_pr_dag.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_prepared_stmt.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_rc_v3.11.0.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_sql_compat.sh` | dead | 1.3 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_stage_template.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_test_inventory.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_tpch_sf1.sh` | orphan_referenced | — | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_v312_01_blocker_closed.sh` | dead | 1.2 | archive-candidate | **从未实跑** | 退役候选（保留+自标头，分母不动） |
| `scripts/gate/check_docs_consistency_v312.sh` | orphan_referenced | — | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_docs_links_v312.sh` | orphan_referenced | — | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_ga_only_v312.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_sqllogictest_selected_v312.sh` | orphan_referenced | — | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_storage_index_wal.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_teaching_corpus_v312.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_v312_dml_integrity.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_v312_join_subquery_semantics.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_v312_no_silent_success.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_v312_parser_real_scripts.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_v312_promotion_to_rc.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_v312_type_function_semantics.sh` | dead | 1.2 | keep-rewire | **从未实跑** | 改接线（既有 job 内换路径，不新增 check_*） |
| `scripts/gate/check_a04_evidence_reproducible.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_admin_enhance.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_alter_add_after.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_architecture.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_attack_surface.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_audit_manifest.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_backup_restore_closed_loop.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_beta_e2e.sh` | dead | 1.2 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_cli_binary_names.py` | dead | 1.1 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_execution_semantics.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_main_freshness.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_mainline.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_no_log_tbl_json.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_orphan_sources.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_pr_template_sections.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_principles.sh` | dead | 1.1 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_proof.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_regression_ci.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_release_binary.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_security.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_ssot_duplicate.py` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_test_count_monotonic.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_upgrade_guide.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_upgrade_v310_v311.sh` | orphan_referenced | — | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_validation_chain.sh` | dead | 1.3 | needs-execution-point | **从未实跑** | 挂执行点（**先跑一次**确认红绿） |
| `scripts/gate/check_4019_sysbench_baseline.sh` | dead | 1.2 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_4020_bulk_load_sf10.sh` | dead | 1.2 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_4021_prometheus_metrics.sh` | dead | 1.1 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_4022_slow_query_log.sh` | dead | 1.2 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_bustubx_b_track_v312.sh` | dead | 1.2 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_g11_qps.sh` | dead | 1.2 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_ga2_soak_artifact.sh` | orphan_referenced | — | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_gmp_consumer.sh` | dead | 1.3 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_oracle_present.sh` | dead | 1.3 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_prometheus_metrics.sh` | orphan_referenced | — | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_sysbench_oltp.sh` | orphan_referenced | — | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_tpch_sf10.sh` | dead | 1.1 | needs-external-host | **从未实跑** | 待外部宿主（本机跑不了 ⇒ 不伪装已验） |
| `scripts/gate/check_coverage.sh` | dead | 1.1 | superseded | **从未实跑** | 已被取代（保留+自标头，不再挂） |
| `scripts/gate/check_rc_gate_v3.10.0.sh` | dead | 1.2 | superseded | **从未实跑** | 已被取代（保留+自标头，不再挂） |

## 3. 按登记处置分组（执行清单）

### `archive-candidate` —— 32 道

**落点动作**：退役候选：**保留文件 + 文件头自标 + 登记改 disposition**（既有先例 `check_rc_gate_v3.10.0.sh` 的 SUPERSEDED 头） ⇒ **分母 168 不动**；**不删文件、不移 `legacy/`**

- `scripts/gate/check_5_principles_v310.sh`
- `scripts/gate/check_alpha_v3.10.0.sh`
- `scripts/gate/check_alpha_v3.11.0.sh`
- `scripts/gate/check_arch2_no_bypass.sh`
- `scripts/gate/check_beta_v3.10.0.sh`
- `scripts/gate/check_beta_v3.11.0.sh`
- `scripts/gate/check_bustubx_edu_cli_smoke_v312.sh`
- `scripts/gate/check_g12_sysbench.sh`
- `scripts/gate/check_g15_perf_report.sh`
- `scripts/gate/check_g16_ddl_syntax.sh`
- `scripts/gate/check_g1_tpch_22_22.sh`
- `scripts/gate/check_g6_set_ops_syntax.sh`
- `scripts/gate/check_g_correctness_v390.sh`
- `scripts/gate/check_ga_v3.12.0.sh`
- `scripts/gate/check_ga_wire_recovery_upgrade.sh`
- `scripts/gate/check_gate_script_refs.sh`
- `scripts/gate/check_historical_backlog_disposition.py`
- `scripts/gate/check_historical_backlog_disposition.sh`
- `scripts/gate/check_p32_statistics.sh`
- `scripts/gate/check_p33_cost_optimizer.sh`
- `scripts/gate/check_p34_parallel_executor.sh`
- `scripts/gate/check_p35_simd.sh`
- `scripts/gate/check_perf_baseline.sh`
- `scripts/gate/check_plan_integrity_v310.sh`
- `scripts/gate/check_pr_dag.sh`
- `scripts/gate/check_prepared_stmt.sh`
- `scripts/gate/check_rc_v3.11.0.sh`
- `scripts/gate/check_sql_compat.sh`
- `scripts/gate/check_stage_template.sh`
- `scripts/gate/check_test_inventory.sh`
- `scripts/gate/check_tpch_sf1.sh`
- `scripts/gate/check_v312_01_blocker_closed.sh`

### `keep-rewire` —— 12 道

**落点动作**：改接线（既有 job 内换脚本路径）⇒ **不新增 `check_*`**、分母不动

- `scripts/gate/check_docs_consistency_v312.sh`
- `scripts/gate/check_docs_links_v312.sh`
- `scripts/gate/check_ga_only_v312.sh`
- `scripts/gate/check_sqllogictest_selected_v312.sh`
- `scripts/gate/check_storage_index_wal.sh`
- `scripts/gate/check_teaching_corpus_v312.sh`
- `scripts/gate/check_v312_dml_integrity.sh`
- `scripts/gate/check_v312_join_subquery_semantics.sh`
- `scripts/gate/check_v312_no_silent_success.sh`
- `scripts/gate/check_v312_parser_real_scripts.sh`
- `scripts/gate/check_v312_promotion_to_rc.sh`
- `scripts/gate/check_v312_type_function_semantics.sh`

### `needs-execution-point` —— 25 道

**落点动作**：挂既有执行点（`STAGE_CONFIG` 的 `stages.<S>.required_gates` 或 `.gitea/workflows/*.yml` 加 step）⇒ **先跑一次**确认红绿

- `scripts/gate/check_a04_evidence_reproducible.sh`
- `scripts/gate/check_admin_enhance.sh`
- `scripts/gate/check_alter_add_after.sh`
- `scripts/gate/check_architecture.sh`
- `scripts/gate/check_attack_surface.sh`
- `scripts/gate/check_audit_manifest.sh`
- `scripts/gate/check_backup_restore_closed_loop.sh`
- `scripts/gate/check_beta_e2e.sh`
- `scripts/gate/check_cli_binary_names.py`
- `scripts/gate/check_execution_semantics.sh`
- `scripts/gate/check_main_freshness.sh`
- `scripts/gate/check_mainline.sh`
- `scripts/gate/check_no_log_tbl_json.sh`
- `scripts/gate/check_orphan_sources.sh`
- `scripts/gate/check_pr_template_sections.sh`
- `scripts/gate/check_principles.sh`
- `scripts/gate/check_proof.sh`
- `scripts/gate/check_regression_ci.sh`
- `scripts/gate/check_release_binary.sh`
- `scripts/gate/check_security.sh`
- `scripts/gate/check_ssot_duplicate.py`
- `scripts/gate/check_test_count_monotonic.sh`
- `scripts/gate/check_upgrade_guide.sh`
- `scripts/gate/check_upgrade_v310_v311.sh`
- `scripts/gate/check_validation_chain.sh`

### `needs-external-host` —— 12 道

**落点动作**：需要外部宿主/凭据 ⇒ 本机跑不了 ⇒ 登记为「待外部宿主」，**不伪装成本机已验**

- `scripts/gate/check_4019_sysbench_baseline.sh`
- `scripts/gate/check_4020_bulk_load_sf10.sh`
- `scripts/gate/check_4021_prometheus_metrics.sh`
- `scripts/gate/check_4022_slow_query_log.sh`
- `scripts/gate/check_bustubx_b_track_v312.sh`
- `scripts/gate/check_g11_qps.sh`
- `scripts/gate/check_ga2_soak_artifact.sh`
- `scripts/gate/check_gmp_consumer.sh`
- `scripts/gate/check_oracle_present.sh`
- `scripts/gate/check_prometheus_metrics.sh`
- `scripts/gate/check_sysbench_oltp.sh`
- `scripts/gate/check_tpch_sf10.sh`

### `superseded` —— 2 道

**落点动作**：已被取代：同 archive-candidate 的做法；**不得**再挂执行点（C7 会红）

- `scripts/gate/check_coverage.sh`
- `scripts/gate/check_rc_gate_v3.10.0.sh`

## 4. 落点的两个真实选项（分母都不动）

| 落点 | 改哪 | class 变成 | 约束 |
|---|---|---|---|
| 阶段执行点 | `docs/governance/STAGE_CONFIG.yaml` 的 `stages.<S>.required_gates`（5 个阶段块） | `stage_required` | 受 `check_stage_config_consistency.sh` 的 C1/C2/C7 约束（G 别名脚本必须出现在 `stage_min` 起每一阶段清单） |
| CI 执行点 | `.gitea/workflows/*.yml` 加一个 run step | `ci_reachable` | 注意 `.github/workflows` **不算**执行点（`gate_executability.py:59` 只认 `.gitea/workflows`） |

**共同约束**：

- 挂上执行点 ⇒ class 变 ⇒ **必须重新 emit 登记**（否则 C2 现场重算对不上 ⇒ 红）。
- 挂上执行点的门禁**若仍标 `archive-candidate`/`superseded` ⇒ 直接红**（`gate_registry.py` 的 C7）⇒ 恢复执行点时必须**同步清掉旧 disposition**。
- **分母 168 不动**：不删脚本、**不移进 `scripts/gate/legacy/`**（分母口径是**非递归** glob `scripts/gate/check_*.sh|py`；`legacy/` 下已有的 10 个**不计入** 168 ⇒ 移进去会立刻掉到 167 ⇒ C1 破）。

## 5. 明确不做

- 不删任何门禁脚本、不移 `legacy/`、不新增 `check_*`、不改门禁分母。
- 不用「不挂执行点」把红藏起来（专家第 4 条）。
- 不把分类文档 §4 的三问改成「批量代确认」—— 分类文档自己写明「每道门禁的处置都需要一次明确决定」。
- 不改门禁脚本的**判据**；只允许改「接线/登记/自标头」这三类。

## 6. 一处必须公开的口径张力

看板 P0-1 的验收写「孤儿数 83→0」，而「分母 168 固定」。`gov_probe.py` 的 `orphan` 定义是「既不在 `required_gates`（listed 34），也不在 CI 可达集（reach）」⇒ **唯一的减法是给执行点**，而删文件会同时减分母 ⇒ 在分母固定下，「83→0」**只能**靠**给这 83 道全部恢复执行点**达成。

⇒ 建议把 P0-1 的验收口径改成「**按 class 逐类清零并登记处置**」，否则该行在分母固定前提下**永远不可闭合**。这是口径问题，不是执行问题。

## 7. 证据来源

- `docs/releases/v4.1.0/GATE_CATALOG.jsonl`（登记 SSOT，168 行）
- `docs/releases/v4.1.0/DEAD_GATES_CLASSIFICATION_2026-11-08.md`（61 道死门禁三类分档）
- `audit/v4.1.0/runs/**/results.jsonl`（归档实跑落账）
- `scripts/gate/gov_probe.py`（`orphan`/`dead` 口径）、`scripts/gate/gate_registry.py`（C1–C7 与 emit/check）、`scripts/gate/gate_executability.py`（执行点解析）
- `scripts/gate/check_audit_manifest.sh` 的 10 条失败另有专篇：`audit/v4.1.0/adjudications/audit-manifest-10-failures-20261117.md`

**本轮只做只读调研**：未执行任何门禁、未改任何文件、未挂任何执行点。
