# SQLRustGo 文档索引

> **最后更新**: 2026-05-29
> **当前版本**: v3.6.0 (开发中)
> **状态**: 文档体系改进中

---

## 一、文档目录结构

```
docs/
├── api/                       # API 文档（新增）
│   ├── README.md              # API 文档入口
│   ├── openapi.yaml           # OpenAPI 规范（规划中）
│   └── rustdoc/               # rustdoc HTML 文档
│
├── oo/                        # OO 文档统一索引（新增）
│   ├── README.md              # OO 文档索引
│   └── releases/              # 各版本 OO 文档
│
├── releases/                  # 版本发布文档
│   ├── v3.6.0/                # v3.6.0（当前开发版本）
│   ├── v3.5.0/                # v3.5.0
│   ├── v3.4.0/                # v3.4.0 (GA 2026-05-28)
│   └── ...
│
├── governance/                # 治理文档
│   ├── GOVERNANCE_INDEX.md    # 治理文档索引
│   ├── GATE_SPEC_MASTER.md    # 门禁规范 (A/B/R/G)
│   └── GATE_CI_CD.md          # CI/CD 流水线
│
├── architecture/               # 架构文档
├── tutorials/                 # 教程
├── gmp/                       # GMP 合规文档
├── gmp-compliance/            # GMP 合规评估
├── proof/                     # 形式化验证
└── ...
```

---

## 二、新增文档（v3.6.0）

### API 文档

| 文档 | 说明 |
|------|------|
| [API 文档入口](api/) | rustdoc + OpenAPI 规范 |
| [OO 文档索引](oo/) | 统一 OO 文档导航（145 个文档） |

**生成 rustdoc**:
```bash
cargo doc --all-features --no-deps
cargo doc --open  # 浏览器查看
```

### OO 文档覆盖

| 版本范围 | 文档数 | 主要内容 |
|----------|--------|----------|
| v2.0 | 1 | Vector Engine + Cascades |
| v2.6-v2.9 | 57 | SQL-92/Formal Verification |
| v3.0 | 24 | 完整 SQL 执行链路 |
| v3.1 | 10 | MERGE/GAP Lock/聚簇索引 |
| v3.2 | 13 | GMP OO 文档 |
| v3.3 | 9 | Trust Infrastructure |
| v3.4 | 1 | GMP Management Suite |
| v3.5 | 9 | AI Native GMP Platform |

---

## 三、版本发布文档

### v3.6.0（当前版本：开发中）

| 文档 | 说明 |
|------|------|
| [开发计划](releases/v3.6.0/DEVELOPMENT_PLAN.md) | v3.6.0 开发计划 |
| [测试计划](releases/v3.6.0/TEST_PLAN.md) | 测试目标与阶段安排 |
| [发布说明](releases/v3.6.0/RELEASE_NOTES.md) | 版本发布说明（规划中） |

### v3.5.0

| 文档 | 说明 |
|------|------|
| [综合说明](releases/v3.5.0/README.md) | v3.5.0 完整文档 |
| [发布说明](releases/v3.5.0/RELEASE_NOTES.md) | 版本发布说明 |
| [变更日志](releases/v3.5.0/CHANGELOG.md) | 详细变更记录 |
| [开发计划](releases/v3.5.0/DEVELOPMENT_PLAN.md) | 开发计划 |
| [测试计划](releases/v3.5.0/TEST_PLAN.md) | 测试计划 |
| [OO 文档](releases/v3.5.0/oo/) | AI Native GMP 文档 |

### v3.4.0（GA 2026-05-28）

| 文档 | 说明 |
|------|------|
| [综合说明](releases/v3.4.0/README.md) | v3.4.0 完整文档 |
| [发布说明](releases/v3.4.0/RELEASE_NOTES.md) | 版本发布说明 |
| [变更日志](releases/v3.4.0/CHANGELOG.md) | 详细变更记录 |
| [Alpha 门禁](releases/v3.4.0/ALPHA_GATE_REPORT.md) | Alpha Gate 结果 |
| [Beta 门禁](releases/v3.4.0/BETA_GATE_REPORT.md) | Beta Gate 结果 |
| [RC 门禁](releases/v3.4.0/RC_GATE_REPORT.md) | RC Gate 结果 |
| [GA 门禁](releases/v3.4.0/GA_GATE_REPORT.md) | GA Gate 结果 |

---

## 四、治理文档

| 文档 | 说明 |
|------|------|
| [治理文档索引](governance/GOVERNANCE_INDEX.md) | 治理文档导航 |
| [门禁规范](governance/GATE_SPEC_MASTER.md) | A/B/R/GA 四级门禁规格 |
| [CI/CD 流水线](governance/GATE_CI_CD.md) | 门禁 CI/CD 实现 |
| [Governance Standard](governance/GOVERNANCE_STANDARD.md) | 治理标准 |

---

## 五、形式化验证

| 文档 | 说明 |
|------|------|
| [Proof 注册表索引](proof/REGISTRY_INDEX.md) | 所有 Proof 文件索引 |
| [工具链安装](proof/TOOLCHAIN_INSTALLATION.md) | TLA+ / Formulog 安装 |

---

## 六、快速链接

| 资源 | 位置 |
|------|------|
| 项目主 README | [README.md](../README.md) |
| 门禁脚本 | [`../scripts/gate/`](scripts/gate/) |
| 文档改进计划 | [DOC_IMPROVEMENT_PLAN.md](../DOC_IMPROVEMENT_PLAN.md) |
| OSS 分析报告 | [OSS_DOC_ANALYSIS.md](../OSS_DOC_ANALYSIS.md) |

---

## 七、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 4.0 | 2026-05-29 | 新增 api/ 和 oo/ 文档体系，更新为 v3.6.0 |
| 3.1 | 2026-05-15 | 更新为 v3.2.0 Beta，添加 v3.2.0 完整文档链 |
| 3.0 | 2026-05-05 | 更新为 v3.1.0 Alpha，添加 v3.1.0 文档 |
| 2.1 | 2026-04-22 | 更新为 v2.9.0，添加 v2.9.0 文档入口 |
| 2.0 | 2026-04-17 | 更新为 v2.8.0，清理过时版本链接 |
| 1.0 | 2026-03-04 | 初始版本，整合所有文档索引 |

---

*本文档由 hermes-z440 维护*
*基于 [DOC_IMPROVEMENT_PLAN.md](../DOC_IMPROVEMENT_PLAN.md) 改进*