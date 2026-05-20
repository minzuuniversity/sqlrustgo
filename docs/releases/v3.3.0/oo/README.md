# OO Documents - v3.3.0 Trust Infrastructure

> **版本**: v1.0
> **日期**: 2026-05-18
> **分支**: `develop/v3.3.0`

---

## 一、目录结构

```
oo/
├── OO_ROADMAP.md                    # OO 文档路线图
├── README.md                        # 本文件
├── Trust-Infrastructure/            # Trust Kernel (P0)
│   ├── PERFORMANCE_GOVERNANCE.md    # 性能治理系统
│   ├── CRASH_SIMULATION_FRAMEWORK.md # 崩溃模拟框架
│   └── WAL_FORMAL_VERIFICATION.md   # WAL 形式化验证
├── Compliance/                      # 合规自动化 (P0)
│   ├── COMPLIANCE_AS_CODE_ENGINE.md # 合规即代码引擎
│   ├── EVIDENCE_ENGINE.md          # 证据引擎
│   └── PROVENANCE_KNOWLEDGE_GRAPH.md # 血缘知识图
└── GMP-Management/                  # GMP 管理能力 (P1)
    ├── WORKFLOW_V2.md              # 工作流引擎 V2
    └── TRUST_VISUALIZATION.md      # 可信可视化
```

## 二、文档映射

| Issue | 功能 | 文档 | 优先级 |
|-------|------|------|--------|
| #1235 | Performance Governance System | PERFORMANCE_GOVERNANCE.md | P0 |
| #1236 | Crash Simulation Framework | CRASH_SIMULATION_FRAMEWORK.md | P0 |
| #1237 | WAL Formal Verification | WAL_FORMAL_VERIFICATION.md | P0 |
| #1238 | Compliance-as-Code Engine | COMPLIANCE_AS_CODE_ENGINE.md | P0 |
| #1239 | Evidence Engine | EVIDENCE_ENGINE.md | P0 |
| #1240 | Provenance Knowledge Graph | PROVENANCE_KNOWLEDGE_GRAPH.md | P0 |
| #1241 | Workflow V2 | WORKFLOW_V2.md | P1 |
| #1242 | Trust Visualization | TRUST_VISUALIZATION.md | P1 |

## 三、快速导航

### Trust Kernel
- [性能治理系统](./Trust-Infrastructure/PERFORMANCE_GOVERNANCE.md) - PR 级性能门禁、基线数据库、Flamegraph
- [崩溃模拟框架](./Trust-Infrastructure/CRASH_SIMULATION_FRAMEWORK.md) - 混沌注入、恢复验证、TLA+ 引导
- [WAL 形式化验证](./Trust-Infrastructure/WAL_FORMAL_VERIFICATION.md) - TLA+ 状态机、Crash Recovery 正确性

### Compliance
- [合规即代码引擎](./Compliance/COMPLIANCE_AS_CODE_ENGINE.md) - 规则 DSL、ALCOA+/21 CFR Part 11/EU GMP
- [证据引擎](./Compliance/EVIDENCE_ENGINE.md) - FDA 审计包、JSON/PDF/ZIP 导出、链验证
- [血缘知识图](./Compliance/PROVENANCE_KNOWLEDGE_GRAPH.md) - Graph DB、Cypher 查询、血缘推导

### GMP Management
- [工作流 V2](./GMP-Management/WORKFLOW_V2.md) - 并行分支、条件路由、子流程、多人会签
- [可信可视化](./GMP-Management/TRUST_VISUALIZATION.md) - 审计链图、合规评分、风险热图

## 四、相关文档

- [开发计划](../DEV_PLAN.md) - v3.3.0 完整开发计划
- [门禁规范](../../governance/gate_spec_v330.md) - Alpha/Beta/GA Gate 要求
- [Trust Infrastructure 策略](../TRUST_INFRASTRUCTURE_STRATEGY.md) - 战略定位

---

*本文档由 hermes-agent 生成*
