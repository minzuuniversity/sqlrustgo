# 测试系统与门禁检查体系整改分析

> **版本**: v1.0
> **日期**: 2026-05-21
> **维护人**: hermes-agent
> **范围**: 测试系统 + 门禁检查体系

---

## 一、现状总结

### 1.1 测试文件统计

| 目录 | 文件数 | 说明 |
|------|---------|------|
| `tests/` | 65 | 集成测试 |
| `tests/e2e/` | 7 | 端到端测试 |
| `tests/ci/` | 3 | CI 专用测试 |
| `crates/*/tests/` | ~50+ | 单元测试 |

### 1.2 门禁脚本统计

| 类别 | 数量 | 示例 |
|------|------|------|
| 版本门禁脚本 | 24 | `check_alpha_v330.sh`, `check_beta_v320.sh` |
| 功能门禁脚本 | 18 | `check_tpch.sh`, `check_coverage.sh` |
| GMP 门禁脚本 | 12 | `check_electronic_signature.sh` |
| 辅助脚本 | 14 | `audit_*.sh`, `gate_*.sh` |
| **总计** | **68** | |

### 1.3 高内存消耗测试

| 测试 | 内存消耗 | 原因 |
|------|---------|------|
| TPC-H SF=10 | ~8.5GB+ | 60M lineitem + 15M orders |
| 72h Stability Test | 持续增长 | 长时间运行 + 内存泄漏累积 |
| buffer_pool_benchmark_test | ~2-4GB | 大量页面缓存 |
| llvm-cov coverage | ~4-8GB | Rust 编译 + 插装 |

---

## 二、问题分析

### 2.1 门禁脚本问题

#### 问题 1: 版本脚本冗余

```
check_alpha_v300.sh
check_alpha_v310.sh
check_alpha_v320.sh
check_alpha_v330.sh  ← 当前版本
check_beta_v300.sh
check_beta_v310.sh
check_beta_v320.sh
check_beta_v330.sh  ← 不存在
check_ga_v300.sh
check_ga_v310.sh
check_ga_v320.sh
check_ga.sh         ← 通用版本
```

**问题**: 每个版本创建独立脚本，导致维护负担。

#### 问题 2: v3.4.0 Gate 脚本缺失

- 存在: `docs/releases/v3.4.0/BETA_GATE_CHECKLIST.md` ✅
- 存在: `docs/releases/v3.4.0/GA_GATE_CHECKLIST.md` ✅
- 缺失: `scripts/gate/check_beta_v340.sh` ❌
- 缺失: `scripts/gate/check_ga_v340.sh` ❌

#### 问题 3: 脚本硬编码版本号

```bash
# check_beta.sh 第28行
check "Docs: VERSION_PLAN.md" "test -f docs/releases/v2.9.0/VERSION_PLAN.md"
```

脚本与版本强耦合，无法通用化。

#### 问题 4: 部分脚本引用不存在的文件

```bash
# check_regression.sh 引用
bash scripts/gate/check_regression.sh --skip-run
# 但 scripts/gate/check_regression.sh 可能不存在或已废弃
```

### 2.2 测试系统问题

#### 问题 5: 测试分类不清晰

- 65 个集成测试文件无明确分类
- 未标记 `#[ignore]` 的大内存测试
- 性能测试与功能测试混杂

#### 问题 6: 缺少内存限制配置

```toml
# Cargo.toml 中缺少测试内存配置
[profile.test]
opt-level = 2
debug = false
```

#### 问题 7: TPC-H SF=10 无保护机制

```rust
// crates/bench/examples/tpch_sf10_importer.rs
const SF10_LINEITEM: usize = 60_000_000;  // ~6GB 内存
```

**风险**: 开发机器运行此测试会 OOM。

### 2.3 门禁检查问题

#### 问题 8: Beta Gate 入口条件检查缺失

```markdown
## Beta Gate 入口条件
- [x] Alpha Gate 通过
- [x] 所有 P0 功能完成
- [ ] 单元测试 ≥90% 通过  ← 实际未执行
```

**问题**: Beta Gate 文档列出了入口条件，但 `check_beta.sh` 未验证。

