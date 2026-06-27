# v3.7.0 Development Plan

## 1. 概述与目标

- **版本**: v3.7.0
- **主题**: 治理体系重建 + 架构清理 + Parser 覆盖率专项修复
- **成功定义**: 所有 Beta Gate B1-B8 全部 PASS，治理体系无漏洞

## 2. 问题来源

| 来源版本 | 问题 | 影响 |
|----------|------|------|
| v3.5.0 | Parser 覆盖率结构性缺陷 (47%) | 跨版本延续 |
| v2.5.0 | mysql-server 编译错误 (30+) | 跨版本延续 |
| v3.5.0 | Executor 覆盖率不足 (72%) | 跨版本延续 |
| v3.0.0 | DML 执行路径未统一 | 架构债务 |
| v3.6.0 | 治理体系存在漏洞 | 治理债务 |

## 3. 功能列表

### 3.1 P0 — 必须完成

| ID | 功能 | Issue | 状态 |
|----|------|-------|------|
| P0-1 | Parser 覆盖率专项修复 | I#2580 | 专项修复 |
| P0-2 | Executor 覆盖率提升 ≥85% | I#2582 | 补充测试 |
| P0-3 | mysql-server 编译错误修复 | I#2581 | 重写测试 |
| P0-4 | Beta Gate B1-B8 全部 PASS | — | 治理改进 |
| P0-5 | 治理体系无漏洞验证 | I#2584, I#2585 | 流程改进 |

### 3.2 P1 — 重要但可延期

| ID | 功能 | Issue | 状态 |
|----|------|-------|------|
| P1-1 | DML 执行路径统一 | I#2583 | 架构清理 |
| P1-2 | TPC-H SF=1 22/22 PASS | — | 测试补充 |
| P1-3 | SQL Corpus ≥85% | — | 测试补充 |

### 3.3 P2 — 架构愿景

| ID | 功能 | 状态 |
|----|------|------|
| P2-1 | ExecutionEngine 职责分离 | 未来架构方向 |
| P2-2 | 并行执行优化 | 基础设施完善 |

## 4. 技术任务

### 4.1 Parser 覆盖率专项修复（P0-1）

**根本原因**: 嵌套测试导致子测试覆盖率无法计入父测试。

**修复策略**:
1. 分析现有测试结构，识别嵌套测试模式
2. 将嵌套测试重构为扁平化测试结构
3. 每个测试模块独立运行 coverage
4. 验证覆盖率从 47% 提升到 ≥85%

**技术步骤**:
```
步骤 1: 扫描所有 #[test] 嵌套的 #[test] 结构
步骤 2: 创建扁平化的测试函数列表
步骤 3: 逐模块验证 coverage 数据
步骤 4: 更新 cargo test 配置以支持扁平化 coverage
```

**Issue**: I#2580
**预计时间**: 3-4 周

### 4.2 Executor 覆盖率提升（P0-2）

**目标**: 从 72% 提升到 ≥85%

**缺口分析**:
- event.rs (155行): 覆盖率缺口
- merge.rs (740行): 覆盖率缺口
- local_executor.rs (2152行): 覆盖率缺口

**修复策略**:
1. 生成详细的 coverage 报告，识别未覆盖的行
2. 补充缺失的单元测试
3. 补充集成测试

**Issue**: I#2582
**预计时间**: 2 周

### 4.3 mysql-server 编译错误修复（P0-3）

**当前状态**: 30+ 编译错误

**错误类型**:
- 11x old_password_hash 缺失
- 4x verify_old_password_response 缺失
- 8x function signature mismatch

**修复策略**:
基于 v3.5.0 修复版本重写测试模块。

**Issue**: I#2581
**预计时间**: 2 周

### 4.4 DML 执行路径统一（P1-1）

**当前架构**:
- ExecutionEngine (2577行): DML 直接调用 storage.insert/delete
- LocalExecutor (2152行): 基于 PhysicalPlan，支持并行

**目标**: 统一执行路径，DML 接入 PhysicalPlan 流水线

**技术步骤**:
```
步骤 1: 定义 DmlPhysicalPlan trait
步骤 2: 实现 Insert/Update/Delete PhysicalPlan
步骤 3: LocalExecutor 支持 DML 执行
步骤 4: 删除 ExecutionEngine 中的直接 storage 调用
步骤 5: 验证所有 DML 测试通过
```

**Issue**: I#2583
**预计时间**: 3 周

## 5. 治理体系改进

### 5.1 问题诊断

| 问题 | 影响 | 严重度 |
|------|------|--------|
| Alpha CONDITIONAL PASS 语义模糊 | 门禁形同虚设 | 🔴 CRITICAL |
| 跨版本债务无追踪 | 问题反复出现 | 🔴 CRITICAL |
| Beta 入口检查未实际执行 | 流程漏洞 | 🔴 CRITICAL |
| Gate 执行日志未存档 | 不可追溯 | 🟡 MAJOR |
| 版本计划不现实 | 期望管理失败 | 🟡 MAJOR |

