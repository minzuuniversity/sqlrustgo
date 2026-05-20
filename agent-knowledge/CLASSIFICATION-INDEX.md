# 文档分类索引

> 根据 chatgpt-ai-isolation-architecture.md 的数据风险分级和四层隔离体系
> 将 gmp_kg_build + gmp-qa-package + gmp_complete_archive 的文档资产按隔离层级分类
> 更新日期: 2026-05-20（补充 gmp_complete_archive 1813份文档）

---

## 一、分类原则

| 数据类型 | 风险级别 | 归属层级 | 是否允许出云 |
|----------|----------|----------|-------------|
| 通用框架代码 | 低 | repo-core-open | 可 |
| SQL 执行器/Parser | 低 | repo-core-open | 可 |
| UI 组件 | 低 | repo-core-open | 可 |
| GMP 法规正文（公开） | 低 | L1-sanitized-zone | 可（已公开） |
| ICH 指南（公开） | 低 | L1-sanitized-zone | 可（已公开） |
| 企业 SOP | 高 | L0-trust-zone | 禁止 |
| 批生产记录（EBR） | 极高 | L0-trust-zone | 禁止 |
| 配方工艺参数 | 极高 | L0-trust-zone | 禁止 |
| 偏差/CAPA 数据 | 高 | L0-trust-zone | 禁止 |
| 电子签名链 | 极高 | L0-trust-zone | 禁止 |
| 审计日志 | 极高 | L0-trust-zone | 禁止 |
| 验证方案/报告 | 高 | L0-trust-zone | 禁止 |
| 数据库 Schema | 中 | L1-sanitized-zone | 可（脱敏后） |
| 模拟/合成数据 | 低 | L1-sanitized-zone | 可 |
| Agent DSL 模板 | 低 | L2-cloud-zone | 可 |
| Agent Prompt | 低 | L2-cloud-zone | 可 |
| 验证规则 | 中 | L3-validation-zone | 禁止 |

---

## 二、gmp_kg_build 文档分类

### L0-trust-zone（绝对可信区，禁止出云）

| 来源 | 文档类型 | 数量 | 分类目标 |
|------|----------|------|----------|
| 2025GMP-MD/02_/SOP | 企业 SOP（机构与人员） | 126 | entities/ + rules/ |
| 2025GMP-MD/02_/DEV | 偏差记录 | 20 | evidence-templates/ |
| 2025GMP-MD/02_/CAP | CAPA 记录 | 2 | evidence-templates/ |
| 2025GMP-MD/02_/REC | 业务记录（人员相关） | 127 | evidence-templates/ |
| 2025GMP-MD/02_/POL | 质量政策 | 5 | rules/ |
| 2025GMP-MD/03_/SOP | 企业 SOP（厂房设施） | 38 | entities/ + rules/ |
| 2025GMP-MD/03_/REC | 业务记录（厂房相关） | 83 | evidence-templates/ |
| 2025GMP-MD/03_/POL | 厂房政策 | 24 | rules/ |
| 2025GMP-MD/04_/SOP | 企业 SOP（设备） | 119 | entities/ + rules/ |
| 2025GMP-MD/04_/VAL | 验证方案 | 62 | rules/ + evidence-templates/ |
| 2025GMP-MD/05_/SOP | 企业 SOP（物料） | 121 | entities/ + rules/ |
| 2025GMP-MD/05_/REC | 业务记录（物料相关） | 118 | evidence-templates/ |
| 2025GMP-MD/05_/CAL | 校准记录 | 2 | evidence-templates/ |
| 2025GMP-MD/06_/SOP | 企业 SOP（验证） | 39 | rules/ |
| 2025GMP-MD/06_/GUI | 管理规程（验证） | 117 | rules/ |
| 2025GMP-MD/06_/POL | 验证政策 | 8 | rules/ |
| 2025GMP-MD/06_/RET | 召回文件 | 7 | workflows/ |
| 2025GMP-MD/07_/SOP | 企业 SOP（文件管理） | 55 | rules/ |
| 2025GMP-MD/07_/BPR | 批生产记录 | 120 | evidence-templates/ |
| 2025GMP-MD/07_/VAL | 验证方案（文件） | 3 | rules/ |
| 2025GMP-MD/08_/SOP | 企业 SOP（生产） | 43 | entities/ + rules/ + workflows/ |
| 2025GMP-MD/08_/AUD | 审计记录 | 15 | evidence-templates/ |
| 2025GMP-MD/08_/OOS | 超标处理 | 2 | rules/ |
| 2025GMP-MD/08_/REC | 业务记录（生产相关） | 21 | evidence-templates/ |
| 2025GMP-MD/08_/STA | 统计分析 | 1 | rules/ |
| ZZ_模拟数据/偏差记录 | 模拟偏差数据 | ~5 | evidence-templates/ |
| ZZ_模拟数据/CAPA记录 | 模拟 CAPA 数据 | ~5 | evidence-templates/ |
| ZZ_模拟数据/供应商记录 | 模拟供应商数据 | ~4 | evidence-templates/ |
| ZZ_模拟数据/检测记录 | 模拟检测数据 | ~4 | evidence-templates/ |
| ZZ_模拟数据/生产记录 | 模拟生产数据 | ~4 | evidence-templates/ |
| knowledge_base_docs/ | 精炼知识文档 | 12 | rules/ + workflows/ |

