# R5: Gate 重构 — 建立可信 CI 检查
**Issue**: #2606
**Author**: Hermes C
**Date**: 2026-05-31
**Status**: RESEARCH COMPLETED
**gate_policy_eval_id**: `run_20260601_006`

---

## 1. Issue 概述

**Retargeted from v3.7.0 Phase0 → v3.8.0**

v3.7.0 is frozen as non-transactional edition. All transaction/integration work deferred to v3.8.0.

目标：建立可信的 CI Gate 检查机制，解决当前 gate 检查存在的"不可靠"问题。

---

## 2. 当前 Gate 体系

### 2.1 Alpha / Beta / RC / GA 四级门禁

| Gate | 检查内容 | 阈值 |
|------|----------|------|
| Alpha | A1 Build / A2 Test / A3 Clippy / A4 Format / A5 Coverage | L1 8 crates >= 75% |
| Beta | B1 Build / B2 WAL Contract / B3 Clippy | Recovery 7/7 PASS |
| RC | RC-1 Build / RC-2 Test / RC-3 Coverage / RC-4 TPC-H | >= 80% |
| GA | 所有门禁 PASS + TPC-H 22/22 | 100% |

### 2.2 已知问题

| 问题 | 描述 | 证据 |
|------|------|------|
| **历史版本占位** | v3.4.0 GA_GATE_REPORT.md 声称 35/35 PASS，实际大部分为 PENDING | Truthfulness 问题 |
| **A5 覆盖率测量不一致** | Z6G4 vs Z440 delta -49pp，测量方法不同 | #2596 |
| **无超时强制** | G5 超时 >120s 未强制执行 | 门禁规范 |
| **Evidence 缺失** | Alpha/Beta/RC gate 无 evidence JSON 存档 | - |

---

## 3. Gate 重构建议

### 3.1 可立即执行（无依赖）

| 改进项 | 操作 | 影响 |
|--------|------|------|
| **Truthfulness 声明** | Alpha/Beta/RC/GA gate 报告必须包含 Truthfulness 声明头 | 低 |
| **Evidence JSON** | gate 运行结果必须写入 `artifacts/gate/v{VERSION}/{PHASE}_gate_evidence.json` | 低 |
| **超时强制** | G5 测试超时 >120s 必须 FAIL | 中 |

### 3.2 需要 PR-830 完成后执行

| 改进项 | 操作 | 依赖 |
|--------|------|------|
| **WAL Contract 测试** | RECOVERY 测试改为 L3 WAL 入口 | PR-830 |
| **B2 Beta Gate Contract** | 创建 BETA_GATE_CONTRACT.md 明确定义 B1~B3 标准 | PR-830 |

### 3.3 v3.9.0 规划

| 改进项 | 说明 |
|--------|------|
| **G-Gate 自动化** | AV1~AV10 Security 检查自动化 |
| **TPC-H CI 集成** | TPC-H 22/22 进入 CI 检查 |
| **Gate 脚本版本化** | gate 版本与 SQLRustGo 版本同步 |

---

## 4. 立即可行的改进

### 4.1 Gate 报告 Truthfulness 声明

在所有 gate 报告模板中添加：

```markdown
> **Truthfulness Declaration**: 本报告所有检查项均有真实执行证据支撑，
> 无 PENDING 占位，无历史数据冒充本次执行。
> 执行命令、输出、解析全程可审计。
```

### 4.2 Evidence JSON Schema

```json
{
  "gate": "Alpha",
  "version": "v3.8.0",
  "commit": "b579a903",
  "executed_at": "2026-05-31T12:00:00Z",
  "checks": [
    {"id": "A1", "name": "Build", "result": "PASS", "command": "cargo build --release --workspace", "output_summary": "Finished in 9.64s"}
  ]
}
```

### 4.3 超时强制（当前已实现但未强制）

当前 `scripts/gate/check_alpha_v380.sh` 中的 G5 超时检查已定义，但需确认 CI 中实际执行。

---

## 5. 结论

R5 Gate 重构的核心是**Truthfulness 机制**：
1. 所有 gate 报告必须包含 Truthfulness 声明
2. Evidence 必须存档
3. 超时必须强制 FAIL

其他改进（WAL Contract 测试、Beta Gate Contract）依赖 PR-830 完成。

---

## 附录：相关 Issue

| Issue | 描述 | 关联 |
|-------|------|------|
| #2596 | 覆盖率测量差异 | Gate 测量标准化 |
| #2584 | Alpha CONDITIONAL PASS semantics unclear | Gate 语义澄清 |
| #2579 | gate_spec缺少I-Gate集成路径检查 | Gate 覆盖完整性 |