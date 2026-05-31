# v3.8.0 文档索引

## 目录结构

```
v3.8.0/
├── README.md                    # 本文件 - 文档索引
├── alpha/                       # Alpha 阶段文档
│   ├── ALPHA_GATE_*.md        # Alpha 门禁相关
│   ├── PR-850_*.md            # PR-850 设计/测试文档
│   ├── PR-870_*.md            # PR-870 设计/测试文档
│   ├── RECOVERY_TEST_*.md    # 恢复测试设计
│   └── TEST_QUALITY_*.md       # 测试质量整改
├── beta/                       # Beta 阶段文档
│   └── BETA_GATE_*.md         # Beta 门禁相关
├── rc/                         # RC 阶段文档
│   └── COMPREHENSIVE_GATE_*.md # 综合门禁报告
├── ga/                         # GA 阶段文档
│   └── GA_GATE_*.md           # GA 门禁清单
│
├── ARCHITECTURE.md             # 架构设计
├── ARCHITECTURE_DECISIONS.md  # 架构决策记录
├── ROADMAP.md                  # 路线图
├── VERSION_PLAN.md             # 版本计划
├── DEVELOPMENT_PLAN.md           # 开发计划
├── FEATURE_CHECKLIST.md        # 功能清单
│
├── PR-800_*.md               # PR-800 (COM_QUERY AST Routing)
├── PR-830E_*.md              # PR-830E (WAL Engine Restart)
├── PR-830F_*.md              # PR-830F (WAL Lifecycle)
├── PR-840_*.md               # PR-840 (DML Transaction)
│
├── INTEGRATION_GATE_*.md      # 集成门禁
├── TEST_PLAN.md               # 测试计划
└── ...
```

## 按阶段分类

### Alpha 阶段
| 文档 | 描述 |
|------|------|
| `alpha/ALPHA_GATE_CONTRACT.md` | Alpha 门禁契约 |
| `alpha/ALPHA_GATE_REPORT.md` | Alpha 门禁报告 |
| `alpha/ALPHA_DESIGN_TEST_GATE.md` | Alpha A7 设计/测试文档检查 |
| `alpha/TEST_QUALITY_REMEDIATION_PLAN.md` | 测试质量整改计划 |
| `alpha/PR-850_DESIGN.md` | PR-850 功能设计 |
| `alpha/PR-850_TEST_DESIGN.md` | PR-850 测试设计 |
| `alpha/PR-870_DESIGN.md` | PR-870 功能设计 (STUB) |
| `alpha/PR-870_TEST_DESIGN.md` | PR-870 测试设计 |
| `alpha/RECOVERY_TEST_DESIGN.md` | 恢复测试设计 |
| `alpha/RECOVERY_TEST_MIGRATION_PLAN.md` | 恢复测试迁移计划 |

### Beta 阶段
| 文档 | 描述 |
|------|------|
| `beta/BETA_GATE_CONTRACT.md` | Beta 门禁契约 |
| `beta/BETA_GATE_REPORT.md` | Beta 门禁报告 |
| `beta/SGL_BETA_GATE_REPORT.md` | SGL Beta 门禁报告 |

### RC 阶段
| 文档 | 描述 |
|------|------|
| `rc/COMPREHENSIVE_GATE_REPORT.md` | 综合门禁报告 |

### GA 阶段
| 文档 | 描述 |
|------|------|
| `ga/GA_GATE_CHECKLIST.md` | GA 门禁清单 |

## 按功能分类

### 架构
| 文档 | 描述 |
|------|------|
| `ARCHITECTURE.md` | 架构设计 |
| `ARCHITECTURE_DECISIONS.md` | 架构决策记录 |
| `ROADMAP.md` | 路线图 |
| `ARCH-900.md` | ARCH-900 存储引擎研究 |

### PR 功能文档
| 文档 | 描述 |
|------|------|
| `PR-800_*.md` | COM_QUERY AST Routing |
| `PR-830E_*.md` | WAL Engine Restart |
| `PR-830F_*.md` | WAL Lifecycle |
| `PR-840_*.md` | DML Transaction Interception |

### 测试
| 文档 | 描述 |
|------|------|
| `TEST_PLAN.md` | 测试计划 |
| `COVERAGE-DELTA-ANALYSIS.md` | 覆盖率分析 |

### 门禁
| 文档 | 描述 |
|------|------|
| `INTEGRATION_GATE_REPORT.md` | 集成门禁报告 |
| `INTEGRATION_GATE_PLAN.md` | 集成门禁计划 |
| `INT1_WAL_RECOVERY_RTI_CHAIN.md` | INT1 RTI 链 |
| `INT234_RTI_CHAIN.md` | INT2/3/4 RTI 链 |

---

*最后更新: 2026-06-01*