**L0 小计: ~1,310 份文档**

### L1-sanitized-zone（脱敏 AI 区，可出云）

| 来源 | 文档类型 | 数量 | 分类目标 |
|------|----------|------|----------|
| 2025GMP-MD/01_/GUI | 文件管理规程（通用） | 13 | schemas/ |
| 2025GMP-MD/01_/SOP | SOP（文件管理，通用性高） | 16 | schemas/ |
| 2025GMP-MD/01_/REC | 记录模板（通用） | 1 | schemas/ |
| 2025GMP-MD/02_/GUI | 质量风险管理（通用框架） | 4 | schemas/ |
| 2025GMP-MD/03_/TRA | 培训文件（通用框架） | 10 | schemas/ |
| 2025GMP-MD/04_/GUI | 洁净区级别划分（通用标准） | 4 | schemas/ |
| 2025GMP-MD/06_/GUI | 验证管理规程（通用框架） | 117 | schemas/ |
| 2025GMP-MD/06_/REC | 验证记录模板 | 29 | schemas/ |
| 2025GMP-MD/07_/GUI | 文件管理规程（通用） | 54 | schemas/ |
| 2025GMP-MD/08_/GUI | 生产管理规程（通用） | 1 | schemas/ |
| 2025GMP-MD/08_/VAL | 验证方案（通用） | 1 | schemas/ |
| 2025GMP-MD/08_/CAL | 校准规程（通用） | 1 | schemas/ |
| 2025GMP-MD/09_/AGR | 协议模板 | 1 | schemas/ |
| ZZ_模拟数据/mock_test | 模拟测试数据 | ~6 | mock-data/ |
| ZZ_原始文件/ | 原始文件（需脱敏后使用） | 73 | schemas/（脱敏后） |

**L1 小计: ~361 份文档**

### L2-cloud-zone（云推理区）

| 来源 | 内容 | 分类目标 |
|------|------|----------|
| 设计文档（docs/design/） | 架构设计、实施计划 | dsl-templates/ + agent-prompts/ |
| kg_schema.md | 知识图谱 Schema | dsl-templates/ |
| 通用 GMP 法规条款 | 公开法规 | agent-prompts/ |

**L2 小计: 由 L0/L1 提炼生成，非原始文档**

### L3-validation-zone（执行验证区）

| 来源 | 内容 | 分类目标 |
|------|------|----------|
| gmp_kg_build 测试报告 | 164 场景测试 | test-scenarios/ |
| gmp-qa-package test-suite | CLI 测试 | test-scenarios/ |
| compliance.rs | 合规检查逻辑 | validators/ |
| audit.rs | 审计追踪逻辑 | validators/ |

**L3 小计: 由代码和测试资产提炼**

---

## 三、gmp-qa-package 独有资产分类

| 资产 | 归属层级 | 分类目标 |
|------|----------|----------|
| gmp.db (SQLite) | L0 | 原始数据库，禁止出云 |
| graph.db (PostgreSQL) | L0 | 图谱数据库，禁止出云 |
| 770 文档 bge-m3 向量 | L0 | 向量索引，禁止出云 |
| Rust 代码 (11文件) | repo-core-open | 可出云 |
| Python 脚本 (6文件) | repo-core-open | 可出云 |
| docs/ 技术文档 (17份) | L1 | 脱敏后可出云 |
| test-results/ | L3 | 测试验证 |

---

## 五、gmp_complete_archive 文档分类（1813份）

### 归档结构概览

