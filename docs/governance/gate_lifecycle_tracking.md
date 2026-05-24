# GMP-Platform 门禁生命周期追踪规范

> **版本**: 1.0
> **日期**: 2026-05-24
> **目的**: 建立门禁失败项的闭环追踪机制，确保所有差距有 Issue、有修复、有验证
> **适用范围**: v1.3.0+ Alpha / Beta / RC / GA 各阶段门禁检查
> **SSOT**: 本文档是 GMP-Platform 门禁生命周期的唯一权威来源

---

## 一、核心原则

### 1.1 门禁失败 = 必须有 Issue

任意门禁检查项 FAIL，必须同时满足以下条件之一：

| 条件 | 处理方式 |
|------|----------|
| 能立即修复（< 1 人天） | 立即修复，验证后合并 |
| 不能立即修复 | **必须**创建 Issue 追踪，标记为当前版本的未决项 |
| 需要较长开发周期 | 将任务延续到 **下个版本**的 `DEVELOPMENT_PLAN.md`，并建立 Issue 映射 |

### 1.2 禁止的模式

```
❌ 门禁 FAIL → 跳过 → 合并代码 → Issue 未创建 → 问题丢失
❌ Issue 已创建 → 未关联到门禁失败项 → 后续无人追踪
❌ 下版本开发 → 不知道上版本有未完成任务 → 重复遗漏
❌ Issue 关闭前 → 无 PR 证据 → 直接关闭
```

### 1.3 正确的模式

```
✅ 门禁检查 FAIL → 创建 Issue + 标记 milestone → 修复 → 验证通过 → 关闭 Issue
✅ 门禁检查 FAIL → 当前版本无法修复 → 延续到 v1.4.0 → 在 v1.4.0 DEVELOPMENT_PLAN.md 中引用原 Issue
✅ Issue 关闭前 → 验证 closedByPullRequestsReferences 非空 → 才能关闭
```

---

## 二、门禁失败处理流程

### 2.1 四种处置结果

```
门禁检查 FAIL
        │
        ├── A. 能立即修复（< 1 人天）
        │       → 立即修复 → 验证 → PR 合并
        │
        ├── B. 不能立即修复但可在本版本完成
        │       → 创建 Issue → 标记 milestone → 排期修复
        │
        ├── C. 无法在当前版本完成
        │       → 创建 Issue → 标记 milestone
        │       → 登记到下版本 DEVELOPMENT_PLAN.md §6
        │       → 在本文档 §七登记
        │
        └── D. 不可抗力（外部依赖/基础设施）
                → 评估豁免可能性
                → 在 GATE_EXEMPTIONS.md 登记
                → 设置复审日期
```

### 2.2 Issue 创建模板

每个门禁失败必须创建 Issue，模板：

```markdown
## Issue 标题
[{门禁项}] {简短描述}

## Issue 内容
### 门禁来源
- 门禁脚本: check_{phase}_v130.sh
- 检查项: {L0A1/L0B2/R5 等}
- 命令: `{实际执行的命令}`

### 失败输出
```
{粘贴完整的失败输出}
```

### 根因分析
{简要分析失败原因}

### 影响范围
- 阻塞: {当前阶段门禁}
- 影响: {对其他门禁项的连带影响}

### 验收条件
- [ ] {具体可验证的条件 1}
- [ ] {具体可验证的条件 2}

### 版本延续
- [ ] 已延续到 v{next}.0 DEVELOPMENT_PLAN.md §6
- [ ] 原 Issue: #{原始 Issue 号}
```

### 2.3 Issue 关闭验证清单

**禁止在无 PR 证据时关闭 Issue**。

```
Issue 关闭前检查
====================

Issue #: ___________

[ ] 1. 执行 curl "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues/{number}"
    结果: ___________ (必须 state=closed 且有 closed_by_pr)

[ ] 2. 确认代码已合并到目标分支
    分支: ___________
    Commit: ___________

[ ] 3. 确认测试通过
    命令: ___________
    结果: ___________

[ ] 4. 确认门禁重新检查 PASS
    命令: ___________

结论: ___________ (可以关闭 / 禁止关闭 - 原因: ___________)
```

---

## 三、Issue 标签规范

创建 Issue 时，必须根据失败类型打标签：

| 标签 | 含义 | 触发条件 |
|------|------|----------|
| `source/gate-alpha` | Alpha 门禁失败 | check_alpha_v130.sh FAIL |
| `source/gate-beta` | Beta 门禁失败 | check_beta_v130.sh FAIL |
| `source/gate-rc` | RC 门禁失败 | check_rc_v130.sh FAIL |
| `source/gate-ga` | GA 门禁失败 | check_ga_v130.sh FAIL |
| `type/performance` | 性能问题 | 延迟/QPS 不达标 |
| `type/coverage` | 覆盖率问题 | 覆盖率低于阈值 |
| `type/feature` | 功能缺失 | 功能未实现 |
| `carried/v1.3.0` | 延续到 v1.3.0 | 当前版本无法完成 |

---

## 四、版本延续机制

### 4.1 延续触发条件

满足以下任一条件，必须将任务延续到下个版本：

| 条件 | 判定 |
|------|------|
| 修复需要 3 人天以上 | 超出当前版本开发周期 |
| 涉及架构变更 | 必须在下一个大版本迭代 |
| 优先级冲突 | 当前版本有更高优先级的 P0 任务 |

### 4.2 延续标准格式

在 `DEVELOPMENT_PLAN.md` §6 中建立映射：

