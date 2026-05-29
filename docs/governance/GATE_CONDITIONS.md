# Gate Conditions Definition

## 概述

本文档定义 SQLRustGo 门禁系统中各种状态的语义，确保治理体系无漏洞、可追溯。

---

## Alpha Gate

### PASS

所有 A1-A5 指标全部 PASS:

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| A1 | Build | cargo build --release --workspace | PASS |
| A2 | Test | cargo test --lib --workspace --exclude sqlrustgo-mysql-server | PASS (0 failure) |
| A3 | Clippy | cargo clippy --all-features -- -D warnings | PASS |
| A4 | Format | cargo fmt --all -- --check | PASS |
| A5 | Coverage | L1 8 crates 综合平均 | >= 75% |

### CONDITIONAL PASS

当 A1-A4 硬性指标 PASS，但 A5 Coverage 介于 50%-75% 之间时使用。

**必须满足的条件**:

1. **所有 A1-A4 硬性指标必须 PASS**
2. **A5 Coverage 必须 >= 50%** (否则 FAIL)
3. **每个 crate 必须 >= 50%** (否则 FAIL)
4. **必须创建 Issue 追踪覆盖率问题**
5. **必须制定 Coverage 改善计划**
6. **CONDITIONAL 状态必须在 2 周内解除**

**禁止条件**:
- A1-A4 任一项 FAIL → 不得 CONDITIONAL
- 任何 crate Coverage < 50% → 不得 CONDITIONAL
- Coverage < 50% → 直接 FAIL

### FAIL

A1-A5 任一项不满足标准。

---

## Beta Gate

### 入口条件

1. Alpha Gate PASS 或 CONDITIONAL PASS (2 周内解除)
2. ALPHA_GATE_REPORT.md 存在
3. DEVELOPMENT_PLAN.md 存在
4. TEST_PLAN.md 存在
5. COVERAGE_ANALYSIS_REPORT.md 存在

### PASS

所有 B1-B8 指标全部 PASS:

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| B1 | Build | cargo build --release --workspace | PASS |
| B2 | Workspace test | cargo test --workspace >= 90% | PASS |
| B3 | Clippy zero | cargo clippy --all-features -- -D warnings | PASS |
| B4 | Format | cargo fmt --all -- --check | PASS |
| B5 | Coverage L1 | L1 avg >= 85% | PASS |
| B6 | TPC-H SF=0.1 | 22/22 PASS | PASS |
| B7 | Security | cargo audit | PASS |
| B8 | SQL compat | SQL Corpus >= 85% | PASS |

### FAIL

B1-B8 任一项不满足标准。

---

## RC Gate

### 入口条件

1. Beta Gate PASS
2. BETA_GATE_REPORT.md 存在
3. 所有 Beta 入口问题已关闭

### PASS

所有 R1-R4 指标全部 PASS:

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| R1 | B1-B4 | Alpha-Beta 所有硬性指标 | PASS |
| R2 | TPC-H SF=1 | 22/22 PASS | PASS |
| R3 | Coverage L1 | L1 avg >= 85% | PASS |
| R4 | QPS regression | <= 5% 退化 | PASS |

### FAIL

R1-R4 任一项不满足标准。

---

## GA Gate

### 入口条件

1. RC Gate PASS
2. RC_GATE_REPORT.md 存在
3. PERFORMANCE_REPORT.md 存在
4. SECURITY_AUDIT.md 存在

### PASS

所有 GA 指标全部 PASS:

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| G1 | R1-R4 | 所有 RC 指标 | PASS |
| G2 | Full test | cargo test --workspace | PASS |
| G3 | Full coverage | L1 avg >= 85%, 每个 crate >= 80% | PASS |
| G4 | TPC-H SF=1 | 22/22 PASS | PASS |
| G5 | Security | cargo audit + 手动审计 | PASS |
| G6 | Documentation | API reference, CHANGELOG, UPGRADE_GUIDE | PASS |

### FAIL

G1-G6 任一项不满足标准。

---

## CONDITIONAL PASS 的正确使用

### 错误示例

```
Alpha Gate: CONDITIONAL PASS (81.97%)
- A5 Coverage: 81.97% >= 75% ✓
- Parser coverage: 47.16% < 75% ✗
```

**问题**: 这里使用了 CONDITIONAL PASS，但条件不明确。Parser 覆盖率 47% 远低于 75%，不符合 CONDITIONAL 的定义。

**正确做法**: 如果 Parser 47% 但总平均 81.97%，说明其他 crate 拉高了平均值。这种情况下：
- 要么 Parser 单独 Issue 追踪 + CONDITIONAL (如果总平均 >= 70%)
- 要么直接 FAIL，因为 Parser 结构性缺陷说明数据不可信

### 正确示例

```
Alpha Gate: CONDITIONAL PASS (74.5%)
- A5 Coverage: 74.5% (>= 50%, < 75%)
- 条件: 所有 crate >= 50%
- 条件: Parser 覆盖率问题创建 Issue I#2580
- 条件: 2 周内 Parser 覆盖率提升到 >= 75%
- 截止日期: 2026-06-13
```

---

## Gate 执行日志要求

每次 Gate 执行必须保存日志：

| 字段 | 要求 |
|------|------|
| 日志文件 | `docs/releases/v<版本>/logs/gate_<阶段>_<commit>_<timestamp>.log` |
| 内容 | 包含所有实际执行的命令和输出 |
| 保存时间 | 执行后立即存档 |
| 保留时间 | 永久 |

---

## 问题追踪要求

| 问题类型 | 追踪要求 |
|----------|----------|
| Coverage 不足 | 创建 Issue，包含目标版本 |
| 编译错误 | 创建 Issue，必须在下一版本修复 |
| 测试失败 | 创建 Issue，必须在当前版本修复 |
| 跨版本债务 | Issue 标题格式 `[debt:<来源版本>]` |

---

## 版本计划现实性要求

| 要求 | 说明 |
|------|------|
| 时间线依据 | 基于历史数据和团队速度 |
| 里程碑标注风险 | 如果时间线紧张，必须标注风险级别 |
| 不可压缩的最小时间 | Parser 修复: 3 周, mysql-server 修复: 2 周 |

---

**最后更新**: 2026-05-30
**维护者**: hermes-agent
**关联 Issue**: I#2584 (Alpha CONDITIONAL PASS semantics unclear)