```
gmp_complete_archive/
├── 01_原始GMP文件/GMP-fixed-MD/    ← 1448份（按章节组织）
│   ├── 01_/SOP,REC                 ← 文件管理 (30)
│   ├── 02_/SOP,DEV,CAP,REC,POL,GUI ← 质量管理 (284)
│   ├── 03_/SOP,REC,POL,TRA        ← 厂房设施 (155)
│   ├── 04_/SOP,GUI,VAL             ← 设备 (185)
│   ├── 05_/SOP,REC,CAL            ← 物料 (241)
│   ├── 06_/SOP,GUI,POL,RET        ← 验证 (200)
│   ├── 07_/SOP,GUI,BPR,VAL        ← 文件/批记录 (232)
│   ├── 08_/SOP,AUD,OOS,REC,STA    ← 生产 (85)
│   ├── 09_/AGR                     ← 协议 (1)
│   ├── ROOT_SOP/                   ← 核心SOP (17)
│   └── ZZ_原始文件/                ← 原始文件 (18)
│
├── 02_补充文档/                     ← 350份（独特补充内容）
│   ├── phase2/                     ← Phase2补充 (134)
│   │   ├── qms/                    ← QMS规程 (27)
│   │   ├── production/             ← 生产规程 (18)
│   │   ├── qc/                     ← QC规程 (20)
│   │   ├── materials/              ← 物料规程 (13)
│   │   ├── facilities/             ← 设施规程 (16)
│   │   ├── validation/             ← 验证规程 (10)
│   │   ├── docmgmt/                ← 文件管理 (12)
│   │   ├── computers/              ← 计算机系统 (6)
│   │   ├── security/               ← 安全卫生 (6)
│   │   └── org/                    ← 组织管理 (4)
│   ├── openclaw_supplement/        ← OpenClaw补充 (98)
│   │   ├── 01_GMP条款解释/         ← 条款解释 (53) ★ L1
│   │   ├── 02_变更控制/            ← 变更控制 (15) ★ L0
│   │   ├── 03_偏差CAPA/            ← 偏差CAPA (15) ★ L0
│   │   ├── 04_验证管理/            ← 验证管理 (5) ★ L0
│   │   └── 05_设备确认/            ← 设备确认 (10) ★ L0
│   ├── unique_to_me/               ← 企业特有 (77)
│   ├── missing/                    ← 缺失补充 (38)
│   ├── sops/                       ← SOP补充 (3)
│   └── phase3/                     ← Phase3 (0, 待补充)
│
└── 07_依赖配置/                     ← 环境配置 (2)
```

### L0-trust-zone 补充（禁止出云）

