# SQLRustGo AI 协作规则

> **版本**: 1.0
> **更新日期**: 2026-03-07
> **维护人**: yinglichina8848

---

## 一、协作模式概述

### 1.1 协作角色

SQLRustGo 采用人机协作开发模式:

| 角色 | 职责 | 权限 |
|------|------|------|
| **Human Architect** | 架构设计, 重大决策 | 全部权限 |
| **AI Developer** | 代码实现, 测试编写 | 提交 PR |
| **AI Maintainer** | 代码审查, 问题诊断 | Review 权限 |
| **CI System** | 自动化检查, 质量门禁 | 检查执行 |

### 1.2 协作流程

```
Human Architect
      │
      ├── 分配任务 (Issue)
      │         │
      │         ▼
      │   AI Developer
      │         │
      │         ├── 编写代码
      │         ├── 编写测试
      │         └── 创建 PR
      │                │
      │                ▼
      │          AI Maintainer
      │                │
      │                ├── 代码审查
      │                └── 质量检查
      │                      │
      │                      ▼
      │                CI System
      │                      │
      │                      ├── 编译检查
      │                      ├── 测试检查
      │                      └── 安全检查
      │                            │
      │                            ▼
      │                      Human Architect
      │                            │
      │                            └── 最终审核
```

---

## 二、角色定义

### 2.1 Human Architect (人类架构师)

**职责**:

- 制定项目架构和技术方向
- 审批重大变更
- 管理版本发布
- 解决争议

**权限**:

- 合并 PR
- 管理分支
- 发布版本
- 管理团队成员

### 2.2 AI Developer (AI 开发者)

**职责**:

- 理解任务需求
- 实现功能代码
- 编写单元测试
- 修复 CI 失败

**工作约束**:

- 必须在 Human Architect 分配的 Issue 范围内工作
- 必须遵循代码规范
- 必须通过所有 CI 检查
- 必须有人工审查才能合并

### 2.3 AI Maintainer (AI 维护者)

**职责**:

- 代码质量审查
- Bug 诊断
- 性能分析
- 安全审计

**工作约束**:

- 可以提出修改建议
- 不能直接合并代码
- 重大问题需升级给 Human Architect

### 2.4 CI System (持续集成)

**职责**:

- 编译检查
- 测试执行
- 代码质量检查
- 安全扫描
- 发布构建

---

## 三、任务分配机制

### 3.1 Issue 管理

```
创建 Issue
      │
      ├── 指定类型 (Feature/Bug/Docs/Refactor)
      ├── 指定优先级 (P0/P1/P2/P3)
      ├── 指定版本 (v1.2.0/v1.3.0)
      └── 分配给 AI Developer
              │
              ▼
        AI Developer 认领
              │
              ▼
        开始开发
```

### 3.2 Issue 标签

| 标签 | 说明 |
|------|------|
| `feature` | 新功能 |
| `bug` | Bug 修复 |
| `docs` | 文档更新 |
| `refactor` | 重构 |
| `performance` | 性能优化 |
| `security` | 安全修复 |
| `P0` | 紧急 |
| `P1` | 高优先级 |
| `P2` | 中优先级 |
| `P3` | 低优先级 |

---

## 四、AI 开发规范

### 4.1 开发前准备

1. **理解需求**: 仔细阅读 Issue 描述
2. **查阅文档**: 了解相关模块的实现
3. **分析影响**: 评估对其他模块的影响
4. **制定方案**: 设计实现方案

### 4.2 代码编写规范

```rust
// 必须遵循的规范
- 使用 cargo fmt 格式化代码
- 使用 clippy 检查代码质量
- 添加必要的文档注释
- 编写单元测试
- 遵循项目的命名规范
```

### 4.3 提交规范

每次提交必须:

- 符合 conventional commits 规范
- 包含变更的清晰描述
- 关联对应的 Issue

### 4.4 PR 创建规范

PR 必须包含:

- 清晰的标题
- 变更描述
- 测试结果
- 相关 Issue 引用

---

## 五、AI 审查规范

### 5.1 审查维度

| 维度 | 检查项 |
|------|--------|
| **正确性** | 代码逻辑正确, 边界条件处理 |
| **可读性** | 命名清晰, 注释充分 |
| **可维护性** | 模块化, 单一职责 |
| **性能** | 时间和空间复杂度 |
| **安全性** | 无安全漏洞 |
| **测试** | 覆盖充分, 场景完整 |

### 5.2 审查输出格式

```
## 代码审查报告

### 总体评价
[通过/需要修改/需要重写]

### 问题列表
1. [问题描述] (严重程度: 高/中/低)
   - 位置: 文件:行号
   - 建议: [修改建议]

### 优点
- [列出代码的优点]

### 建议
- [可选的改进建议]
```