#### 问题 9: 覆盖率目标不一致

| Gate | 总覆盖率 | Executor 覆盖 |
|------|---------|-------------|
| Alpha | ≥75% | ≥60% |
| Beta | ≥85% | ≥70% |
| RC | ≥90% | ≥85% |
| GA | ≥95% | ≥90% |

**问题**: 不同版本的覆盖率目标在脚本中硬编码，难以追踪。

#### 问题 10: 缺少 Gate 执行结果持久化

Gate 脚本仅输出到 stdout，未生成结构化报告：
```bash
echo "=== Beta Gate Results: PASS=$PASS / $TOTAL ==="
```

**建议**: 生成 JSON 报告如 `gate_results_v340_beta.json`。

---

## 三、瓶颈分析

### 3.1 内存瓶颈

```
┌─────────────────────────────────────────────────────────────┐
│                    内存消耗分层                               │
├─────────────────────────────────────────────────────────────┤
│ Level 1: <2GB     │ 单元测试、parser tests               │
│ Level 2: 2-4GB   │ integration tests, buffer pool tests  │
│ Level 3: 4-8GB   │ llvm-cov coverage, TPC-H SF=0.1     │
│ Level 4: 8-16GB  │ TPC-H SF=1, e2e tests               │
│ Level 5: 16GB+   │ TPC-H SF=10, 72h stability         │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 执行时间瓶颈

| 测试类型 | 预计时间 | 瓶颈 |
|----------|----------|------|
| `cargo test --lib` | 5-10 min | 编译 + 执行 |
| `cargo test --test '*'` | 15-30 min | 集成测试 |
| `cargo llvm-cov` | 20-40 min | 覆盖率收集 |
| TPC-H SF=1 | 10-30 min | 查询执行 |
| 72h Stability | 72h | 长时间运行 |

### 3.3 并行化不足

当前 `check_coverage.sh` 虽有并行化：
```bash
# 但仅针对 L1 crate 并行
for crate in "${L1_CRATES[@]}"; do
    get_crate_coverage "$crate" &
done
wait
```

**问题**: 未区分内存密集型测试与轻量级测试。

---

## 四、整改方案

### 4.1 门禁脚本重构

#### 方案 1: 创建统一 Gate 框架

```bash
#!/usr/bin/env bash
# check_gate.sh - 通用 Gate 框架
set -euo pipefail

VERSION="${1:-v3.4.0}"
PHASE="${2:-beta}"  # alpha, beta, rc, ga

GATE_DOC="docs/releases/${VERSION}/${PHASE^^}_GATE_CHECKLIST.md"
GATE_SCRIPT="scripts/gate/check_${PHASE}_v${VERSION#v}.sh"

# 读取 Gate 文档并执行检查
```

#### 方案 2: 清理废弃脚本

```bash
# 删除 v3.0.0 - v3.2.0 的版本特定脚本
rm scripts/gate/check_alpha_v300.sh
rm scripts/gate/check_alpha_v310.sh
rm scripts/gate/check_alpha_v320.sh
# ... 保留通用版本
```

#### 方案 3: 创建 v3.4.0 Gate 脚本

```bash
#!/usr/bin/env bash
# check_beta_v340.sh
set -euo pipefail

VERSION="v3.4.0"
PHASE="beta"

# B1-B4: 代码质量
cargo build --release --workspace
cargo test --lib -- -n 4  # 4 并行

# B5-B8: GMP API
cargo build -p sqlrustgo-gmp-api
cargo test -p sqlrustgo-gmp-api --lib

# B9-B11: GMP Retrieval v2
cargo test -p sqlrustgo-gmp-retrieval -- bm25
cargo test -p sqlrustgo-gmp-retrieval -- rrf

# B12-B14: Trust Infrastructure
cargo test -p sqlrustgo-evidence-engine --lib
cargo test -p sqlrustgo-workflow-v2 --lib
cargo test -p sqlrustgo-trust-viz --lib

