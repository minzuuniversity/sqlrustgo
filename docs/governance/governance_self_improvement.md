# GMP-Platform 治理自我进化与健康检查机制

> **版本**: 1.0
> **日期**: 2026-05-24
> **目的**: 建立治理规则的自我检查、自我反馈、自动进化机制，防止治理文档僵化
> **适用范围**: 所有 governance 文档

---

## 一、核心原则

### 1.1 治理三权分立

```
┌─────────────────────────────────────────────┐
│              治理规则 (Rule)                  │
│  gate_spec / RELEASE_POLICY / LIFECYCLE     │
└─────────────────────────────────────────────┘
                       ↑
               审查 (Audit)         执行 (Enforce)
                       ↑                       │
┌─────────────────────────────────────────────┐
│           治理执行 (Execution)               │
│  gate scripts / CI/CD / Issue tracking      │
└─────────────────────────────────────────────┘
```

### 1.2 自我进化循环

```
每次发布周期结束
       ↓
治理健康检查（本文档 §三）
       ↓
识别规范与实际执行差距（§四 差距分析）
       ↓
生成优化建议（§五 改进机制）
       ↓
更新 governance 文档
       ↓
下一周期验证效果
```

---

## 二、治理健康检查 — 每个版本发布后必须执行

### 2.1 检查频率

| 检查点 | 时机 | 执行人 |
|--------|------|--------|
| 门禁后自检 | 每个门禁检查（Alpha/Beta/RC/GA）FAIL 时 | Agent |
| 版本发布前 | 进入下一阶段前 | Human Architect |
| 版本发布后 | Tag vX.Y.Z 创建后 48h 内 | Agent |
| 重大合并后 | 每次 PR 合并后 | Agent |

### 2.2 检查清单

每次门禁 FAIL 时，必须同时执行以下检查：

#### 检查 A: 规则是否完整

```
[ ] 失败的检查项是否有对应的 Issue？
[ ] Issue 是否绑定了 milestone？
[ ] Issue 是否包含：来源门禁、检查命令、失败输出？
[ ] 是否在 gate_lifecycle_tracking.md §七登记？
```

#### 检查 B: 追踪是否闭环

```
[ ] Issue 关闭前是否验证了关联的 PR 已合并？
[ ] 修复的 PR 是否已合并到目标分支？
[ ] 相关测试是否在本地或 CI 通过？
[ ] 门禁重新检查是否 PASS？
```

#### 检查 C: 版本延续是否执行

```
[ ] 无法在当前版本修复的问题是否已延续到下版本？
[ ] 是否在 DEVELOPMENT_PLAN.md §6 建立了映射？
[ ] 下版本的 Issue 是否引用了原 Issue？
```

#### 检查 D: 豁免是否登记

```
[ ] 无法修复且可豁免的问题是否在 GATE_EXEMPTIONS.md 登记？
[ ] 豁免是否经过 Tech Lead 审批？
[ ] 豁免是否有明确的复审日期？
```

---

## 三、门禁失败标准处置流程

### 3.1 四种处置结果

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
       │       → 在 gate_lifecycle_tracking.md §七登记
       │
       └── D. 不可抗力（外部依赖/基础设施）
               → 评估豁免可能性
               → 在 GATE_EXEMPTIONS.md 登记
               → 设置复审日期
```

---

## 四、治理规范与实际执行差距分析

### 4.1 差距识别

每次版本发布后，分析以下差距：

| 差距类型 | 识别方法 | 处置 |
|----------|----------|------|
| **规范缺失** | 脚本有检查项但规范未定义 | 补充规范，或移除脚本检查项 |
| **规范过时** | 规范定义与实际流程不符 | 更新规范或修正流程 |
| **执行缺失** | 规范有要求但未实现 | 实现或申请豁免 |
| **阈值偏差** | 规范阈值与实际测试结果不符 | 调整阈值或改进实现 |
| **文档不匹配** | 21 docs 要求与实际文档不符 | 补充或修正文档 |

### 4.2 自反馈记录

在 `gate_lifecycle_tracking.md` 中，每次版本发布后更新：

```markdown
## v{X}.{Y}.{Z} 周期回顾

