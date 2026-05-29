# v3.6.0 Coverage Analysis Report

## Alpha Gate Results (2026-05-29)

| Crate | Coverage | vs Alpha(75%) | vs GA(85%) | 状态 |
|-------|----------|---------------|------------|------|
| types | 87.62% | ✅ | ✅ | 🟢 |
| parser | 47.16% | ❌ | ❌ | 🔴 需专项冲刺 |
| planner | 92.23% | ✅ | ✅ | 🟢 |
| optimizer | 91.26% | ✅ | ✅ | 🟢 |
| executor | 72.04% | ❌ | ❌ | 🟡 需补充测试 |
| storage | 81.76% | ✅ | ❌ | 🟡 |
| transaction | 91.51% | ✅ | ✅ | 🟢 |
| catalog | 92.17% | ✅ | ✅ | 🟢 |
| **Average** | **81.97%** | ✅ | ❌ | |

## Coverage Gaps

### Parser (47.16%) — 根因
- parser.rs 5,077 行单体文件，test_debug_having() 缺闭合括号
- ~70 个测试函数嵌套为内联项，未计入测试
- **修复方案**: 创建独立测试文件 crates/parser/tests/ (不修改 parser.rs)

### Executor (72.04%) — 根因
- event.rs + merge.rs 测试刚部署到 Z6G4
- stored_proc/execution.rs 覆盖率仅 14.43% (1482 未覆盖区域)
- stored_proc/cte.rs 覆盖率 32.93%

## Beta Gate 目标
- Average ≥85%
- 所有 crate 单项 ≥75%
- parser + executor 专项测试冲刺
