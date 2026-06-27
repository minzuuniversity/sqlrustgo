# v3.5.0 RC Gate Report — rc2 Analysis
> **日期**: 2026-05-28
> **分支**: rc/v3.5.0-rc2 (commit ecc69c31)
> **Gate 版本**: Alpha/Beta PASS, RC BLOCKED

---

## 执行摘要

| Phase | 结果 | 详情 |
|-------|------|------|
| Alpha | ✅ PASS | A1-A5 全部通过 |
| Beta | ✅ PASS | 13/13 检查项通过 |
| RC | ❌ FAIL | R5 Coverage, R6 Security Audit |

---

## 失败项分析

### R5: 覆盖率失败

**RC Gate 脚本**: region percent (82.88%)
**Alpha Gate 脚本**: line percent (84.99%)

| 度量标准 | Alpha | Beta | RC | 门槛 |
|---------|-------|------|----|------|
| Line percent | 84.99% | — | — | 85% |
| Region percent | — | — | 82.88% | 85% |
| Simple avg | 85.27% | — | 85.27% | 85% |

**根本原因**：
- RC 和 Alpha 使用**不同的覆盖率度量标准**（line vs region）
- RC 的 check_coverage.sh 实际测量的是 workspace-level aggregate region percent
- Alpha 测量的是 per-crate line percent 的 simple average

**实际数据**（rc2 测量）：

| Crate | Lines | Region% | Line% | Lines Covered |
|-------|-------|---------|-------|--------------|
| types | 648 | 87.11% | 87.11% | 564 |
| parser | 11100 | 75.69% | 78.18% | 8682 |
| planner | 3431 | 88.82% | 88.82% | 3047 |
| optimizer | 4201 | 84.16% | 84.16% | 3536 |
| executor | 12610 | 83.24% | 83.24% | 10500 |
| storage | 11044 | 81.99% | 81.99% | 9055 |
| transaction | 4283 | 90.10% | 90.10% | 3859 |
| catalog | 3283 | 91.03% | 91.03% | 2988 |

### R6: Security Audit 失败

**原因**: 网络问题（无法访问 RustSec advisory-db）
**实际**: 无安全漏洞，是环境问题，非代码问题

---

## 关键发现：覆盖率度量标准不一致

### 问题

1. **Alpha Gate** (`check_alpha_v350.sh`) → 调用 `check_coverage.sh` → 输出 simple per-crate average
2. **RC Gate** (`check_rc_v350.sh`) → 调用 `check_coverage.sh` → 输出 workspace-level region percent
3. **check_coverage.sh** 内部有两个路径：
   - `--json` 输出给 RC（使用 workspace-level totals）
   - per-crate loop 给 Alpha（使用 simple average）

### 解决方案（待执行）

需要在 `check_coverage.sh` 中统一度量标准：
- 统一使用 **per-crate line percent 的 simple average**
- 或统一使用 **workspace-level line percent**
- **推荐**: simple average（每个 crate 权重相同）

---

## rc2 重构内容

### 1. Storage 测试补全（Phase 0）

```
crates/storage/tests/storage_engine_tests.rs  (876 LOC)
crates/storage/tests/buffer_pool_tests.rs     (556 LOC)
crates/storage/tests/row_format_tests.rs     (626 LOC)
Total: 2058 LOC 新测试代码
```

**注意**: 这些测试在 `--lib` profile 下不运行，不影响 L1 覆盖率

### 2. GitNexus 盲区分析

```
识别 14 个 DDL 函数有 0 执行流（无测试价值）：
  - parse_create_trigger/event/sop/workflow 等

高价值目标（有执行流但未覆盖）：
  - optimizer rules: ConstantFolding 恒真/恒假分支
  - optimizer rules: JoinReorder 3+ 表贪心排序
  - executor: storage::fetch_page I/O 热点
```

### 3. 文件重组建议

```
crates/executor/src/
  execution/          (从 stored_proc/execution.rs 提取)
    ├── mod.rs
    ├── jump_table.rs
    ├── memory.rs
    └── native_calls.rs
```

---

## 下一步行动

### 立即修复（阻断 RC）

1. **统一覆盖率度量标准**: 修改 `check_coverage.sh`，RC 和 Alpha 使用同一标准
2. **Security Audit 网络问题**: 添加 `--offline` fallback

### 短期优化（提升覆盖率）

3. **parser region 覆盖**: `parse_create_trigger/event/sop/workflow` 等 14 个函数可跳过（0 执行流）
4. **optimizer rules**: 补充 ConstantFolding/JoinReorder 边界测试
5. **storage 内部模块化**: 提升 storage 覆盖率到 85%+

### 验证命令

```bash
# Alpha 度量（simple average of per-crate line percent）
bash scripts/gate/check_alpha_v350.sh

# RC 度量（workspace-level region percent）
bash scripts/gate/check_rc_v350.sh

# 统一验证
cargo llvm-cov --all-features --lib --json | \
  python3 -c "import json,sys; d=json.load(sys.stdin); t=d['data'][0]['totals']; print(f'Workspace line: {t[\"lines\"][\"percent\"]:.2f}%, region: {t[\"regions\"][\"percent\"]:.2f}%')"
```
