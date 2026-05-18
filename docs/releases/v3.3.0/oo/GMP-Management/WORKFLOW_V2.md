# OO-GM1: Workflow V2

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1241
> **状态**: 新建设计

---

## 一、概述

### 1.1 目标

实现企业级工作流引擎，支持并行分支、条件分支、子流程、多人审批等复杂场景。

### 1.2 核心理念

```
Workflow V2 = State Machine + Parallel Execution + Conditional Routing + Timeout Handling
```

### 1.3 v1 vs v2 对比

| 特性 | v1 | v2 |
|------|----|----|
| 串行审批 | ✅ | ✅ |
| 并行分支 | ❌ | ✅ |
| 条件分支 | ❌ | ✅ |
| 子流程 | ❌ | ✅ |
| 多人审批 | 基础 | 高级 (会签/或签) |
| 超时处理 | ❌ | ✅ |
| 委托/转交 | ❌ | ✅ |

---

## 二、架构设计

### 2.1 组件关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Workflow V2 Engine                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────────────────────┐  │
│  │  Workflow   │───▶│   Engine    │───▶│   Execution Context         │  │
│  │  Definition │    │   Core      │    │   Manager                   │  │
│  └──────────────┘    └──────────────┘    └────────────────────────────┘  │
│                             │                                           │
│                             ▼                                           │
│  ┌──────────────┐    ┌──────────────┐    ┌────────────────────────────┐  │
│  │  State      │    │   Branch    │    │   Timeout                  │  │
│  │  Machine    │    │   Router    │    │   Manager                   │  │
│  └──────────────┘    └──────────────┘    └────────────────────────────┘  │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    Notification Service                                │  │
│  │  - Email    - Slack    - Dashboard    - SMS (optional)              │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件说明

| 组件 | 说明 | 位置 |
|------|------|------|
| Workflow Definition | 工作流定义 DSL | `crates/workflow-def/` |
| Engine Core | 执行引擎 | `crates/workflow-engine/` |
| State Machine | 状态机管理 | `crates/workflow-state/` |
| Branch Router | 分支路由 | `crates/workflow-router/` |
| Timeout Manager | 超时处理 | `crates/workflow-timeout/` |
| Notification Service | 通知服务 | `crates/workflow-notify/` |

---

## 三、工作流定义

### 3.1 DSL 结构

```yaml
workflow:
  id: batch_release_v2
  name: Batch Release Workflow V2
  version: 2.0
  description: >
    Multi-stage batch release with parallel quality checks
    and conditional release criteria

start_state: initial_review

states:
  initial_review:
    type: task
    assigned_to: quality_reviewer
    timeout: 24h
    on_complete: parallel_quality_checks
    on_timeout: escalate_to_qa_director

  parallel_quality_checks:
    type: parallel
    branches:
      - id: chemical_analysis
        assigned_to: lab_tech_1
        timeout: 48h
        on_complete: join
        on_timeout: escalate_to_qa_director
      - id: physical_testing
        assigned_to: lab_tech_2
        timeout: 48h
        on_timeout: escalate_to_qa_director
      - id: documentation_review
        assigned_to: documentation_specialist
        timeout: 24h
        on_timeout: escalate_to_qa_director
    join:
      type: join
      strategy: all  # all | any | n
      require: [chemical_analysis, physical_testing, documentation_review]

  quality_approval:
    type: approval
    assigned_to:
      - quality_manager
      - production_manager
    strategy: all  # all (会签) | any (或签)
    timeout: 24h
    require_signatures: 2
    on_approve: final_release
    on_reject: deviation_created
    on_timeout: escalate_to_qa_director

  final_release:
    type: task
    assigned_to: batch_release_officer
    timeout: 4h
    on_complete: completed
    on_timeout: auto_escalate

  deviation_created:
    type: subprocess
    workflow_id: deviation_management
    input_mapping:
      source_batch: batch_id
      deviation_type: quality_rejection
    on_complete: quality_approval

  completed:
    type: end
    final: true
```

### 3.2 状态类型

```yaml
# 状态类型说明

task:
  description: 单人任务
  fields:
    - assigned_to: UserId | RoleId
    - timeout: Duration
    - instructions: String

approval:
  description: 审批（可多会签/或签）
  fields:
    - assigned_to: [UserId | RoleId]
    - strategy: all | any | n
    - require_signatures: N
    - timeout: Duration

parallel:
  description: 并行分支
  fields:
    - branches: [Branch]
    - join:
        type: join
        strategy: all | any | n

conditional:
  description: 条件分支
  fields:
    - conditions: [Condition]
    - default: StateId

subprocess:
  description: 子流程调用
  fields:
    - workflow_id: WorkflowId
    - input_mapping: Map
    - output_mapping: Map

join:
  description: 并行分支汇合
  fields:
    - strategy: all | any | n
    - require: [BranchId]

end:
  description: 结束状态
  fields:
    - final: true
```

---

## 四、执行模型

### 4.1 并行分支执行