### 5.2 改进方案

#### 5.2.1 CONDITIONAL PASS 语义明确化

创建 `docs/governance/GATE_CONDITIONS.md`:

```markdown
# Gate 条件定义

## Alpha Gate

### CONDITIONAL PASS 条件

当 Alpha Gate 报告中出现 CONDITIONAL PASS 时，必须满足:

1. **所有 A1-A4 硬性指标必须 PASS**:
   - A1: Build (release) PASS
   - A2: Test (lib) PASS
   - A3: Clippy PASS
   - A4: Format PASS

2. **A5 Coverage 可以 CONDITIONAL**:
   - 条件: L1 avg >= 70% (但 < 75%)
   - 条件: 每个 crate 必须 >= 50%
   - 条件: 必须有明确的 Coverage 改善计划
   - 条件: 必须有对应 Issue 追踪

3. **CONDITIONAL 必须在 2 周内解除**
```

#### 5.2.2 跨版本债务追踪机制

创建 `docs/governance/DEBT_TRACKING.md`:

```markdown
# 跨版本债务追踪

## Issue 命名规范

所有跨版本延续问题必须使用以下格式创建 Issue:
- 标题: `[debt:<来源版本>] <问题描述>`
- Example: `[debt:v3.5.0] Parser coverage 47% structural deficiency`

## Issue 模板

```markdown
## 问题描述
<描述>

## 根本原因
<分析>

## 影响版本
- <版本>: <状态>
- <版本>: <状态>

## 修复策略
<计划>

## 状态
- 创建时间: <日期>
- 来源版本: <版本>
- 目标版本: <版本>
- 状态: OPEN | IN_PROGRESS | CLOSED
```
```

#### 5.2.3 Gate 执行日志存档

创建 `scripts/gate/log_gate.sh`:

```bash
#!/bin/bash
# Gate 执行日志存档脚本
# 用法: ./log_gate.sh <alpha|beta|rc|ga> <commit_sha>

GATE_TYPE=$1
COMMIT=$2
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_DIR="docs/releases/v${VERSION}/logs"

mkdir -p "$OUTPUT_DIR"
LOG_FILE="${OUTPUT_DIR}/gate_${GATE_TYPE}_${COMMIT}_${TIMESTAMP}.log"

{
    echo "=== Gate Execution Log ==="
    echo "Type: $GATE_TYPE"
    echo "Commit: $COMMIT"
    echo "Timestamp: $TIMESTAMP"
    echo ""
    echo "--- cargo build --release --workspace ---"
    cargo build --release --workspace 2>&1
    echo ""
    echo "--- cargo test --lib --workspace ---"
    cargo test --lib --workspace 2>&1
    echo ""
    echo "--- cargo clippy ---"
    cargo clippy --all-features -- -D warnings 2>&1
} | tee "$LOG_FILE"

echo "Log saved to: $LOG_FILE"
```

#### 5.2.4 Beta 入口验证流程

创建 `scripts/gate/verify_beta_entry.sh`:

```bash
#!/bin/bash
# Beta 入口验证脚本
# 逐项验证 Beta Gate B1-B8，必须全部 PASS 才能进入 Beta

set -e

echo "=== Beta Entry Verification ==="
echo ""

PASS_COUNT=0
TOTAL_COUNT=8

# B1: Build (release)
echo -n "B1 Build (release)... "
if cargo build --release --workspace > /dev/null 2>&1; then
    echo "PASS"
    ((PASS_COUNT++))
else
    echo "FAIL"
fi

# B2: Workspace test
echo -n "B2 Workspace test... "
RESULT=$(cargo test --workspace 2>&1)
PASS_RATE=$(echo "$RESULT" | grep -o '[0-9]\+%' | head -1 | tr -d '%')
if [ "$PASS_RATE" -ge 90 ]; then
    echo "PASS ($PASS_RATE%)"
    ((PASS_COUNT++))
else
    echo "FAIL ($PASS_RATE%)"
fi

# B3: Clippy zero
echo -n "B3 Clippy zero... "
if cargo clippy --all-features -- -D warnings > /dev/null 2>&1; then
    echo "PASS"
    ((PASS_COUNT++))
else
    echo "FAIL"
fi

# B4: Format
echo -n "B4 Format... "
if cargo fmt --all -- --check > /dev/null 2>&1; then
    echo "PASS"
    ((PASS_COUNT++))
else
    echo "FAIL"
fi

# B5: Coverage L1 >= 85%
echo -n "B5 Coverage... "
COVERAGE=$(cargo coverage --all --json 2>/dev/null | jq '.l1_avg')
if (( $(echo "$COVERAGE >= 85" | bc -l) )); then
    echo "PASS ($COVERAGE%)"
    ((PASS_COUNT++))
else
    echo "FAIL ($COVERAGE%)"
fi

# B6: TPC-H SF=1
echo -n "B6 TPC-H SF=1... "
if ./run_tpch.sh --sf 1 > /dev/null 2>&1; then
    echo "PASS"
    ((PASS_COUNT++))
else
    echo "FAIL"
fi

# B7: Security
echo -n "B7 Security... "
if cargo audit > /dev/null 2>&1; then
    echo "PASS"
    ((PASS_COUNT++))
else
    echo "FAIL"
fi

# B8: SQL compat
echo -n "B8 SQL compat... "
SQL_RATE=$(./run_sql_corpus.sh --json 2>/dev/null | jq '.pass_rate')
if (( $(echo "$SQL_RATE >= 85" | bc -l) )); then
    echo "PASS ($SQL_RATE%)"
    ((PASS_COUNT++))
else
    echo "FAIL ($SQL_RATE%)"
fi

echo ""
echo "=== Summary ==="
echo "PASS: $PASS_COUNT/$TOTAL_COUNT"

if [ "$PASS_COUNT" -eq "$TOTAL_COUNT" ]; then
    echo "Beta Entry: APPROVED"
    exit 0
else
    echo "Beta Entry: REJECTED"
    exit 1
fi
```

