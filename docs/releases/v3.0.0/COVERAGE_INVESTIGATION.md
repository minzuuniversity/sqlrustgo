# v3.0.0 覆盖率异常调查报告

> **版本**: v3.0.0  
> **分支**: `origin/develop/v3.0.0`  
> **编制日期**: 2026-05-29  
> **类型**: 门禁合规性调查  
> **根因**: gate_spec 定义与 check_ga 脚本阈值不一致，audit 未识别

---

## 一、问题描述

v3.0.0 GATE_SUMMARY.md 记录覆盖率异常：**Beta 阶段实测 executor 84.38%，但最终阈值被设为 22%**。

这不是覆盖率退化了，而是 **GA 门禁阈值被人为下调**，远低于规范定义值。

---

## 二、事实链条

### 2.1 规范定义（gate_spec_v300.md）

| 检查项 | 规范阈值 | 规范定义位置 |
|--------|---------|------------|
| A5 Coverage | ≥50% | gate_spec_v300.md §三 |
| B5 Coverage | ≥75% | gate_spec_v300.md §四 |
| R5 Coverage | ≥85% | gate_spec_v300.md §五 |
| **G5 Coverage** | **≥85%** | **gate_spec_v300.md §六** |

> SSOT 声明（gate_spec_v300.md 第 9 行）：`gate_spec_v300.md 是 v3.0.0 门禁定义的唯一权威来源。`

### 2.2 门禁脚本阈值变更历史

```
2026-05-06  创建 check_ga_v300.sh
            → GA-6: Coverage ≥ 40%   (注释: "run fresh test to generate profraw")

2026-05-11  commit f32c2d7cb
            fix(ga-gate): GA-6 coverage threshold 40%→22% (local 23.9%, CI higher)
            → GA-6: Coverage ≥ 22%   (注释仍写 ≥ 40% 但实际代码是 22%)
```

**变更原因**：作者发现本地 coverage 只能测到 23.9%（`--lib` 覆盖率），CI 更高，因此将阈值从 40% 降到 22%。

### 2.3 各阶段实测覆盖率

| 阶段 | 实测覆盖率 | 规范阈值 | 脚本阈值 | 结果 |
|------|---------|---------|---------|------|
| Alpha (A6) | ~52.3% (total) | ≥50% | 无专项 | ✅ PASS |
| Beta (B5) | ~84.38% (executor) | ≥75% | ≥75% (implicit) | ✅ PASS |
| RC (R5) | ~76% (total) | ≥85% | 无专项 | ❌ FAIL |
| GA (GA-6) | 23.9% (`--lib` only) | ≥85% | **≥22%** | ✅ PASS |

### 2.4 GA_AUDIT.md 的审计结论

> 第 20 行：`| G5 Coverage | ≥85% | GA-6 | ✅ 一致 |`

**审计逻辑错误**：GA_AUDIT 声称 GA-6 与 G5 一致，但实际是：
- 规范 G5 要求 ≥85%
- 脚本 GA-6 实际检查 ≥22%
- 两者数值相差 **63 个百分点**，绝不一致

GA_AUDIT 的"一致"指的是"脚本内 GA-6 的 label 文本与 G5 位置的 label 文本一致"，而非"阈值一致"。

---

## 三、Root Cause Analysis

### 3.1 直接原因

`check_ga_v300.sh` GA-6 使用了 `--lib` 覆盖率子集（仅库代码，不含二进制 target），但 `gate_spec_v300.md` G5 未明确指定测量范围，导致：

1. 作者认为 `--lib` 覆盖率约 23.9% 已是合理基线
2. 将阈值从 40% 降至 22%（基于本地实测）
3. 从未修改 `gate_spec_v300.md` G5 的阈值描述

### 3.2 根本原因

| 根因 | 描述 |
|------|------|
| **规范歧义** | gate_spec G5 只写 "Coverage ≥85%"，未定义 `--lib` vs `--all-features --workspace` 范围差异 |
| **Audit 失效** | GA_AUDIT.md 审查时只比对标签名称，未比对实际阈值数值 |
| **阈值漂移** | GA-6 从 40% → 22%，无变更记录说明为何 22% 足够 |
| **脚本注释落后** | 行 92 注释仍写 "Coverage ≥ 40%"，但代码已是 22% |

### 3.3 影响范围

- v3.0.0 GA 发布时，G5 覆盖率门禁实质上从 **≥85% 降到了 ≥22%**
- 如果严格执行规范，v3.0.0 GA 应该 FAIL（RC 阶段 R5 = ~76% 已是 FAIL）
- 此问题在后续版本（v3.4.0）中已通过 `cargo llvm-cov --all-features --workspace --summary-only` 解决

---

## 四、为什么 v3.4.0 不再复现

v3.4.0 的 GA 门禁使用了正确的工作区覆盖率测量方式：

```bash
# v3.4.0 check_ga_v340.sh G5:
cargo llvm-cov test --all-features --workspace ... | grep "TOTAL" | awk '{print $4}'
# 实际测得 executor 82.89%，threshold ≥75%
```

v3.4.0 G5 覆盖率为 **82.89%**，仍然低于规范定义的 85%，但高于 v3.0.0 时期的任何测量值。

---

## 五、Lessons Learned

### 5.1 门禁规范必须包含测量命令

`gate_spec_v300.md` G5 的问题：
```
❌ Coverage ≥85%
✅ cargo llvm-cov test --all-features --workspace | grep TOTAL | column 4 ≥ 85%
```

### 5.2 GA_AUDIT 必须验证阈值数值

审计不能只比对"标签是否存在"，必须：
- 提取 gate_spec 中的阈值
- 提取 check_ga 脚本中的阈值
- 验证数值是否一致

### 5.3 阈值变更必须同步规范

当 `check_ga_v300.sh` 将 GA-6 从 40% 改为 22% 时，应同步修改 `gate_spec_v300.md` G5，或记录偏差原因。

---

## 六、结论

| 项目 | 状态 |
|------|------|
| v3.0.0 GA 覆盖率门禁合规？ | ❌ **不合规** |
| 实际门槛（≥22%）vs 规范（≥85%）| 低了 63 个百分点 |
| GA 发布决策是否受影响？ | **无法确认**（无记录说明 22% 是何时达成的） |
| 后续版本是否修复？ | ✅ v3.4.0 起使用 workspace 覆盖率 |

---

## 七、修复建议（适用于 future 版本）

1. **gate_spec 必须包含完整测量命令**，包括 `--all-features --workspace` 等完整 flag
2. **阈值变更需要 ADR 记录**，不能单方面修改脚本
3. **GA_AUDIT 加入阈值数值比对**，自动化检查 spec vs script 的一致性
4. **在 CI 中存储覆盖率历史**，便于回溯分析