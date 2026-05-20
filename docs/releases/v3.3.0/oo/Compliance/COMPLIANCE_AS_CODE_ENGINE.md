# OO-CE1: Compliance-as-Code Engine

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1238
> **状态**: 新建设计

---

## 一、概述

### 1.1 目标

将 GMP 规则表达为可执行代码，实现合规检查的自动化和版本化管理。

### 1.2 核心理念

```
Compliance-as-Code = Rule DSL + Versioned Rules + Automated Enforcement + Audit Trail
```

### 1.3 支持标准

| 标准 | 说明 |
|------|------|
| ALCOA+ | 数据完整性原则 |
| 21 CFR Part 11 | FDA 电子记录/签名法规 |
| EU GMP Annex 11 | 欧盟 GMP 计算机化系统 |
| ISO 27001 | 信息安全管理 |
| HIPAA | 医疗数据保护 |

---

## 二、架构设计

### 2.1 组件关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   Compliance-as-Code Engine                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────┐   │
│  │  Rule DSL   │───▶│   Compiler  │───▶│   Rule Engine               │   │
│  │  (YAML)    │    │             │    │  (evaluation + enforcement) │   │
│  └─────────────┘    └─────────────┘    └─────────────────────────────┘   │
│                                                        │                     │
│  ┌─────────────┐    ┌─────────────┐                   ▼                     │
│  │ Built-in    │───▶│  Rule       │◀────────  Violation Handler           │
│  │  Rules     │    │  Registry   │                                          │
│  └─────────────┘    └─────────────┘    ┌─────────────────────────────┐   │
│                                         │   Audit Trail              │   │
│                                         │   (immutable log)          │   │
│                                         └─────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件说明

| 组件 | 说明 | 位置 |
|------|------|------|
| Rule DSL | YAML 规则定义语言 | `crates/compliance-dsl/` |
| Compiler | DSL → AST → IR | `crates/compliance-compiler/` |
| Rule Engine | 规则评估和执行 | `crates/compliance-engine/` |
| Built-in Rules | 内置 GMP 规则库 | `rules/gmp/` |
| Audit Trail | 违规记录 | `compliance/audit/` |

---

## 三、规则 DSL

### 3.1 规则结构

```yaml
rule:
  metadata:
    id: GMP-001
    name: dual_signature_for_batch_release
    version: 1.0
    description: >
      Batch release requires dual signatures from
      Quality Reviewer and Production Manager
    tags:
      - batch_release
      - dual_control
      - gmp_critical

  applies_to:
    entity: batch_release
    scope:
      - facility: all
        product_categories: [drug, biologic, device]

  conditions:
    and:
      - signatures.count >= 2
      - signatures.has_role(quality_reviewer)
      - signatures.has_role(production_manager)
      - workflow.state == approved
      - timestamp.within_business_hours OR
        timestamp.is_emergency_release

  enforcement:
    on_violation:
      - block_transaction
      - generate_deviation(deviation_type: missing_signature)
      - write_audit_log(
          action: BLOCKED,
          reason: DUAL_SIGNATURE_REQUIRED,
          required_roles: [quality_reviewer, production_manager],
          actual_signatures: signatures.count
        )
      - notify(
          recipients: [qa_director, production_manager],
          channel: email,
          urgency: high
        )

  evidence:
    - signature_proof
    - workflow_proof
    - timestamp_proof
    - approval_chain_proof

  version_history:
    - version: 1.0
      date: 2026-01-15
      changed_by: compliance_team
      reason: Initial release
```

### 3.2 条件表达式

```yaml
# 简单条件
conditions:
  user.role == quality_reviewer

# 复合条件
conditions:
  and:
    - user.role == quality_reviewer
    - user.certification.valid == true
    - user.certification.expiry > now()

# OR 条件
conditions:
  or:
    - workflow.state == approved
    - emergency_release.approved == true

# NOT 条件
conditions:
  not:
    user.is_suspended == true

# 数值范围
conditions:
  signatures.count >= 2
  batch.quantity >= 100 AND batch.quantity <= 10000

# 时间条件
conditions:
  timestamp.within_business_hours == true
  timestamp.day_of_week in [MON, TUE, WED, THU, FRI]
```

### 3.3 内置规则库

```
rules/gmp/
├── ALCOA/
│   ├── legible.yaml          # 数据可读性
│   ├── attributable.yaml     # 数据可归属
│   ├── contemporaneous.yaml  # 数据同步
│   ├── original.yaml         # 数据原始性
│   └── accurate.yaml         # 数据准确性
├── CFR_Part_11/
│   ├── electronic_signature.yaml
│   ├── audit_trail.yaml
│   ├── system_validation.yaml
│   └── record_retention.yaml
├── EU_Annex_11/
│   ├── risk_management.yaml
│   ├── computerised_systems.yaml
│   └── data_integrity.yaml
└── custom/
    ├── batch_release.yaml
    ├── deviation_management.yaml
    └── change_control.yaml
```

