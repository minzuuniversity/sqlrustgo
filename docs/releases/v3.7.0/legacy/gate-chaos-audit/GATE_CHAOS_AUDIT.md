# Gate Chaos Audit Report — SQLRustGo v3.7.0

> **Version**: v3.7.0 GA Final
> **Branch**: `origin/develop/v3.7.0`
> **Date**: 2026-05-30
> **Auditor**: Hermes Agent

---

## 一、现状总览

### 1.1 门禁脚本散落位置

```
gate/                          ← CI 实际调用的入口
  └── hermes_gate.sh           ← L1 基础检查（clippy/fmt/python/shell syntax）
  
scripts/gate/                  ← 按功能分类的门禁脚本集合
  ├── gate.sh                  ← 废弃 v2.7.0 遗留，CI 不调用
  ├── check_coverage.sh        ← 覆盖率检查，未集成 CI
  ├── check_perf_baseline.sh   ← 性能基线，stub 实现
  ├── check_security.sh        ← 安全检查
  ├── check_docs.sh            ← 文档检查
  ├── check_evidence_binding.sh← 证据绑定检查，存在但未集成 CI ⚠️
  ├── check_performance.sh      ← 性能检查
  ├── check_sql_compat.sh     ← SQL 兼容性
  ├── check_alpha.sh          ← Alpha 门禁
  ├── check_plan_integrity.sh  ← 计划完整性
  ├── check_proof.sh          ← 证据证明
  ├── check_attack_surface.sh  ← 攻击面
  ├── check_docs_links.sh      ← 文档链接
  ├── audit_*.sh              ← 4 个审计脚本
  ├── run_hermes_gate.sh       ← 调用旧验证引擎，重复
  ├── log_gate.sh            ← 日志
  ├── gate_webhook.sh         ← webhook
  ├── send_gate_*.sh          ← 告警
  └── verify_beta_entry.sh    ← Beta 入口验证

.gitea/workflows/
  └── ci.yml                  ← CI 定义（仅在 v2.8.0 相关分支运行）
```

### 1.2 CI Workflow 问题