### 5.5 多 AI 协调规则
> **版本**: 1.1
> **更新日期**: 2026-06-26
> **关联**: ADR-014 (Multi-AI Coordination)
当多个 AI Agent（Hermes-Z6G4 / Claude-Code / 其他）在同一仓库协作时：
#### 5.5.1 物理隔离与共享资源协调
**Worktree 隔离**：
- 每个 AI Agent 在独立 worktree 中工作
- Worktree 命名规范：`.worktrees/{agent-id}-{date}`
- 避免多个 Agent 同时修改同一分支
**分支协调**：
- Agent 创建分支命名：`{agent-id}/feature-{name}`
- 避免多个 Agent 同时修改 `develop/v3.9.0` 直接提交
- 合并通过 PR 而非直接 push
**共享资源**：
- PAT (Personal Access Token) 不可跨 Agent 共享
- 每个 Agent 使用自己的认证凭据
- 配置在 `.claude/` 或环境变量中
#### 5.5.2 Issue 归属与 Claim 署名
**Issue 认领**：
```
[agent: hermes-z6g4] 认领 issue #3600
```
- 第一个认领的 Agent 获得归属
- 其他 Agent 在认领的 Issue 下工作前应 DM 协调
**Claim 格式**：
```
[agent: {agent_id}] {动作}结果 — YYYY-MM-DD HH:MM
```
- 所有评论必须带 `[agent: {agent_id}]` 前缀
- 无前缀视为 User（人类）发言
#### 5.5.3 标签系统
|标签|用途|适用 Agent|
|---|---|---|
|`agent:hermes`|Hermes Agent 创建/认领|hermes-z6g4|
|`agent:claude`|Claude Code 创建/认领|claude-code|
|`multi-ai-verify`|多 AI 协作验证 Issue|任意|
|`governance-audit`|治理审计 Issue|任意|
#### 5.5.4 调度规则
**Gate FAIL 时的调度**：
- 发现 Gate FAIL 的 Agent 负责主导修复
- 其他 Agent 等待修复完成后再并发
- 避免多个 Agent 同时修复同一 Gate
**冲突解决**：
- 同一文件被多个 Agent 并发修改 → 以先 merge 到目标分支的为准
- Issue 下的验证结果冲突 → 要求各 Agent 提供独立实跑证据
- User（李哥）拥有最终裁决权
**Gate 推送约束**（ADR-001 G-09）：
- Gate FAIL 状态下禁止 push 到 252 Gitea
- 违反者标记为 `[AFP-VIOLATION: Type-B]`
---

## 六、不可绕过的 Release Gate

### 6.1 Release Gate 定义

Release Gate 是代码合并到主分支前必须通过的质量检查点。**AI 不可绕过**。

### 6.2 Gate 检查项

| Gate | 检查项 | 执行者 |
|------|--------|--------|
| **Compile Gate** | 编译通过 | CI |
| **Test Gate** | 单元测试通过 | CI |
| **Lint Gate** | 代码规范通过 | CI |
| **Security Gate** | 安全扫描通过 | CI |
| **Review Gate** | 人工审查通过 | Human |
| **Arch Gate** | 架构审查通过 | Human |

### 6.3 门禁失败处理

```
门禁失败
      │
      ├── 编译失败
      │   └── AI Developer 修复
      │
      ├── 测试失败
      │   └── AI Developer 修复 + 分析原因
      │
      ├── 代码规范失败
      │   └── AI Developer 自动修复
      │
      ├── 安全扫描失败
      │   └── Human Architect 评估
      │
      └── 审查失败
          └── 按审查意见修改
```

---

## 七、责任与追责

### 7.1 AI 责任边界

AI 开发者需要对其生成的代码负责:

- 代码正确性
- 测试覆盖率
- 文档更新

### 7.2 人工审核责任

Human Architect 对以下事项负责:

- 最终合并决策
- 版本发布
- 安全合规

### 7.3 问题追溯

所有变更必须可追溯:

- Commit 关联 Issue
- PR 关联 Commit
- Release 关联 Tag

---

## 八、协作工具

### 8.1 沟通渠道

| 渠道 | 用途 |
|------|------|
| GitHub Issues | 任务分配, 问题跟踪 |
| GitHub PRs | 代码审查, 变更讨论 |
| GitHub Discussions | 技术讨论 |

### 8.2 自动化工具

| 工具 | 用途 |
|------|------|
| GitHub Actions | CI/CD |
| Cargo | 编译, 测试 |
| Clippy | 代码检查 |
| Cargo Audit | 安全扫描 |

---

## 九、版本兼容性

### 9.1 API 变更

所有 API 变更必须:

- 通过 Human Architect 审批
- 记录在 CHANGELOG
- 更新版本号

### 9.2 破坏性变更

破坏性变更 (Breaking Changes) 必须:

- 在 MAJOR 版本发布
- 提供迁移指南
- 提前通知用户

---

## 十、相关文档

| 文档 | 说明 |
|------|------|
| [BRANCH_GOVERNANCE.md](../BRANCH_GOVERNANCE.md) | 分支治理 |
| [RELEASE_LIFECYCLE.md](./RELEASE_LIFECYCLE.md) | 版本生命周期 |
| [RELEASE_POLICY.md](./RELEASE_POLICY.md) | 发布策略 |
| 贡献指南 | (待创建) |

---

## 十一、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-07 | 初始版本 |

---

*本文档由 yinglichina8848 维护*