## 6. 测试策略

### 6.1 Alpha 测试轨

```bash
cargo test --lib --workspace --exclude sqlrustgo-mysql-server
```

### 6.2 Beta 测试轨

```bash
cargo test --workspace
cargo coverage --all --json  # L1 avg >= 85%
./run_tpch.sh --sf 0.1     # 22/22 PASS
./run_sql_corpus.sh --json  # >= 85%
```

### 6.3 RC 测试轨

```bash
cargo test --workspace
cargo coverage --all --json  # L1 avg >= 85%
./run_tpch.sh --sf 1        # 22/22 PASS
./run_sql_corpus.sh --json  # >= 85%
cargo bench -- --niter 3     # QPS regression <= 5%
```

## 7. 门禁计划

| 阶段 | 入口条件 | 标准 | 状态 |
|------|----------|------|------|
| Alpha | develop/v3.7.0 分支 | A1-A5 PASS (Coverage >= 75%) | ❌ PENDING |
| Beta | Alpha PASS + P0 issues | B1-B8 PASS | ❌ PENDING |
| RC | Beta PASS | R1-R4 PASS | ❌ PENDING |
| GA | RC PASS | 全部检查 PASS | ❌ PENDING |

## 8. 里程碑

| 日期 | 事件 | 关键交付 |
|------|------|----------|
| 2026-06-01 | 开发启动 | 分支创建 |
| 2026-06-15 | Alpha Gate | A1-A5 PASS |
| 2026-07-01 | Beta Gate | B1-B8 PASS + mysql-server 修复 |
| 2026-07-15 | RC Gate | R1-R4 PASS |
| 2026-07-30 | GA Release | 全部 PASS |

## 9. 风险评估

| 风险 | 级别 | 缓解 |
|------|------|------|
| Parser 结构性缺陷修复复杂 | 🔴 | 专项投入，2 人周 |
| DML 执行路径统一影响范围广 | 🟡 | 分阶段验证，小步提交 |
| Beta 测试轨执行时间长 | 🟡 | 并行执行，优化资源 |
| 治理体系改进涉及流程变更 | 🟡 | 先在 v3.6.0 验证，再推广 |

## 10. 附录

### 10.1 关联 Issue

| Issue | 标题 | 来源 |
|-------|------|------|
| I#2580 | Parser coverage structural deficiency | v3.5.0 |
| I#2581 | mysql-server 30+ compile errors | v2.5.0 |
| I#2582 | Executor coverage below GA threshold | v3.5.0 |
| I#2583 | DML execution path not unified | v3.0.0 |
| I#2584 | Alpha CONDITIONAL PASS semantics unclear | v3.6.0 |
| I#2585 | Cross-version debt tracking missing | v3.6.0 |

### 10.2 关联文档

- `docs/releases/v3.6.0/LEGACY_ISSUE_ANALYSIS.md`
- `docs/governance/GATE_CONDITIONS.md`
- `docs/governance/DEBT_TRACKING.md`
- `scripts/gate/log_gate.sh`
- `scripts/gate/verify_beta_entry.sh`

### 10.3 治理脚本清单

| 脚本 | 用途 | 位置 |
|------|------|------|
| log_gate.sh | Gate 执行日志存档 | scripts/gate/ |
| verify_beta_entry.sh | Beta 入口验证 | scripts/gate/ |
| check_ssot_alignment.sh | SSOT 交叉检查 | scripts/gate/ |
| track_debt.sh | 跨版本债务追踪 | scripts/gate/ |