---

## 四、执行流程

### 4.1 规则评估流程

```
┌─────────────────────────────────────────────────────────────────┐
│                   Rule Evaluation Flow                            │
└─────────────────────────────────────────────────────────────────┘

Transaction enters system
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Rule Matching                                                 │
│     - Determine entity type (batch_release, signature, etc.)     │
│     - Query rule registry for applicable rules                   │
│     - Filter by scope (facility, product_category, etc.)        │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Condition Evaluation                                          │
│     - Load context (user, timestamp, workflow state, signatures)  │
│     - Compile conditions to executable predicates                │
│     - Evaluate in order (AND/OR/NOT)                             │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Enforcement Action                                           │
│     - If PASS: allow transaction, log approval                    │
│     - If FAIL: execute enforcement actions                        │
│       - block_transaction: abort with error                     │
│       - generate_deviation: create deviation record              │
│       - write_audit_log: immutable audit entry                  │
│       - notify: send alerts                                      │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Evidence Collection                                           │
│     - Gather evidence items (signatures, workflow, etc.)         │
│     - Generate hash proofs                                       │
│     - Attach to audit trail                                      │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Violation 处理流程

```
┌─────────────────────────────────────────────────────────────────┐
│                 Violation Handling Flow                           │
└─────────────────────────────────────────────────────────────────┘

Condition evaluates to FALSE
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Block Transaction                                            │
│     - Return error to caller                                     │
│     - Include violation details                                  │
│     - HTTP 403 / Database rollback                               │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Generate Deviation                                           │
│     - Create deviation record                                     │
│     - Link to blocked transaction                                │
│     - Assign deviation ID                                         │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Write Audit Log                                              │
│     - Immutable append                                           │
│     - Include: timestamp, user, rule, context, action            │
│     - Generate cryptographic hash                               │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Notify Stakeholders                                          │
│     - Email to responsible persons                               │
│     - Dashboard alert                                            │
│     - Slack notification (optional)                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 五、数据结构

### 5.1 规则存储

```rust
// Rule storage in compliance engine
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    pub version: Version,
    pub entity_type: EntityType,
    pub scope: Scope,
    pub conditions: ConditionExpression,
    pub enforcement: EnforcementActions,
    pub evidence: Vec<EvidenceType>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum EntityType {
    BatchRelease,
    Signature,
    WorkflowTransition,
    ChangeControl,
    Deviation,
    Calibration,
    Maintenance,
}

pub struct Scope {
    pub facilities: Vec<FacilityId>,
    pub product_categories: Vec<ProductCategory>,
    pub applicability: ApplicabilityRule,
}
```

### 5.2 违规记录

```rust
pub struct ViolationRecord {
    pub id: ViolationId,
    pub rule_id: RuleId,
    pub transaction_id: TransactionId,
    pub context: HashMap<String, Value>,
    pub evaluated_conditions: Vec<ConditionResult>,
    pub enforcement_actions_taken: Vec<EnforcementAction>,
    pub timestamp: DateTime<Utc>,
    pub resolution: Option<Resolution>,
}

pub struct AuditLogEntry {
    pub id: AuditEntryId,
    pub violation_id: Option<ViolationId>,
    pub action: AuditAction,
    pub actor: ActorId,
    pub details: Value,
    pub hash: Sha256Hash,
    pub previous_hash: Sha256Hash,  // Chain integrity
}
```

---

## 六、验收标准

### 6.1 功能验收

| 检查项 | 命令 | 标准 |
|--------|------|------|
| DSL 解析 | `compliance-dsl parse rule.yaml` | 成功解析 |
| 规则编译 | `compliance-compiler build rule.yaml` | 生成 IR |
| 条件评估 | `cargo test condition_evaluation` | 正确求值 |
| 违规阻止 | `cargo test violation_blocks_transaction` | 事务被阻止 |
| 审计记录 | `cargo test audit_log_immutable` | 记录不可变 |

### 6.2 合规标准覆盖

| 标准 | 规则数量 | 覆盖率 |
|------|---------|--------|
| ALCOA+ | 5 | 100% |
| 21 CFR Part 11 | 8 | 100% |
| EU GMP Annex 11 | 6 | 100% |
| Custom GMP | 15+ | 可扩展 |

---

## 七、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `crates/compliance-dsl/` - 规则 DSL 实现
- `crates/compliance-engine/` - 规则引擎实现
- `rules/gmp/` - 内置 GMP 规则库

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
