# PR-5379 TEST_PLAN — 覆盖目标与阶段

> 关联: `PR-5379_SPEC.md` · 证据: `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md`

---

## 1. 覆盖目标

本 PR 的"被测对象"不是某个函数，而是**CI 的可信度**。因此覆盖目标分三层：

| 层 | 目标 | 判定方式 |
|---|---|---|
| L1 数据层 | fixture 在无网络、无预置数据的干净环境里可确定性重建 | `prepare_tpch_fixture.sh` 幂等 + 4 类生成时自检全 OK |
| L2 断言层 | 每个新增断言的期望值来自**独立**来源，且**问的是有意义的问题** | SQLite 预言机对照 + 防空转不变量 + 变异测试 |
| L3 披露层 | CI 的结论与它实际执行的东西一致 | 执行核对门禁 + 三态摘要算术 |

## 2. 覆盖矩阵

| 需求（SPEC §2.1） | 测试手段 | 位置 |
|---|---|---|
| fixture 运行时生成、零入库 | `prepare_tpch_fixture.sh` 幂等实测；`check_no_log_tbl_json.sh` | `scripts/gate/` |
| 生成器现实度 | 4 类生成时自检；22 查询结果规模实测 | `scripts/gate/generate_tpch_sf.py` |
| 13 个零断言文件补真断言 | 断言计数 + SQLite 对照 + 防空转不变量 | `tests/integration/oracle/*.rs` |
| 独立预言机正确性 | `sqlite_oracle.rs` 自带单元测试（10 项） | `tests/common/sqlite_oracle.rs` |
| 缺数据 → NOT-RUN | 双模式实测（有 fixture / 无 fixture） | 40 个 fixture 依赖目标 |
| profile 不匹配 → NOT-RUN | `oracle_g1_tpch_sha256` 在 SF=0.01 下判 profile_mismatch | `tests/integration/tpch/` |
| 目标执行核对门禁 | 用**真实 CI 日志**（run 7613）验证 exit 1 + 列出 382 个 missing | `scripts/gate/check_test_target_execution.sh` |
| 已知差异登记合规 | 空表 exit 0；每条须有 issue + owner；actual==oracle 强制清理 | `scripts/gate/check_tpch_known_differences.sh` |
| 断言真的能杀缺陷 | 3 个引擎侧变异必须全部 CAUGHT | `scripts/gate/mutate_tpch_oracle_tests.sh` |
| CI 三态披露算术 | 合成日志验证 passed/failed/not_run/ignored 的加减 | `.gitea/workflows/ci.yml` |
| 引擎侧 22 查询对照 | SQLite 独立预言机逐条比对 | `tests/integration/sql/eval_22_vs_sf01.rs` |

## 3. 阶段

| 阶段 | 动作 | 通过标准 |
|---|---|---|
| P0 分析 | GitNexus + AOCI 检索影响面；实测复现 5 层问题 | 每层都有可复现命令与真实输出 |
| P1 RED | 先落 4 个门禁脚本（此时应能失败） | 门禁在旧状态下 FAIL（执行核对门禁实测 exit 1） |
| P2 GREEN | 实现共享模块、CI 接线、生成器改造、schema 修正 | 40 个目标在新 fixture + 新 schema 下通过 |
| P3 REFACTOR | 去重实现（共享 `adapt_sql` / 共享批量装载 / manifest 指纹） | 同一语义只有一处权威定义 |
| P4 GATE | clippy / fmt / 4 个门禁 / 变异测试 / `gitnexus detect_changes` | 全绿 |
| P5 PR | 5-类文档 + 独立盲审 + CI 验证 | 盲审无阻断项，CI 绿 |
| P6 RECORD/LEARN | 证据文档 + 9 条 lesson | 证据里每条结论都有实测输出 |

## 4. 明确不做（DEFERRED）的覆盖

| 项 | 原因 | 去向 |
|---|---|---|
| SF=1 规模层的 22 查询全量对照 | 需要 dbgen 产 SF=1 数据，CI 成本与本轮目标不匹配 | #5387 |
| `tests/integration/oracle/` 剩余 19 个零断言文件 | 超出本 PR 范围 | #5391 |
| 首次全量暴露的 382 个目标的失败归因 | 需要在 CI 跑出真实清单后分流 | #5388 |
| wire 类目标在修正 schema 后的期望值重标定 | 属"按错位数据标定的断言" | #5388 |

## 5. 验收口径

- **PASS** 必须同时满足：断言成立 **且** 该断言的期望值来自独立来源 **且** 该用例没有退化（结果非空 / 覆盖面非平凡）。
- **NOT-RUN** 是一等状态：必须带受控原因码，**不计入 passed**，且出现在披露清单里。
- **SKIP / ignore / 超时** 一律不得计入 PASS（`AGENTS.md §8` #5）。