| 问题 | 描述 |
|------|------|
| **分支范围过窄** | 仅在 `develop/v2.8.0` / `beta/v2.8.0` / `ci/*` 运行，**不在 v3.7.0 运行** |
| **根目录 gate/** | `gate/hermes_gate.sh` 是实际 CI 入口，但文档在 `scripts/gate/` |
| **postcheck 重复** | postcheck 调用 `gate/hermes_gate.sh` + `verify/verification_engine.py` + `audit/self_audit.py`，但 `scripts/gate/run_hermes_gate.sh` 也做类似的事 |
| **evidence binding 缺失** | `check_evidence_binding.sh` 存在但未在 CI 中调用 |
| **L2/L3 缺失** | v3.7.0 CI 无 execution consistency 和 ACID 检查 |

---

## 二、门禁体系重构方案

### 2.1 统一门禁入口

**删除**：废弃脚本
- `scripts/gate/gate.sh`（废弃 v2.7.0 遗留）
- `scripts/gate/run_hermes_gate.sh`（重复功能）

**统一入口**：`gate/gate.sh`（重写）

```bash
#!/usr/bin/env bash
# gate/gate.sh — 统一门禁入口
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${1:-}"
EXIT_CODE=0

echo "=== SQLRustGo Gate Check v3.7.0 ==="
echo "Version: ${VERSION:-unknown}"
echo "Commit: $(git rev-parse HEAD)"
echo ""

# L1: Code Quality
echo "[L1] Running code quality checks..."
bash "$ROOT/scripts/gate/check_alpha.sh"       || { echo "[FAIL] L1 alpha"; EXIT_CODE=1; }

# L1: Coverage
echo "[L1] Running coverage check..."
bash "$ROOT/scripts/gate/check_coverage.sh"     || { echo "[FAIL] L1 coverage"; EXIT_CODE=1; }

# L1: Security
echo "[L1] Running security check..."
bash "$ROOT/scripts/gate/check_security.sh"     || { echo "[FAIL] L1 security"; EXIT_CODE=1; }

# L2: Performance baseline (if established)
if [ -f "$ROOT/benchmark_baseline.json" ]; then
    echo "[L2] Running performance check..."
    bash "$ROOT/scripts/gate/check_perf_baseline.sh" || EXIT_CODE=1
else
    echo "[L2] SKIP performance (no baseline)"
fi

# L3: SQL compatibility
echo "[L3] Running SQL compatibility check..."
bash "$ROOT/scripts/gate/check_sql_compat.sh"   || EXIT_CODE=1

# Evidence binding
echo "[Governance] Running evidence binding check..."
bash "$ROOT/scripts/gate/check_evidence_binding.sh" "$VERSION" "$ROOT/gate/evidence_binding_out/" || {
    echo "[WARN] Evidence binding check failed"
    EXIT_CODE=1
}

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo "=== Gate Result: PASSED ==="
else
    echo "=== Gate Result: FAILED ==="
fi

exit $EXIT_CODE
```

### 2.2 CI Workflow 修复

**修复 1：扩展分支范围到 v3.7.0**

```yaml
on:
  push:
    branches:
      - develop/v2.8.0
      - develop/v3.7.0              # ← 新增
      - beta/v2.8.0
      - ci/gitea-compat
      - ci/v2.8.0-*
  pull_request:
    branches:
      - develop/v2.8.0
      - develop/v3.7.0              # ← 新增
      - beta/v2.8.0
      - ci/gitea-compat
      - ci/v2.8.0-*
```

**修复 2：统一 postcheck**

```yaml
  postcheck:
    runs-on: [hp-z6g4]
    needs: [test]
    steps:
      - name: Checkout
        run: |
          git clone --depth=1 "http://192.168.0.252:3000/$REPO" repo
          cd repo
          git fetch --depth=1 origin "$GITEA_SHA"
          git checkout "$GITEA_SHA"
      - name: Gate
        run: |
          cd repo
          chmod +x gate/gate.sh
          bash gate/gate.sh
      - name: Evidence binding
        run: |
          cd repo
          bash scripts/gate/check_evidence_binding.sh "${VERSION:-v3.7.0}" "gate/evidence_binding_out/"
```

### 2.3 覆盖率统一（PR-900 工作的一部分）

```bash
# 统一命令
cargo llvm-cov --workspace --ignore-funcs --output-format lcov > lcov.info

# Z6G4 vs Z440 cross-validation
# CI 在两个平台各跑一次，比对 delta < 10pp
```

---

## 三、门禁脚本功能映射（重构后）

| 层级 | 检查项 | 脚本 | CI 集成 | 说明 |
|------|--------|------|---------|------|
| L1 | build | CI implicit | ✅ | CI lint-build job |
| L1 | test | CI implicit | ✅ | CI test job |
| L1 | clippy | CI implicit | ✅ | CI lint-build job |
| L1 | fmt | CI implicit | ✅ | CI lint-build job |
| L1 | coverage ≥ 50% | `scripts/gate/check_coverage.sh` | ✅ gate.sh | 统一入口调用 |
| L1 | security | `scripts/gate/check_security.sh` | ✅ gate.sh | |
| L1 | docs | `scripts/gate/check_docs.sh` | ✅ gate.sh | |
| L1 | alpha entry | `scripts/gate/check_alpha.sh` | ✅ gate.sh | |
| L2 | execution consistency | `scripts/test/execution_consistency_harness.py` | ❌ 缺失 | v3.8.0 PR-850 后加 |
| L2 | perf baseline | `scripts/gate/check_perf_baseline.sh` | ⚠️ stub | 需要真实实现 |
| L3 | ACID isolation | `scripts/test/isolation_test_suite.py` | ❌ 缺失 | v3.8.0 PR-890 后加 |
| L3 | crash recovery | `scripts/test/crash_simulation.py` | ❌ 缺失 | v3.8.0 PR-830 后加 |
| Governance | evidence binding | `scripts/gate/check_evidence_binding.sh` | ✅ gate.sh | 证据伪造检查 |
| Governance | truthfulness | `scripts/gate/check_truthfulness.sh` | ❌ 缺失 | 新增 |
| Governance | plan integrity | `scripts/gate/check_plan_integrity.sh` | ✅ gate.sh | |

---

## 四、建议的门禁脚本清理

### 4.1 删除废弃脚本

| 脚本 | 理由 |
|------|------|
| `scripts/gate/gate.sh` | 废弃 v2.7.0 遗留，功能被 `gate/gate.sh` 替代 |
| `scripts/gate/run_hermes_gate.sh` | 重复功能，调用旧验证引擎 |
| `scripts/gate/collect_self_opt_metrics.sh` | 内部优化指标收集，非门禁 |
| `scripts/gate/log_gate.sh` | 日志功能，可选保留 |

### 4.2 保留并维护的脚本

| 脚本 | 用途 |
|------|------|
| `gate/gate.sh` | **统一门禁入口**（重写） |
| `gate/hermes_gate.sh` | **CI postcheck 实际调用**（轻量 L1 检查） |
| `scripts/gate/check_*.sh` | 功能模块化，按需调用 |
| `scripts/gate/check_evidence_binding.sh` | 证据绑定检查，集成到 gate.sh |

### 4.3 待实现的检查

| 检查 | 状态 | 说明 |
|------|------|------|
| L2 Execution Consistency | ❌ 缺失 | v3.8.0 PR-850 后实现 |
| L3 ACID Isolation | ❌ 缺失 | v3.8.0 PR-890 后实现 |
| L3 Crash Recovery | ❌ 缺失 | v3.8.0 PR-830 后实现 |
| Truthfulness Check | ❌ 缺失 | 新增，防止改文档过门禁 |
| Coverage unified | ⚠️ 未统一 | v3.8.0 PR-900 统一工具 |

---

## 五、CI 门禁增强计划（v3.7.0）

### 5.1 P0 — 立即修复

1. **CI Workflow 扩展到 v3.7.0 分支**
2. **统一 gate.sh 入口**，消除 `gate/hermes_gate.sh` 和 `scripts/gate/gate.sh` 并存混乱
3. **覆盖率检查集成 CI**（≥50% 基线）

### 5.2 P1 — 下一步

1. **evidence binding 集成到 CI postcheck**
2. **perf baseline 真实实现**（当前只有 stub）
3. **更新 CI 分支触发器**

### 5.3 P2 — v3.8.0

1. L2 Execution Consistency 检查（PR-850 后）
2. L3 ACID Isolation 检查（PR-890 后）
3. Truthfulness 检查（新增）

---

## 六、结论

| 决策 | 建议 |
|------|------|
| **门禁脚本混乱** | 统一入口为 `gate/gate.sh`，删除废弃脚本 |
| **CI 分支范围** | 扩展到 `develop/v3.7.0`（当前仅 v2.8.0） |
| **evidence binding** | 集成到 CI postcheck |
| **L2/L3 门禁** | v3.8.0 PR-850/890 后再加 |
| **覆盖率统一** | PR-900 第一优先级（49pp delta 问题） |