```markdown
## v{NEXT} 延续任务（来自 v{VERSION} 未完成项）

| 原 Issue | 任务描述 | 原版本状态 | v{NEXT} 目标 | 验收条件 |
|----------|----------|------------|---------------------|----------|
| #{issue} | {任务} | {当前状态} | {目标} | {验收条件} |
```

---

## 五、追踪数据模型

```
Issue (Gitea)
  ├── milestone: vX.Y.Z-{phase}
  ├── labels: [source/gate-beta, priority/p0, type/feature]
  ├── linked_to: PR (通过 body 中引用关联)
  ├── tracked_in: gate_lifecycle_tracking.md §七
  └── carried_forward_to: v{X+1}.Y.Z DEVELOPMENT_PLAN.md §6
        │
        └── merged: true/false
              │
              └── closed: true/false (with PR evidence)
```

---

## 六、门禁脚本与规范的双向同步

### 6.1 原则

```
规范定义 ←→ 脚本实现
    ↑              ↓
    ←── 差距 ───←
```

每向发生以下情况，必须同步更新：

| 触发事件 | 同步要求 |
|----------|----------|
| 规范新增检查项 | 脚本必须在下一版本实现 |
| 脚本新增检查项 | 必须同步到规范 SSOT |
| 规范修改阈值 | 脚本必须同步更新 |
| 脚本发现更好的实现 | 必须回填到规范并说明理由 |

### 6.2 同步检查

```bash
# 检查规范中定义的检查项是否都在脚本中实现
for gate in $(grep "^| [A-Z][0-9]" gate_spec_v130.md | awk '{print $2}'); do
    if ! grep -q "$gate" scripts/gate/check_*v130.sh; then
        echo "MISSING: $gate not in any gate script"
    fi
done

# 检查脚本中的检查项是否都在规范中定义
for script_check in $(grep "check_output\|TOTAL=" scripts/gate/check_*v130.sh | grep -oE '[A-Z][0-9]+'); do
    if ! grep -q "$script_check" gate_spec_v130.md; then
        echo "EXTRA: $script_check in script but not in spec"
    fi
done
```

---

## 七、门禁生命周期记录

### v1.3.0 周期记录

#### L0-Alpha Gate 检查

**日期**: 2026-05-24

**脚本**: `scripts/gate/check_alpha_v130.sh`

| 检查项 | 命令 | 结果 | 说明 |
|--------|------|------|------|
| A1 | `cargo build --release --workspace` | ✅ PASS | |
| A2 | `cargo build --release -p gmp-cli` | ✅ PASS | |
| A3 | `cargo test --lib` | ✅ PASS | 0 failures |
| A4 | `cargo fmt --all -- --check` | ✅ PASS | |
| A5 | `cargo clippy --all-targets -- -D warnings` | ✅ PASS | |
| A6 | `python3 -m py_compile` | ✅ PASS | 15 files |
| A-F1 | `gmp-retrieve --version` | ✅ PASS | binary exists |
| A-F2 | `gmp-server` binary | ✅ PASS | binary exists |
| A-F3 | `cargo build -p gmp-event-bus` | ✅ PASS | |
| A-F4 | `cargo build -p gmp-api` | ✅ PASS | |
| A-S1 | Gate Spec exists | ✅ PASS | |
| A-S2 | Development Plan exists | ✅ PASS | |
| A-S3 | Branch `develop/v1.3.0` | ✅ PASS | |

**入口条件检查**:

| 条件 | 状态 |
|------|------|
| Workspace 全量编译成功 | ✅ 成功 |
| 所有 crate clippy 零警告 | ✅ 零警告 |
| 单元测试 ≥90% 通过 | ✅ 100% (0 failures) |

#### L0-Alpha 结果

```
L0-Alpha Gate 结果: 13/13 PASSED
状态: ✅ PASS
阻塞项: 无
执行时间: 2026-05-24
```

#### L0-Beta Gate 检查

**日期**: 待定

**入口条件**:

| 条件 | 状态 |
|------|------|
| L0-Alpha Gate PASS | ✅ 13/13 PASSED (2026-05-24) |
| 21-docs 完整性 PASS | ✅ 21/21 |
| P0 功能完成 | 🔄 #124 Layer 0 管线开发中 |

#### L0-Beta 结果

```
L0-Beta Gate 结果: X/14 PASSED
状态: 🔄 待执行
```

---

## 八、版本发布后自检

每次版本发布前，验证上版本遗留问题：

```
版本发布前检查
=================

[ ] 上版本所有 OPEN Issue 均有延续记录
[ ] 延续 Issue 已在 gate_lifecycle_tracking.md §七登记
[ ] GATE_EXEMPTIONS.md 中豁免项均有复审日期
[ ] 所有 Issue 关闭前均验证了 PR 关联证据
[ ] 门禁脚本与 gate_spec_v130.md 一致性检查通过
```

---

## 九、相关文档

| 文档 | 作用 |
|------|------|
| `gate_spec_v130.md` | v1.3.0 门禁定义 SSOT |
| `gate_lifecycle_tracking.md` | 门禁生命周期追踪（本文档） |
| `GATE_EXEMPTIONS.md` | 门禁豁免记录 |
| `ISSUE_CLOSING_VERIFICATION.md` | Issue 关闭验证流程 |
| `VERSION_DOCS_SPEC.md` | 版本文档规范 |
| `governance_self_improvement.md` | 治理自我进化机制 |

---

*本文档由 hermes agent 创建，参考 SQLRustGo governance 体系*
*最后更新: 2026-05-24*
