# 当前 open Issue DAG 分析 (2026-08-10)

## 总览: 21 个 open Issue

按 minimax agent 状态分类:
- **minimax 已认领** (5): #3907 #3909 #3910 #3911 (claimed)
- **其他 agent 认领/未认领** (16)

## DAG (依赖关系分析)

```
            ┌── #3907 V312-20
            ├── #3909 V312-22
            ├── #3910 V312-23
minimax ─────├── #3911 V312-24 (P0, 已认领)
claimed ─────┴── (其他 minimax 之前 issue 全部 closed)

独立项 (无 minimax 上下文依赖, 可立即开始):
├── #3984 V312-11-fix VALUES 构造器 (FROM clause) - parser 改动, 独立
├── #3985 V312-11-fix Double-quoted identifier - parser 改动, 独立
├── #3986 V312-11-fix SET statement - parser 改动, 独立
├── #3980 V312-19-followup signoff tolerance - 1 file 改动, 独立
└── #3945 V312-19-followup corpus runner fix - 已 PR #3982 partial, 独立收尾

阻塞项 (依赖其他 Issue):
├── #3970 V312-17 VALUES 构造器执行 ─┐
├── #3971 V312-18 NOT NULL/CTAS/别名 ─┼─ 互不依赖但都需要 executor 改动
├── #3969 V312-16 sqllogictest runner ──┘ (可与 #3984 协同)
├── #3964 V313-01 WAL Replay - 依赖 #3911 V312-24 测试基础设施
├── #3898 V312-11 SQLite SQLLogicTest - 依赖 #3986/#3985/#3984/#3969 (harness 支持)
├── #3959 V312-24 LOAD DATA/TLS/Compression - 依赖 #3911 V312-24
├── #3942 V312-19-followup R2.8 - 依赖 #3911 V312-24
├── #3943 V312-19-followup R2.4 SEM-4 - 依赖 #3911 V312-24
└── #3887 V312-MASTER - 总控, 等待所有子项 closed

未认领 (其他 agent 领域):
├── #3903 V312-16 Window/GIS/JSON (其他 agent)
├── #3904 V312-17 Coverage/Disabled-Test (其他 agent)
├── #3905 V312-18 SF=10/Sysbench/Observability (其他 agent)
├── #3907 V312-20 v3.6-v3.10 Cross-version (minimax claimed, 之前 open)
├── #3908 V312-21 (已 closed 之前)
└── #3909 V312-22 Execution Architecture (minimax claimed, 之前 open)
```

## 可直接开始 (无依赖, 单 PR 可完成)

| Issue | 范围 | 单 PR 大小 | 估计复杂度 | 验收标准 |
|-------|------|----------|----------|----------|
| **#3980** signoff tolerance | `assert_reviewer_signoff.sh` 1 文件, 5-10 行 | <50 行 | 低 | signoff 在 signoff 落后 HEAD 1-3 commits 时仍 exit 0 |
| **#3986** SET statement | parser: 加 SET 关键字处理 | <100 行 | 中 | `SET debug_force_external=true` 不再 parse error; 3 个 quantile_fun.test 可运行 |
| **#3985** Double-quoted identifier | parser: 加 `"x"` 作为 identifier 引用 | <50 行 | 低 | `SELECT "BigColumn" FROM t` 不再 parse error |
| **#3984** VALUES in FROM | parser: FROM 子句接受 `(VALUES(...))` | <100 行 | 中 | `setops__test_setops.test` PASS |
| **#3945** corpus runner status | `test_sql_corpus.sh` Pattern 3 修正 | <100 行 | 中 | ALL_TARGETS_REPORT 反映真实 status (parser_fixtures=pass, sqllogictest_local=6/16, tpch_sf1 真实 count) |

## 阻塞项 (需要其他 issue 先完成)

| Issue | 阻塞依赖 | 解锁条件 |
|-------|----------|----------|
| **#3970** VALUES 执行 | #3984 (parser) + executor 改造 | parser 支持 VALUES 后, executor 加 VALUES evaluation |
| **#3971** NOT NULL/CTAS | executor 改造 (无前置 parser 依赖) | (相对独立, 但需要 #3909 V312-22 已完成) |
| **#3969** sqllogictest runner 增强 | 无前置依赖 | (独立, 可立即开始) |
| **#3964** V313-01 WAL Replay | #3911 V312-24 测试基础设施 | 测试 harness 可用后开始 |
| **#3898** V312-11 SQLite SQLLogicTest | #3969 (set variable / include / connection) | runner 支持 directives 后开始 |
| **#3959** V312-24 LOAD DATA | 单独 (但属于 minimax 之前 work) | (独立, 可由 minimax 继续) |
| **#3942** R2.8 A5 coverage | #3911 V312-24 (测试基础设施) | 优化测试运行时间后开始 |
| **#3943** R2.4 SEM-4 | #3911 V312-24 | (独立, 但需要测试数据) |
| **#3887** V312-MASTER | 所有子项 closed | (总控, 不能自己 closed) |

## minimax agent 认领的 4 个 open issue

| Issue | 状态 | 优先级 |
|-------|------|--------|
| **#3907** V312-20 v3.6-v3.10 Cross-version Disposition | minimax claimed (2026-08-09), 仍 open | 中 |
| **#3909** V312-22 Execution Architecture Debt | minimax claimed (历史性), 仍 open | 中 |
| **#3910** V312-23 Storage/Index/WAL Tooling | minimax claimed (历史性), 仍 open | 中 |
| **#3911** V312-24 Test Infrastructure (P0) | minimax claimed (2026-08-10), 已 assign, 仍 open | 高 |

## 推荐: 可直接开始的 5 个独立项 (按工作量升序)

1. **#3980** signoff tolerance (低, 5 min)
2. **#3985** Double-quoted identifier (低, 30 min)
3. **#3984** VALUES in FROM (中, 1 hour)
4. **#3986** SET statement (中, 1-2 hours)
5. **#3945** corpus runner status fix (中, 1-2 hours)

## 推荐: minimax agent 重点 follow-up (3 个 V312-19 follow-up)

- **#3945** corpus runner (与 #3984/#3986 协同, 都涉及 parser 改动)
- **#3980** signoff tolerance (独立, 立即可做, 防止 sign-off chicken-and-egg)
- **#3943** R2.4 SEM-4 (独立, 需测试基础设施就绪后开始)

## 建议的下一步 (按优先级)

1. **本周内** (低难度独立项): 完成 #3980, #3985, #3984, #3986, #3945
2. **V312-19 follow-up** (中等): #3980 + #3945 完成后再申请 #3906 / #3972 re-apply close
3. **后续** (V312-24 依赖): #3911 + #3942 + #3943 + #3959 联合整改

## 建议的 openspec change

1. **openspec/changes/v312-11-parser-fixes**: 整合 #3984 #3985 #3986 三件套
2. **openspec/changes/v312-19-followup-final**: 整合 #3980 #3945
3. (可选) **openspec/changes/v312-19-call-graph-correction**: 整合 #3942 #3943
