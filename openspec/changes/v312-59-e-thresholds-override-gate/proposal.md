## Why

`docs/releases/v3.12.0/STAGE.yaml` 第 118-132 行列出 12 个 thresholds_override 字段
(其中 13 项 boolean=true 必选门禁)，但目前 `scripts/gate/check_beta_v3.12.0.sh` 仅强制了 3 项：

| 字段 | 现状 gate | 行动 |
|---|---|---|
| COVERAGE_MIN_PER_CRATE=80 | check_coverage_v312.sh | 已 PASS（每 crate 独立 ≥80%） |
|  SQLLOGICTEST_SMOKE_REQUIRED | B6_SQLLOGICTEST_SMOKE_GATE | 已 PASS |
|  GMP_AUDIT_TAMPER_TEST_REQUIRED | 隐式 B6_AUDIT_HASH_CHAIN | 显式化为独立项 |
|  GMP_RETRIEVAL_CITATION_REQUIRED | 隐式 B6_HYBRID_RETRIEVAL | 显式化为独立项 |
|  TPCH_SF1_CORRECTNESS_REQUIRED | 路径存在 B6_TPCH_SF1_G4 | 显式 row-count 22/22 assertion |
|  MYSQL_WIRE_E2E_REQUIRED | **无 gate** | 新增 wire protocol 全绿检查 |
|  LOAD_DATA_BULK_IMPORT_REQUIRED | 隐含 B6_V312_56D | 新增 LOAD DATA row_count 校验 |
|  CRASH_RECOVERY_REQUIRED | **无 gate** | 新增 recovery test 全绿检查 |
|  UPGRADE_DOWNGRADE_REQUIRED | **无 gate** | 新增 upgrade roundtrip 全绿检查 |
|  BUSTUBX_EDU_SQLITE_CLI_REQUIRED | 隐含 #4359 | 显式化为独立项 |
|  SQLLOGICTEST_SELECTED_TARGETS_REQUIRED | 依赖 RC-5 | 关联 RC-5 显式校验 |
|  GMP_CORPUS_UNCLASSIFIED_FAILURE_MAX=0 | **无 gate** | 新增 |
|  MIXED_SOAK_HOURS=168 | 依赖 GA-2 | 关联 GA-2 显式校验 |

**风险**：如果 gate 与 STAGE.yaml 字段不同步，GA promotion 时可能出现「声明满足但实际未跑」的反模式（Anti-Fabrication-Policy-v1.0 第 5 条明令禁止）。

## What Changes

- **新增** `scripts/gate/check_v312_gate_thresholds.sh`：13 项 boolean/数值字段 + 8 项实际跑测试的全绿检查
- **新增** `scripts/gate/check_v312_stage_yaml_sync.sh`：STAGE.yaml `thresholds_override` 字段 ↔ gate 脚本实际检查项的一致性校验
- **修改** `scripts/gate/check_beta_v3.12.0.sh`：新增 `B8_THRESHOLDS_OVERRIDE` 检查，调用上述两个脚本
- **修改** `docs/releases/v3.12.0/STAGE.yaml`：在 `promotion_to_RC_requires` 和 `promotion_to_GA_requires` 中显式引用 B8_THRESHOLDS_OVERRIDE
- **新增** `docs/releases/v3.12.0/evidence/v312-59-e/EVIDENCE.md`：记录每个 boolean 字段的实际 gate 输出

## Anti-patterns（明令禁止）

- ❌ 把 boolean=true 改为 boolean=false 逃避 gate
- ❌ 用 `expiry 2027-06-30` 推回任何 8 项布尔门禁
- ❌ "覆盖率 L1_8 85.86% ≥ 80 已满足" 论据 — 必须每个 crate 单独 ≥80%
- ❌ SOAK 时长改为 24h 然后说 "168 太长"
- ❌ "TPCH_SF1_CORRECTNESS 不要求 22/22" 论据 — STAGE.yaml `promotion_to_GA_requires` 第 5 项明确 "no unexplained zero-row/checksum mismatch"

## Provenance

- discovered_during: v312-beta-remediation-2026-08-20
- generated_by: claude-code v3.12.0
- source_run: v3.12.0-thresholds-override
- evidence_root: `docs/releases/v3.12.0/STAGE.yaml` lines 118-132
- branch: `develop/v3.12.0` @ 8c952a7b5
- cross_ref: #4383 V312-59 (grand-parent), #4384 V312-59-B (Beta WARN), #4385 V312-59-C (RC), #4386 V312-59-D (GA), #3887 V312-MASTER, #4388 (this issue)
- policy: Anti-Fabrication-Policy-v1.0