| 来源 | 文档类型 | 数量 | 已提取到 agent-knowledge | 目标文件 |
|------|----------|------|--------------------------|----------|
| 02_补充文档/openclaw_supplement/02_变更控制 | 变更控制规程 | 15 | ✓ | L0/rules/change-control-rules.yaml |
| 02_补充文档/openclaw_supplement/03_偏差CAPA | 偏差/CAPA规程 | 15 | ✓ | L0/rules/deviation-capa-rules-enhanced.yaml |
| 02_补充文档/openclaw_supplement/04_验证管理 | 验证管理规程 | 5 | ✓ | L0/rules/validation-rules.yaml |
| 02_补充文档/openclaw_supplement/05_设备确认 | 设备确认方案 | 10 | ✓ | L0/rules/validation-rules.yaml |
| 02_补充文档/phase2/qms | QMS规程 | 27 | ✓ | L0/rules/qms-rules.yaml |
| 02_补充文档/phase2/production | 生产规程 | 18 | ✓ | L0/rules/production-rules.yaml |
| 02_补充文档/phase2/materials | 物料规程 | 13 | ✓ | L0/rules/material-warehouse-rules.yaml |
| 02_补充文档/phase2/facilities | 设施规程 | 16 | ✓ | L0/rules/facility-equipment-rules.yaml |
| 02_补充文档/phase2/qc | QC规程 | 20 | 待提取 | L0/rules/qc-rules.yaml |
| 02_补充文档/phase2/validation | 验证规程 | 10 | 部分提取 | L0/rules/validation-rules.yaml |
| 02_补充文档/phase2/docmgmt | 文件管理规程 | 12 | ✓ | L0/rules/data-integrity-doc-rules.yaml |
| 02_补充文档/phase2/computers | 计算机系统 | 6 | ✓ | L0/rules/data-integrity-doc-rules.yaml |
| 02_补充文档/phase2/security | 安全卫生 | 6 | ✓ | L0/rules/facility-equipment-rules.yaml |
| 02_补充文档/unique_to_me/* | 企业特有规程 | 77 | 待提取 | L0/rules/ + entities/ |
| 02_补充文档/missing | 缺失补充 | 38 | 待提取 | L0/rules/ + entities/ |
| 01_原始GMP文件/ROOT_SOP | 核心SOP | 17 | 部分提取 | L0/entities/ + rules/ |
| 01_原始GMP文件/各章节SOP | 企业SOP | ~1200 | 与2025GMP-MD重叠 | L0/entities/ + rules/ |

**gmp_complete_archive L0 补充小计: ~350份独特文档（不含与2025GMP-MD重叠部分）**

### L1-sanitized-zone 补充（可出云）

| 来源 | 文档类型 | 数量 | 已提取到 agent-knowledge | 目标文件 |
|------|----------|------|--------------------------|----------|
| 02_补充文档/openclaw_supplement/01_GMP条款解释 | GMP条款解释 | 53 | ✓ | L1/schemas/gmp-clauses-index.yaml |
| 01_原始GMP文件/各章节GUI | 通用管理规程 | ~300 | 与2025GMP-MD重叠 | L1/schemas/ |
| 01_原始GMP文件/各章节TRA | 培训文件 | ~10 | 与2025GMP-MD重叠 | L1/schemas/ |

**gmp_complete_archive L1 补充小计: ~53份独特文档**

### gmp_complete_archive 与 2025GMP-MD 的关系

| 维度 | 01_原始GMP文件/GMP-fixed-MD | gmp_kg_build/2025GMP-MD |
|------|---------------------------|------------------------|
| 文档数量 | 1448 | 1677 |
| 文档组织 | 按章节编号(01-09) | 按章节编号(01-08) |
| 文档类型 | SOP/GUI/VAL/BPR/REC等 | SOP/GUI/VAL/BPR/REC等 |
| 重叠度 | 高度重叠（同源文件） | 高度重叠（同源文件） |
| 差异 | 含ROOT_SOP(17份) + ZZ_原始文件(18份) | 含ZZ_模拟数据 + knowledge_base_docs |
| 结论 | **01_原始GMP文件与2025GMP-MD为同一批文件的不同整理版本** | |

**关键差异: gmp_complete_archive 的独特价值在于 02_补充文档（350份），这是2025GMP-MD所没有的**

---

## 六、Agent 数字知识资产汇总

### L0-trust-zone 规则库

| 规则文件 | 规则数量 | 来源 |
|----------|----------|------|
| weighing-rules.yaml | 7 | 2025GMP-MD 称量SOP |
| deviation-capa-rules-enhanced.yaml | 6+ | openclaw_supplement/03_偏差CAPA |
| change-control-rules.yaml | 7 | openclaw_supplement/02_变更控制 |
| validation-rules.yaml | 6+ | openclaw_supplement/04+05 |
| production-rules.yaml | 6 | phase2/production |
| qms-rules.yaml | 8 | phase2/qms |
| data-integrity-doc-rules.yaml | 4+ | phase2/docmgmt + openclaw |
| material-warehouse-rules.yaml | 5 | phase2/materials + ROOT_SOP |
| facility-equipment-rules.yaml | 5 | phase2/facilities + security |
| **合计** | **55+** | |

### L0-trust-zone 实体定义

| 实体文件 | 实体数量 | 来源 |
|----------|----------|------|
| gmp-core-entities.yaml | 6 | 2025GMP-MD |
| validation-change-entities.yaml | 4 | openclaw_supplement/04+05+02 |
| **合计** | **10** | |

### L1-sanitized-zone Schema

| Schema文件 | 来源 |
|------------|------|
| gmp-clauses-index.yaml | openclaw_supplement/01_GMP条款解释 |

### L3-validation-zone 测试场景

| 场景文件 | 场景数量 | 来源 |
|----------|----------|------|
| weighing-test-scenarios.yaml | 10 | 称量操作 |

---

## 四、双仓分类

### 仓库 A: Core Open Source（可出云）

| 资产 | 来源 |
|------|------|
| SQLRustGo 引擎代码 | gmp-qa-package/src/sqlrustgo/ |
| GMP CLI 工具 | gmp-qa-package/build/ |
| Python 构建脚本 | gmp-qa-package/scripts/ |
| 通用 Agent 框架 | 待开发 |
| 通用 Workflow Engine | 待开发 |
| UI 组件 | 待开发 |
| 数据库 Schema（脱敏） | agent-knowledge/L1-sanitized-zone/schemas/ |

### 仓库 B: Private GMP Layer（禁止出云）

| 资产 | 来源 |
|------|------|
| 企业 SOP 全文 | gmp_kg_build/2025GMP-MD/*/SOP/ |
| 批生产记录模板 | gmp_kg_build/2025GMP-MD/07_/BPR/ |
| 配方工艺参数 | gmp_kg_build/2025GMP-MD/08_/SOP/ |
| 偏差/CAPA 数据 | gmp_kg_build/2025GMP-MD/02_/DEV+CAP/ |
| 审计策略 | gmp_kg_build/2025GMP-MD/08_/AUD/ |
| 向量知识库 | gmp-qa-package/data/gmp.db |
| 判定规则库 | agent-knowledge/L0-trust-zone/rules/ |
| 实体定义库 | agent-knowledge/L0-trust-zone/entities/ |