```
┌─────────────────────────────────────────────────────────────────┐
│              Parallel Branch Execution Model                      │
└─────────────────────────────────────────────────────────────────┘

State: parallel_quality_checks
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Fork into N branches                                           │
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │  Chemical   │  │  Physical   │  │  Document   │           │
│  │  Analysis   │  │  Testing    │  │  Review     │           │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘           │
│         │                 │                 │                    │
│         ▼                 ▼                 ▼                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │ Branch A    │  │ Branch B    │  │ Branch C    │           │
│  │ Complete?   │  │ Complete?   │  │ Complete?   │           │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘           │
│         │                 │                 │                    │
│         └─────────────────┼─────────────────┘                    │
│                           │                                      │
│                           ▼                                      │
│                  ┌─────────────────┐                            │
│                  │   Join Gate     │                            │
│                  │   (strategy)    │                            │
│                  └────────┬─────────┘                            │
│                           │                                      │
│              ┌───────────┴───────────┐                          │
│              │                       │                          │
│            ALL                      ANY                          │
│           Complete?               Complete?                      │
│              │                       │                          │
│              ▼                       ▼                          │
│      ┌─────────────┐          ┌─────────────┐                 │
│      │ Proceed to  │          │ Proceed to  │                 │
│      │ next state  │          │ next state  │                 │
│      └─────────────┘          └─────────────┘                 │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 条件分支执行

```
┌─────────────────────────────────────────────────────────────────┐
│              Conditional Branch Execution Model                    │
└─────────────────────────────────────────────────────────────────┘

State: route_decision
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Evaluate conditions in order                                    │
│                                                                 │
│  Condition 1: batch.quality_score >= 95                         │
│    YES → high_quality_release                                    │
│    NO  ↓                                                         │
│                                                                 │
│  Condition 2: batch.quality_score >= 85                         │
│    YES → standard_release                                        │
│    NO  ↓                                                         │
│                                                                 │
│  Condition 3: batch.has_deviation == false                      │
│    YES → conditional_release                                    │
│    NO  ↓                                                         │
│                                                                 │
│  Default: deviation_required                                     │
└─────────────────────────────────────────────────────────────────┘
```

### 4.3 子流程调用

```
┌─────────────────────────────────────────────────────────────────┐
│                 Subprocess Execution Model                       │
└─────────────────────────────────────────────────────────────────┘

Parent Workflow                           Child Workflow
─────────────────                         ─────────────────
                                          ┌─────────────────┐
deviation_created                         │  Deviation      │
      │                                   │  Management     │
      │ call subprocess                   │                 │
      ├─────────────────────────────────▶│                 │
      │                                   │  - Investigate │──▶
      │                                   │  - CAPA        │──▶
      │                                   │  - Close       │──▶
      │                                   └────────┬────────┘
      │                                          │ return
      │◀──────────────────────────────────────────┘
      │   resume with output
      │
      ▼
quality_approval
```

---

## 五、超时处理

### 5.1 超时策略

```yaml
timeout_handling:
  escalate_to: qa_director
  escalation_chain:
    - role: qa_manager
      delay: 1h
    - role: qa_director
      delay: 2h
  actions:
    - send_notification
    - create_timeout_record
    - log_audit_event
  auto_action:  # 无响应时的自动操作
    type: auto_approve_with_notation
    require: 1 additional_approval
```

### 5.2 委托/转交

```yaml
# 委托机制
delegation:
  allowed: true
  max_depth: 2
  requires_approval: false  # true for high-value workflows

# 转交机制
transfer:
  allowed: true
  original_assignee: notified
  audit_trail: full
```

---

## 六、数据结构

### 6.1 工作流实例

```rust
pub struct WorkflowInstance {
    pub id: InstanceId,
    pub workflow_id: WorkflowId,
    pub status: InstanceStatus,  // running, paused, completed, cancelled
    pub current_states: Vec<ActiveState>,
    pub variables: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ActiveState {
    pub state_id: StateId,
    pub branch_id: Option<BranchId>,  // for parallel branches
    pub assigned_to: Vec<ActorId>,
    pub entered_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub context: StateContext,
}

pub struct StateContext {
    pub task_id: Option<TaskId>,
    pub approval_signatures: Vec<Signature>,
    pub parallel_progress: HashMap<BranchId, BranchStatus>,
    pub subprocess_instances: Vec<InstanceId>,
}
```

---

## 七、验收标准

### 7.1 功能验收

| 检查项 | 命令 | 标准 |
|--------|------|------|
| 串行审批 | `cargo test workflow_serial_approval` | 通过 |
| 并行分支 | `cargo test workflow_parallel_branches` | 所有分支正确执行 |
| 条件分支 | `cargo test workflow_conditional_routing` | 条件正确路由 |
| 子流程 | `cargo test workflow_subprocess` | 正确调用和返回 |
| 会签/或签 | `cargo test workflow_multi_signature` | 签名策略正确 |
| 超时处理 | `cargo test workflow_timeout` | 超时触发正确动作 |
| 委托/转交 | `cargo test workflow_delegation` | 委托链正确 |

### 7.2 性能验收

| 指标 | 标准 |
|------|------|
| 并发实例数 | > 10,000 |
| 状态转换延迟 (P99) | < 10ms |
| 超时检查频率 | 每分钟 |

---

## 八、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `crates/workflow-def/` - 工作流定义
- `crates/workflow-engine/` - 执行引擎
- `crates/workflow-state/` - 状态机

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
