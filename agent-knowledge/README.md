# Agent 数字知识目录

> 基于 chatgpt-ai-isolation-architecture.md 四层隔离体系
> 创建日期: 2026-05-20
> 状态: 初始构建

---

## 目录结构

```
agent-knowledge/
├── README.md                          ← 本文件
├── CLASSIFICATION-INDEX.md            ← 文档分类索引（含 gmp_complete_archive 补充）
├── MINIMAL-LOOP-PLAN.md              ← 最小闭环系统开发和验证计划
│
├── L0-trust-zone/                     ← Layer 0: 绝对可信区（禁止出云）
│   ├── entities/                      ← GMP 实体定义
│   │   ├── gmp-core-entities.yaml     ← 核心实体（物料/设备/岗位/工序）
│   │   └── validation-change-entities.yaml ← 验证/变更/风险评估实体
│   ├── rules/                         ← 判定规则库（55+条规则）
│   │   ├── weighing-rules.yaml        ← 称量规则 (7条)
│   │   ├── deviation-capa-rules-enhanced.yaml ← 偏差/CAPA增强规则
│   │   ├── change-control-rules.yaml  ← 变更控制规则
│   │   ├── validation-rules.yaml      ← 验证与确认规则
│   │   ├── production-rules.yaml      ← 生产管理规则
│   │   ├── qms-rules.yaml            ← 质量管理体系规则
│   │   ├── data-integrity-doc-rules.yaml ← 数据完整性/文件管理规则
│   │   ├── material-warehouse-rules.yaml ← 物料与仓储管理规则
│   │   └── facility-equipment-rules.yaml ← 设施与设备管理规则
│   ├── workflows/                     ← 流程模型（DAG 定义）
│   │   └── weighing-workflow.yaml     ← 称量操作流程
│   └── evidence-templates/            ← 证据模板（记录表单结构）
│
├── L1-sanitized-zone/                 ← Layer 1: 脱敏 AI 区（Schema 保留，值删除）
│   ├── schemas/                       ← 数据库 Schema（脱敏）
│   │   └── gmp-clauses-index.yaml     ← GMP法规条款索引（53章条款解释）
│   ├── mock-data/                     ← 合成/模拟数据
│   └── synthetic-rules/               ← 脱敏后的规则（不含真实参数值）
│
├── L2-cloud-zone/                     ← Layer 2: 云推理区（仅抽象 DSL）
│   ├── dsl-templates/                 ← Agent 可生成的 DSL 模板
│   └── agent-prompts/                 ← Agent 系统 Prompt 模板
│
├── L3-validation-zone/                ← Layer 3: 执行验证区
│   ├── validators/                    ← 本地验证器规则
│   └── test-scenarios/                ← 测试场景定义
│       └── weighing-test-scenarios.yaml ← 称量测试场景 (10个)
│
├── repo-core-open/                    ← 仓库 A: Core Open Source（可出云）
│   ├── framework/                     ← 通用框架代码
│   └── generic-agent/                 ← 通用 Agent 框架
│
└── repo-private-gmp/                  ← 仓库 B: Private GMP Layer（禁止出云）
    ├── sop-abstracts/                 ← SOP 摘要（结构化提取）
    └── formula-templates/             ← 配方/工艺模板
```

## 隔离原则速查

| 层级 | 允许出云 | 模型要求 | 数据形态 |
|------|----------|----------|----------|
| L0 Trust Zone | 禁止 | 本地 LLM ONLY | 原始数据 + 完整规则 |
| L1 Sanitized Zone | 可（脱敏后） | 本地/云端 | Schema + Mock 数据 |
| L2 Cloud Zone | 可 | 云端 LLM | 抽象 DSL + Prompt |
| L3 Validation Zone | 禁止 | 本地 | 验证规则 + 测试场景 |

## 数据流向

```
L0 (原始数据/规则) ──脱敏──→ L1 (Schema/Mock) ──抽象──→ L2 (DSL/Prompt)
                                                            │
                                                            ▼
L0 (原始数据) ←──验证── L3 (Validator) ←──建议── L2 (云模型输出)
```