### 规范 vs 执行差距
| 差距 | 类型 | 处置 |
|------|------|------|

### Issue 追踪统计
| 指标 | 数值 |
|------|------|
| 新建 Issue | N |
| 已关闭 | M |
| 延续到下版本 | K |
| 豁免登记 | J |

### 治理改进建议
1. {建议 1}
2. {建议 2}
```

---

## 五、治理文档自进化流程

### 5.1 进化触发条件

治理文档在以下情况必须更新：

| 触发条件 | 更新内容 | 责任人 |
|----------|----------|--------|
| 新增门禁检查项 | gate_spec_v130.md + check_*.sh | Agent |
| 新增文档要求 | VERSION_DOCS_SPEC.md + completeness_check.sh | Agent |
| 发现治理漏洞 | 本文档 + 相关治理文档 | Human Architect |
| 版本发布后 | gate_lifecycle_tracking.md §周期回顾 | Agent |
| 豁免被滥用 | GATE_EXEMPTIONS.md 豁免政策收紧 | Tech Lead |

### 5.2 进化审查清单

每次治理文档更新后，必须验证：

```
[ ] 新增检查项是否在 gate_spec_v130.md 定义？（SSOT 原则）
[ ] 新增规范是否在 check_*.sh 中实现？
[ ] 变更是否影响了已有 Issue 的追踪状态？
[ ] 是否需要更新 GATE_EXEMPTIONS.md？
[ ] 是否需要更新 DEVELOPMENT_PLAN.md §6 延续任务？
[ ] 是否在本次版本 gate_lifecycle_tracking.md 中记录？
```

### 5.3 文档版本管理

治理文档使用语义化版本：

```
governance/{doc}_v{Major}.{Minor}.md
```

| 变更类型 | 版本规则 | 示例 |
|----------|----------|------|
| 补充检查项 | Minor +1 | v1.0 → v1.1 |
| 修改阈值/定义 | Major +1 | v1.1 → v2.0 |
| 结构性重构 | Major +1 | v2.0 → v3.0 |

---

## 六、闭环追踪系统

### 6.1 Issue 标签强制

创建 Issue 时，**必须**打标签：

| 标签 | 触发条件 |
|------|----------|
| `source/gate-alpha` | Alpha 门禁失败 |
| `source/gate-beta` | Beta 门禁失败 |
| `source/gate-rc` | RC 门禁失败 |
| `source/gate-ga` | GA 门禁失败 |
| `carried/v1.4.0` | 延续到 v1.4.0 |

### 6.2 Issue 关闭验证流程

详见 `ISSUE_CLOSING_VERIFICATION.md`。

**核心规则**: 禁止在无 PR 证据时关闭 Issue。

---

## 七、门禁脚本自追踪检查

### 7.1 脚本健康检查

```bash
#!/bin/bash
# governance_health_check.sh — 治理健康检查
set -euo pipefail

echo "=== GMP-Platform 治理健康检查 ==="
echo ""

# 检查 1: 门禁脚本 REPO_ROOT 是否正确
echo "--- 检查 1: Gate 脚本路径 ---"
for script in scripts/gate/check_*.sh; do
    if grep -q '/home/openclaw' "$script" 2>/dev/null; then
        echo "  ⚠️  $script: 包含硬编码路径 /home/openclaw"
    else
        echo "  ✅ $script: 路径正确"
    fi
done

# 检查 2: Issue 标签一致性
echo ""
echo "--- 检查 2: Issue 标签规范 ---"
# ...

# 检查 3: GATE_EXEMPTIONS 复审日期
echo ""
echo "--- 检查 3: 豁免复审日期 ---"
# ...