# 生成报告
echo '{"gate": "beta", "version": "v3.4.0", "status": "PASS"}' > gate_results_v340_beta.json
```

### 4.2 测试系统重构

#### 方案 4: 标记高内存测试

```rust
// tests/tpch_sf10_test.rs
#[test]
#[ignore = "需要 16GB+ 内存，仅在专用 CI runner 执行"]
fn test_tpch_sf10_queries() {
    // ...
}
```

#### 方案 5: 添加内存感知配置

```toml
# Cargo.toml
[profile.test]
opt-level = 2
debug = false

# 在本地测试中使用较少内存
[profile.dev.test]
opt-level = 1

# 用于高内存测试的独立配置
[profile.heavy-test]
inherits = "release"
opt-level = 3
debug = false
```

#### 方案 6: 测试分层执行

```bash
#!/usr/bin/env bash
# run_tests.sh - 分层测试执行
set -euo pipefail

MEMORY_GB=$(free -g 2>/dev/null | awk '/^Mem:/{print $2}' || echo 16)

if [ "$MEMORY_GB" -lt 8 ]; then
    echo "Running L1 (low memory) tests only..."
    cargo test --lib -- -n 1
elif [ "$MEMORY_GB" -lt 16 ]; then
    echo "Running L1+L2 tests..."
    cargo test --lib -- -n 2
    cargo test --test integration -- -n 2
else
    echo "Running all tests including L3..."
    cargo test --all-features
fi
```

### 4.3 门禁结果持久化

#### 方案 7: JSON 格式报告

```bash
# 在 Gate 脚本末尾生成
cat > "gate_results_${VERSION}_${PHASE}.json" <<EOF
{
    "gate": "${PHASE}",
    "version": "${VERSION}",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "commit": "$(git rev-parse HEAD)",
    "status": "$STATUS",
    "pass": $PASS,
    "total": $TOTAL,
    "blocked": $BLOCKERS,
    "results": [
        {"name": "B1", "status": "PASS"},
        ...
    ]
}
EOF
```

---

## 五、执行计划

### Phase 1: 清理 (1 天)

| 任务 | 命令 | 状态 |
|------|------|------|
| 删除废弃 gate 脚本 | `rm scripts/gate/check_alpha_v300.sh` 等 | ⏳ |
| 创建 v3.4.0 gate 脚本 | `cat > check_beta_v340.sh` | ⏳ |
| 更新 .gitignore | 添加 `gate_results_*.json` | ⏳ |

### Phase 2: 测试标记 (1 天)

| 任务 | 文件 | 状态 |
|------|------|------|
| TPC-H SF=10 标记 `#[ignore]` | `tests/tpch_sf10_test.rs` | ⏳ |
| 72h 测试标记 `#[ignore]` | `tests/stability_72h_test.rs` | ⏳ |
| 添加内存配置到 Cargo.toml | `Cargo.toml` | ⏳ |

### Phase 3: 门禁框架 (2 天)

| 任务 | 状态 |
|------|------|
| 创建通用 `check_gate.sh` 框架 | ⏳ |
| 实现 Gate 文档解析 | ⏳ |
| 添加 JSON 报告生成 | ⏳ |

### Phase 4: CI 集成 (1 天)

| 任务 | 状态 |
|------|------|
| 更新 Gitea Actions workflow | ⏳ |
| 配置内存感知测试 runner | ⏳ |
| 添加 gate 结果上传 | ⏳ |

---

## 六、风险与缓解

### 风险 1: 删除旧脚本可能丢失历史信息

**缓解**: 在删除前创建归档分支 `archive/gate-scripts-v3x`

### 风险 2: 标记 `#[ignore]` 可能导致测试被遗忘

**缓解**: 在 Gate 文档中明确列出所有忽略的测试，并要求定期验证

### 风险 3: 新框架可能破坏现有 CI

**缓解**: 先在 feature 分支验证，PR 审查通过后再合并

---

## 七、预期收益

| 指标 | 整改前 | 整改后 |
|------|--------|--------|
| Gate 脚本数量 | 68 | ~15 |
| 测试执行时间 (内存 < 8GB) | OOM | < 10 min |
| Gate 结果可追溯性 | 无 | JSON 报告 |
| 高内存测试保护 | 无 | `#[ignore]` 标记 |

---

*本报告由 hermes-agent 生成 | 2026-05-21*
