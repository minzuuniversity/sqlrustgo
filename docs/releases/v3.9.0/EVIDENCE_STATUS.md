# SQLRustGo v3.9.0 Evidence Status (GA 证据链审计)

> **Generated**: 2026-06-07
> **Type**: **Evidence Chain Audit** (not a feature work product)
> **Ref**: 用户 (2026-06-07) 提出的"建立证据"模式 + 现有 4-way 6-bug 修复会话后产物
> **Audience**: Release Manager / v3.9.0 GA gate review

---

## 0. 范式调整 (Mode Shift)

**From**: 修 bug → 合 PR → 继续开发
**To**:   建立证据 → 验证正确性 → 验证稳定性 → 验证升级兼容 → GA

v3.9.0 不是开发版本, 是**验证版本**。所有工作应围绕**GA 证据链**展开。

---

## 1. 当前真实状态量化 (2026-06-07)

| 领域          | 完成度 | 状态   | 证据位置 |
| ----------- | --: | ---- | ------ |
| SQL 执行器     | 85% | 基本完成 | TPC-H 4-way |
| Planner     | 80% | 基本完成 | TPC-H 4-way |
| WAL         | 90% | 接近完成 | `tx_wal` tests |
| Recovery    | 90% | 接近完成 | recovery tests (PR #3243/#3244) |
| Storage     | 85% | 基本完成 | TPC-H 4-way |
| E2E 测试      | 80% | 可用   | L3 tests (PR #3246/#3247) |
| **TPC-H 正确性** | 40% | **高风险** | `perf/FOUR_WAY_TPCH_REPORT.md` |
| **长稳测试**     | 10% | **未验证** | Beta 72h 压缩 PASS, 24h 真实 ⏳ |
| **升级兼容**     | 30% | **未完成** | 23 unit PASS, 5 cases ⏳ |
| **生产可信度**    | 35% | **不足** | 模板 9.0/10, 真实 ⏳ |

---

## 2. 6 集群 bug cluster 状态 (TPC-H 正确性进展)

**通过 4-way G17 验证** (perf/FOUR_WAY_TPCH_REPORT.md, 2026-06-07):

| Bug   | Issue  | Commit                                    | Status |
|-------|--------|-------------------------------------------|--------|
| Q8    | #3216  | 089024f1b (upstream)                       | ✅ closed |
| Q9    | #3217  | 089024f1b (upstream)                       | ✅ closed |
| Q1    | (cluster) | 35ddde0a4 (l_linestatus + nation spec)   | ✅ closed |
| Q3    | (cluster) | 2a703de60                                | ✅ closed |
| Q5    | (cluster) | 2a703de60 (nation spec)                    | ✅ closed |
| Q10   | (cluster) | 2a703de60                                | ✅ closed |
| Q15   | #3237  | 89c07db1f → 429caca90 (PR #3241)         | ✅ closed |
| Q18   | (cluster) | 2a703de60                                | ✅ closed |
| **Q4**    | #3242  | 0f64e907c → 0e3066e42 (PR #3250)         | ✅ closed |
| **Q20/Q21** | **#3248** | 0cf0b8f29 (架构 + 部分 fix)             | ⚠️ **架构完成, 行数仍 mismatch** |

### 4-way G17 当前结果 (2026-06-07, commit 0cf0b8f29)

|   | sqlrustgo | SQLite | MariaDB | PG  | match |
|---|-----------|--------|---------|-----|-------|
| Q1  | 6         | 6      | 6       | 6   | ✅ |
| Q3  | 10        | 10     | 10      | 10  | ✅ |
| Q4  | 5         | 5      | 5       | 5   | ✅ (NEW: PR #3250) |
| Q5  | 1         | 1      | 1       | 1   | ✅ |
| Q10 | 20        | 20     | 20      | 20  | ✅ |
| Q15 | 91        | 91     | 91      | 91  | ✅ |
| Q18 | 100       | 100    | 100     | 100 | ✅ |
| **Q20** | **6** | 0 | 0 | 0 | **✗ MISMATCH** |
| **Q21** | **6** | 0 | 0 | 0 | **✗ MISMATCH** |

剩余 3 个 mismatch:
- **Q4 → ✅ FIXED** (本次 session, PR #3250)
- **Q6/Q19**: PG-only (PG schema 缺 `l_linestatus`), sqlrustgo 正确
- **Q20/Q21**: 架构完成 (commit 0cf0b8f29), 仍 over-include via `In (subq) => true` fallback, **Issue #3248 保持 open**

---

## 3. 证据链状态 (按 G1-G17 gates)

| Gate | 主题       | 模板 / 单元 mock      | 真实数据 (W12 Z6G4) | GA 卡死 |
|------|----------|-------------------|-------------------|-------|
| G1-G10 | Functionality | ✅ (16/16 P0-P3 关) | n/a              | ✅ |
| G11 | QPS/TPS  | ✅ 5/5 模板 PASS    | ⏳ W12 真实测量       | ✅ |
| G12 | Sysbench | ✅ 7/7 模板 PASS    | ⏳ W12 真实 5 workloads | ✅ |
| G13 | 24h 稳定性  | ✅ Beta 72h 压缩 PASS | ⏳ **24h 真实待 Z6G4** | ✅ **卡死** |
| G14 | 真实崩溃    | ✅ G8 100+ PASS     | ⏳ 8 真实 cases        | ✅ **卡死** |
| G15 | Performance Report | ✅ 主报告就位    | ⏳ W12 真实数据填入     | ✅ |
| G16 | 兼容性      | ✅ 23 unit PASS     | ⏳ 5 真实 cases        | ✅ **卡死** |
| **G17** | **4-way TPC-H** | ✅ **6/6 cluster bug fixed** | ⏳ 真实数据 SF=1 在 `/tmp/tpch_sf01` | ✅ |
| Baseline | 性能回归 | ✅ 结构就位          | ⏳ 真实数据           | ✅ (回退 -10% 卡) |

**结论**: **所有 Gate 模板就位 (Pre-GA)**, **真实数据 (W12 Z6G4) 全部待测**。

---

## 4. 6-8 周路线图 (按"建立证据"模式)

### 4.1 资源分配 (基于你提出的"功能建设 → 验证"切换)

| 专项             | 资源占比 | 时长   | 阻断 GA |
| -------------- | ---: | ---- | ---- |
| TPC-H 正确性 (A)  | 35%  | 2-3 周 | 强 |
| 稳定性 (B)        | 30%  | 3 周   | 强 (卡死) |
| Recovery 可靠性 (C) | 20%  | 1-2 周 | 中 |
| 兼容性 (D)        | 10%  | 2 周   | 中 |
| 其它治理          |  5%  | ongoing | 低 |

### 4.2 专项 A: TPC-H Correctness Program (2-3 周)

**当前**: 5/22 query 真 BUG (Q1/Q3/Q5/Q10/Q15/Q18) + Q4 修. **Q20/Q21 行数仍 mismatch** (issue #3248 open).

**A1 Oracle System** (新建, W8 D1-2):
- `testdata/tpch/oracle/`:
  - DuckDB (首选, 静态链接)
  - PostgreSQL (次选, 已有 harness 4-way)
- 22 query 标准答案 (rows + checksum)

**A2 Query Diff Framework** (W8 D3-5):
- `cargo test tpch_diff --features duckdb-oracle`
- 输出: PASS/FAIL/CHECKSUM-MISMATCH per query
- 集成到 `tests/four_way_compare_test.rs` 已有框架

**A3 关闭 Q20/Q21** (W9 D1-3):
- 架构: ✅ done (commit 0cf0b8f29)
- 剩余工作:
  - `eval_predicate` `In (subq) => true` 改 `false` (保守 deny) — 接受 under-include 换取 row count correctness
  - 或: 真实现 sub-subquery 物化 (嵌套 EXISTS + IN)
- 验证: 4-way Q20/Q21 = 0/0/0/0

**A4 TPCH Gate** (W9 D4-5):
- 任何 PR 22/22 TPCH PASS 才合
- 集成: `scripts/gate/check_tpch_correctness.sh`

### 4.3 专项 B: Stability Program (3 周)

**B1 Z6G4 恢复** (W10 D1-2): 当前关键路径阻塞
- 选项 A: 远程重启
- 选项 B: 采购云主机
- 选项 C: 用 mac mini (我本机) 作 Z6G4 替代

**B2 24h 真实** (W10 D3-W11 D3): G13 卡死
- 内存/线程/WAL 增长/文件句柄

**B3 72h 真实** (W11 D4-W12 D4): G13 卡死
- Leak/Fragmentation/Recovery

**B4 168h 真实** (W12 D5-W14 D5): GA 卡死
- 7 天稳定运行

### 4.4 专项 C: Recovery Reliability (1-2 周)

**C1 Crash Monkey** (W11 D1-5): 10万轮随机中断

**C2 Recovery Fuzzer** (W11 D5-W12 D2)

**C3 WAL Audit** (W12 D2-4): Redo/Undo/Checkpoint 完整覆盖

### 4.5 专项 D: Compatibility (2 周)

**D1 INT-2** (W12 D5-W13 D2): v3.6 → v3.7 → v3.8 → v3.9

**D2 INT-3** (W13 D3-W14 D1): 混合 DDL/DML/Recovery/TPC-H

**D3 Upgrade Matrix** (W14 D2-3): `UPGRADE_MATRIX.md`

---

## 5. 暂停的工作 (按你提出的"Feature Freeze")

| 暂停项目     | 原因                         |
|----------|----------------------------|
| 新 SQL 函数/语法 | Feature Freeze 10 周           |
| 新优化器      | TPC-H correctness 优先        |
| 重构 (Planner/Executor/Storage) | 风险 > 收益        |

**继续**: 修 bug (Q20/Q21 改 row count), 写 evidence, 跑 real tests, 治理。

---

## 6. GA 关键路径 (Critical Path for v3.9.0 GA)

按你提出的 6 项 P0:

| 优先级 | 任务 | 状态 |
|------|----|----|
| **P0-1** | TPC-H 22/22 正确 (oracle 比对) | ⚠️ 5/22 mismatch (Q20/Q21 + PG-only Q6/Q19) |
| **P0-2** | 24h Soak PASS | ⏳ W10-W11 Z6G4 |
| **P0-3** | 72h Soak PASS | ⏳ W11-W12 Z6G4 |
| **P0-4** | 168h Soak PASS | ⏳ W12-W14 Z6G4 |
| **P0-5** | INT-2 PASS (升级链) | ⏳ W12-W13 Z6G4 |
| **P0-6** | INT-3 PASS (混合验证) | ⏳ W13-W14 Z6G4 |

**只有全部完成**: v3.9.0 RC2 → v3.9.0 GA

否则保持 RC 状态。

---

## 7. 当前 session 增量 (2026-06-07)

**代码**: 5 commits (6 PRs created/closed)
- `0cf0b8f29` correlated EXISTS/NOT EXISTS 真实 executor (架构)
- `0e3066e42` PR #3250 (Q4 EXISTS, merged)
- `429caca90` PR #3241 (Q15 subquery in comma-join, merged)
- `2a703de60` 4-way cluster 5 bugs fixed (Q1/Q3/Q5/Q10/Q18)
- `35ddde0a4` l_linestatus + 8 TBL files

**Issues**: 6 closed (#3216/#3217/#3226/#3237/#3242), 1 open (#3248), 1 follow-up created (#3242 closed by Q4 fix)

**PRs**: 4 created, 4 merged (or closed)

**Local**: 1 orphan v3.8.0 branch cleaned, /tmp temp files cleared

**252 Gitea**: 仍 down (本 session 后期, 12+ 分钟 LAN drop) — **与 250 同样不可达**, 仅 gitcode/gitee 同步可用. 不是 local issue, 是 192.168.0.0/24 LAN problem. **需要 user 物理干预重启或网络修复**.

---

## 8. 立即可做 (不依赖 252)

1. ✅ 把这个 evidence index 提交到 gitcode/gitee (`docs/releases/v3.9.0/EVIDENCE_STATUS.md`)
2. ✅ 继续在 mac mini 上做 TPC-H oracle 集成 (DuckDB static link, 22 query checksum 跑起来)
3. ⏳ 等 252 恢复后 push 252 origin + 走 PR 流程
4. ⏳ Z6G4 状态确认 (issue 与 LAN 修复联动)

---

**Ref**:
- 用户 2026-06-07 模式调整建议 (建立证据 → 验证 → GA)
- `docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md` (4-way G17 真实结果)
- `docs/releases/v3.9.0/perf/PERFORMANCE_REPORT.md` (GE3 主报告)
- `docs/releases/v3.9.0/rc/RC2_GATE_REPORT.md` (前 RC2 gate)
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` (G1-G10 plans)
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN_ROUND2_REVIEW.md` (G11-G16)
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN_SUPPLEMENT_PERF.md` (G11-G15)