# 检查 4: completeness_check.sh 覆盖 21 docs
echo ""
echo "--- 检查 4: 文档完整性检查 ---"
bash docs/completeness_check.sh 1.3.0
```

---

## 八、实际工作总结（v1.3.0 经验固化）

### 8.1 v1.3.0 实际执行记录

| 阶段 | 事件 | 执行结果 |
|------|------|----------|
| Beta 开发 | PR #138 文档合并 | ✅ 21-docs 完成 |
| Beta 开发 | PR #140 WAL + Binary BLOB | ✅ #122, #123 关闭 |
| Beta 开发 | 发现 Gate 脚本硬编码路径 | ✅ 修复：动态路径 |
| Beta 开发 | 发现层架构文档错误 | ✅ 5-layer 修复 |
| Beta 开发 | L0-Alpha Gate 自检 | ✅ 13/13 PASSED |
| Beta 开发 | 创建治理自进化文档 | ✅ 5个新文档 |
| Beta 开发 | 创建 ISSUE_CLOSING_VERIFICATION | ✅ Issue 关闭验证 |

### 8.2 规则进化项（基于 v1.3.0 经验）

从 v1.3.0 实际工作中抽取的规则：

**规则 1**: Gate 脚本必须使用动态 REPO_ROOT

```
旧规则: REPO_ROOT="/home/openclaw/..." 硬编码
新规则: REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
```

**规则 2**: Gate 脚本检查的二进制名称必须与实际一致

```
旧规则: 检查 gmp 或 gmp-cli
新规则: 检查 gmp-retrieve（实际二进制名称）
```

**规则 3**: 新增 governance 文档必须登记到 GOVERNANCE_STANDARD.md 自检清单

```
新增: gate_lifecycle_tracking.md → 自检清单 +1
新增: governance_self_improvement.md → 自检清单 +1
新增: ISSUE_CLOSING_VERIFICATION.md → 自检清单 +1
```

### 8.3 v1.3.0 健康检查结果

| 检查项 | 结果 |
|--------|------|
| REPO_ROOT 动态路径 | ✅ 已修复 (4/4 scripts) |
| Issue 关闭验证机制 | ✅ 已建立 |
| GATE_EXEMPTIONS 复审日期 | ✅ 已添加 |
| 门禁生命周期追踪 | ✅ 已建立 |
| 治理自进化机制 | ✅ 已建立 |
| v1.3.0 gate_spec | ✅ 已建立 |

### 8.4 待改进项

| # | 改进项 | 优先级 | 责任人 |
|---|--------|--------|--------|
| 1 | gmp-api crate 自测用例不足 | P1 | @liying |
| 2 | Layer 0 管线未实现 (#124) | P0 | @liying |
| 3 | gmp-server 健康检查接口未实现 | P2 | @liying |

**规则 2**: 文档层编号错误必须修复

```
旧规则: FEATURE_MATRIX.md 使用 4 层架构
新规则: FEATURE_MATRIX.md 使用 5 层架构（L0-L5）
```

**规则 3**: Issue 关闭前必须验证 PR 关联

```
旧规则: 可直接关闭 Issue
新规则: 必须通过 Gitea API 验证关联 PR 已合并
```

---

## 九、相关文档

| 文档 | 作用 |
|------|------|
| `gate_spec_v130.md` | 门禁定义 SSOT |
| `gate_lifecycle_tracking.md` | 门禁生命周期追踪 |
| `GATE_EXEMPTIONS.md` | 门禁豁免记录 |
| `ISSUE_CLOSING_VERIFICATION.md` | Issue 关闭验证流程 |
| `VERSION_DOCS_SPEC.md` | 版本文档规范 |

---

*本文档由 hermes agent 创建，基于 v1.3.0 实际工作经验总结*
*参考 SQLRustGo governance_self_improvement.md*
*最后更新: 2026-05-24*
