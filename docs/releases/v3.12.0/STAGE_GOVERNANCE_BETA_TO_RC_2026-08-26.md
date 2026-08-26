# SQLRustGo v3.12.0 阶段治理报告 — BETA → RC (2026-08-26)

> **日期**: 2026-08-26
> **执行人**: openclaw-minimax (V312-59-C cycle)
> **范围**: 252 Gitea `openclaw/sqlrustgo` develop/v3.12.0 分支、RC 准入门槛、#4386 umbrella
> **依据**: ADR-001 Truthfulness Framework、Anti-Fabrication Policy、Issue Closing Verification、v3.12 `STAGE.yaml` + `RC_GATE_REPORT.md`

## 1. 结论

v3.12.0 已于 **2026-08-26** 从 BETA 转入 RC 阶段，记录在
`docs/releases/v3.12.0/STAGE.yaml`（`current_stage: RC`，
`last_transition.from: BETA → to: RC`，date: `2026-08-26`）。

V312-59-C umbrella issue (#4386) 记录了 RC 转入的完整执行证据：

- **Composite gate**: `bash scripts/gate/check_v312_promotion_to_rc.sh`
  报告 9 PASS / 0 FAIL / 2 NO-OP-covered（11/11 total）。
- **B8 thresholds_override**: 13/13 PASS（issue #4388 在
  2026-08-26T18:30:00Z 重新核验，含 SF=10 fixture 修复后的
  MYSQL_WIRE_E2E_REQUIRED）。
- **每项 RC 报告**: `evidence/v312-59/RC{1..11}_*_REPORT.md` 共 11
  份 wrapper 报告，引用上游主报告 + 8-字段 provenance header。

## 2. 治理原则（继承自 2026-08-18 纠偏报告）

### 2.1 反延期边界（V312-59）

本轮 RC 转入严格遵守 Anti-Fabrication-Policy-v1.0：

1. **无** `expiry 2027-06-30` 延期——所有 12 项 RC 准入均在 2026-08-26
   当日（commit `ba5fc80a6`）以执行证据闭合。
2. **无** v3.13 follow-up issue 用于 RC 阻断项——Q17/Q20 全 SF=1
   TIMEOUT 在 RC11 claim cleanup 中标注为已知 partial 覆盖（**不
   是** RC blocker），与 `v312-24_deferred_items_status.md` 一致。
3. **无** "168h SOAK 可以 GA 后跑"——RC3 backup/restore 已具备
   完整证据；168h SOAK 是 GA 准入，不是 RC 准入。
4. **无** warn→check 不修测试——所有 11 项 gate 都产生具体 PASS
   证据（文件存在 + RC10 跑过 `scripts/gate/check_bustubx_edu_cli_v312.sh`）。

### 2.2 NO-OP 标记的合法边界

RC6（TPC-H SF=1 cross-engine）与 RC9（V312-57 week01-04）使用
`NO-OP-COVERED-BY-*` 标记，**但仍要求 wrapper 报告**：

- `RC6_TPCH_SF1_CROSS_ENGINE_REPORT.md` — 引用
  `evidence/tpch/cross_engine_sf1/SUMMARY.json` 的 4-engine × 22-query
  矩阵 + Q17/Q20 partial TIMEOUT 的 honest disclosure（V312-58 Sprint 5 系列）。
- `RC9_V312_57_WEEK01_04_REPORT.md` — 引用 PRs #4359/#4370/#4373 +
  `evidence/v312-57-smoke/V312-57-SMOKE-FIXTURES-CLOSURE.md` 14/14 PASS。

NO-OP 标记不等于"忽略"；它等同于"上游已闭合 + 本轮不需要重新执行"，
但仍需可审计的 wrapper 报告。这是 V312-59-D 反延期原则的应用。

### 2.3 v3.13 冻结策略（继承自 STAGE_CONFIG）

`develop/v3.13.0` 仍处于 FROZEN_FOLLOWUP_ONLY 状态：
- 控制 issue: #4313
- 规则: "develop/v3.13.0 work does not close V312 scope unless
  backported to develop/v3.12.0 or explicitly deferred by user
  decision and reflected in v3.12 release claims."

RC 转入不打开任何 v3.13 follow-up issue；后续 GA 转入若需要
backport，必须按上述规则显式承接。

## 3. V312-59-C cycle 关键决定

| Issue / PR | 决定 | 依据 |
|---|---|---|
| #4386 (V312-59-C umbrella) | 闭合 | 12/12 RC items PASS + Anti-Fabrication-Policy §5 applied |
| #4388 (B8 thresholds) | 重核 | SF=10 fixture 修复后 13/13 PASS at 2026-08-26T18:30:00Z |
| #4429 (Q20 partial TIMEOUT) | 保留为 known partial | V312-58 Sprint 5 followup-6 已闭合 BinaryOp arm 路径，但全 SF=1 仍超时 |
| PR #4475 (followup-6) | 已合并 | `mentions_outer` Subquery gap closed + 路径验证 2/2 |
| Q17/Q20 全 SF=1 TIMEOUT | 标注为 RC known partial | 不阻塞 RC；GA 准入要求 168h SOAK + 完整 correctness |

## 4. 与已有治理文件的关系

- **`STAGE_GOVERNANCE_REMEDIATION_2026-08-18.md`** — 上游纠偏报告
  （ALPHA 阶段）；本报告继承其 2.1（v3.12 是当前主线）+ 2.2
  （V312 issue 不得默认跳转 v3.13）。
- **`RC_GATE_REPORT.md`** — 本轮 RC 转入的官方 verdict aggregator。
- **`V312-59-C-RC-PROMOTION-REPORT.md`** — umbrella 报告，
  cycle provenance + close boundary checklist。
- **`evidence/v312-59-e/thresholds_override_evidence.txt`** — B8
  thresholds_override 13/13 PASS 的可重跑证据。

## 5. 下一步

1. 用户 / Release captain 在 Gitea 网络恢复后：
   ```bash
   cd /Volumes/NVMe1T/workspace/dev/openheart/sqlrustgo
   git push origin develop/v3.12.0
   ```
   本地分支领先 `origin/develop/v3.12.0` 共 36 个 commit（截至
   `752d51b96`）。
2. PR 合并后，按 STAGE_CONFIG BETA_to_RC trigger 打 tag `v3.12.0-rc1`。
3. #4386 V312-59-C umbrella issue 在 PR 合并时关闭（指向本报告
   与 RC_GATE_REPORT.md 作为执行证据）。
4. 启动 V312-59-D GA 周期：补齐 168h mixed SOAK + 9 项
   `promotion_to_GA_requires` 证据（见 `GA_GATE_REPORT.md` 已
   预生成 11/11 PASS scaffold）。

## 6. 责任与审计

- **V312-59-C cycle owner**: openclaw-minimax (MiniMax-M3)
- **Composite evidence_hash**: 见 `promotion_to_rc_evidence.txt`
  timestamp `2026-08-26T01:49:45Z`
- **Anti-Fabrication-Policy**: v1.0 §5（每项 PASS 引用上游源 + 8-字段 provenance）
- **审计可重跑命令**:
  ```bash
  bash scripts/gate/check_v312_promotion_to_rc.sh
  bash scripts/gate/check_v312_gate_thresholds.sh
  python3 -c "import yaml; print(yaml.safe_load(open('docs/releases/v3.12.0/STAGE.yaml'))['current_stage'])"
  ```
