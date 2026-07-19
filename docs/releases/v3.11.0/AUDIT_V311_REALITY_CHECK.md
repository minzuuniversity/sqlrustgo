# v3.11.0 全面真实性核查报告

> **Version**: v3.11.0
> **Date**: 2026-07-20
> **Auditor**: MiniMax-M3 (independent verification, 2nd pass)
> **Method**: 实测 cargo test / cargo llvm-cov / 源码静态分析 / 文档交叉对照
> **Status**: 大量虚假声明;真实完成度显著低于文档声明

---

## 一、执行摘要(真实情况)

| 维度 | 文档声明 | 实测/核实结果 | 偏差 |
|------|----------|--------------|------|
| **v3.11.0 GA tag** | 已发布 | `v3.11.0` tag 真实存在(在 develop/v3.11.0 上,不在 main 上) | 误导性 |
| **TPC-H SF=1 22/22 PASS** | 多个文档声明 | **未执行**:`tpch_sf1_22_in_process_regression` 仍 `#[ignore]`,fixture 不存在 | **虚假** |
| **TPC-H SF=0.001 22 queries 测试** | "1 PASS" | `tpch_22_mysql_cli_wire_test` 因 data dir 缺失,测试代码 `[SKIP]` 后报 "ok" | **假通过** |
| **In-process TPC-H 22 测试** | 5/5 PASS | 3 个全部失败:Connection reset by peer(引擎无法启动) | **虚假** |
| **存储层 lib 测试** | "tests pass" | `sqlrustgo-storage` **lib 测试无法编译**(E0596 缺 `mut`) | **阻断** |
| **总测试数** | "615+ lib tests" | 7 crate 实测:**1,430 PASS / 0 FAIL / 1 IGNORED**;storage 未跑(编译失败) | 真实 |
| **覆盖率 L1_8=80.60%** | GA Gate G3 PASS | 实测:parser 62.45%(-8.77%),mysql-server 40.62%(-10.91%),storage 0%(不编译) | **虚假** |
| **23/23 任务完成** | LEGACY_DEBT_TRACKING_TABLE | FEATURE_CHECKLIST 自相矛盾:V311-03/04/05/08/10/11/12/14/18/20/21 全部 ⏳ TODO | **内部矛盾** |
| **F-23/F-24/F-25/F-26 集成** | CLOSED ✅ | 源码验证 F-23/F-24 真正集成;F-25/F-26 仅 **crates/storage lib** 导出,主执行路径未调用 | 部分虚假 |
| **历史债务 17/45 关闭** | audit report | 实际:26/48 完成(54%),含 4 项 PENDING 阻塞 GA | 大致真实 |
| **168h SOAK** | "✅ PASS" | 实际只跑 **51h**,未达 168h GA 阈值;GA 要求 168h | **虚假** |
| **cargo test --workspace** | "300+ tests pass" | **未执行成功**;storage 编译错误阻断整 workspace 跑 | **未验证** |

---

## 二、虚假的测试"PASS"逐项分析

### 2.1 🔴 TPC-H 22/22 测试 — 完全未执行

**文档声明**:
- `COMPREHENSIVE_ASSESSMENT_REPORT.md` L13: "TPC-H SF=1 22/22 通过"
- `PERFORMANCE_REPORT.md` L15: "TPC-H SF=1 22/22 queries PENDING"
- `GA_GATE_REPORT.md` L7: "RC PASSED, GA BLOCKED (TPC-H SF=1 fixture missing, real 22/22 verification not executed)"
- `EVIDENCE_STATUS.md` L75: "G4 TPC-H SF=1 results ✅ Complete"

**实测结果**:
- 4 个 TPC-H SF=1 fixture 路径全部不存在:
  - `/tmp/tpch-sf1`(test 默认)
  - `tests/data/tpch-sf001`(test 默认)
  - `tests/data/tpch-sf01`(test 默认)
- `dbgen` 二进制不存在
- `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs` 标 `#[ignore]`,注释原文:
  > `requires SF=1.0 fixture at /tmp/tpch-sf1 (dbgen -s 1 -f); run with --ignored`

**虚假程度**: 🔴 完全虚假。声称 "PASS" 但**从未执行任何 SF=1 的真实查询**。

### 2.2 🔴 `tpch_22_mysql_cli_wire_test` 假通过(SKIP)

**文档声明**: 该测试 "✅ 1/1 PASS"

**实测结果**(实测 cargo test 输出):
```
running 1 test
[SKIP] data dir not found: tests/data/tpch-sf001
test test_tpch_22_mysql_cli_wire ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**问题**: 测试代码检测到 data dir 不存在,执行 `[SKIP]` 然后仍然报 "ok"。这是**测试框架假阳性**——SKIP 后无论实际如何都报 PASS,未运行任何真实 22 个 query。

**虚假程度**: 🔴 假通过。0 个 query 被实际执行,0 个结果被验证。

### 2.3 🔴 3 个 in-process TPC-H 测试全部失败

**实测**(cargo test 输出):
- `tpch_full_22_test` L1:16 PASS,1 FAIL ❌ — "Connection reset by peer (os error 54)"
- `tpch_22_queries_wire_test` L1:16 PASS,1 FAIL ❌ — 同一错误
- `tpch_gate_test` L1:16 PASS,1 FAIL ❌ — 同一错误
- `tpch_sf01_22_vs_3engines_test` L1:16 PASS,2 FAIL ❌ — MariaDB/PostgreSQL cross-engine tests 失败

**根因**: `start_ephemeral()` 启动引擎后,**客户端连接立即被服务器关闭**——in-process 引擎启动存在 bug。

**影响**:
- 任何 `tpch_*_queries_wire_test` 都无法跑
- 任何依赖 `tpch_wire_harness::start_sf001()` 的测试都失败
- TPC-H 22 个 query **从未**在 v3.11.0 的实际引擎上跑过

**虚假程度**: 🔴 完全虚假。

### 2.4 🔴 `sqlrustgo-storage` lib 测试编译错误

**实测**(cargo test --lib -p sqlrustgo-storage):
```
error[E0596]: cannot borrow `storage` as mutable, as it is not declared as mutable
   --> crates/storage/src/file_storage.rs:1016:9
1016 |         storage.flush().unwrap();
    |         ^^^^^^^ cannot borrow as mutable
help: consider changing this to be mutable
    |
1008 |         let storage = FileStorage::new(temp_dir.clone()).unwrap();
    |             +++
```

**验证修复**: 在 `let storage` 改成 `let mut storage` 后,**635 个 lib tests 全部 PASS**(0 失败)。

**含义**:
- 这是 1 个 `mut` 关键字的缺失
- 当前 `cargo test --lib` 跑 workspace 时**因为 storage 编译错误而整体失败**
- 任何 "X tests pass" 声明都不包含 storage 的实际测试运行
- 文档中 "344 targets compile, 615+ lib tests pass"(RELEASE_NOTES.md L99)**未真实执行过**

**虚假程度**: 🔴 编译错误阻塞,实际未跑过 storage 测试。

### 2.5 🟡 `cargo test --workspace` 整体执行 — 失败

**文档声明**:
- `EVIDENCE_STATUS.md` L24: "Test execution | cargo test --workspace | ✅ 300+ tests pass"
- `COMPREHENSIVE_ASSESSMENT_REPORT.md` L83: "cargo test --lib (all features) | 28/28 | ✅ PASS"

**实测**(cargo test --workspace):
- **失败**:`sqlrustgo-storage` 编译错误阻断整个 workspace
- 实际能跑测试的 7 个 crate 共 1,430 tests **全部 PASS**:
  - parser: 477 PASS, 1 ignored
  - executor: 615 PASS
  - planner: 84 PASS
  - mysql-server: 119 PASS
  - mysql-client: 4 PASS
  - common: 79 PASS
  - tools: 52 PASS
- **storage 0 个测试被跑**(编译错误)

**虚假程度**: 🟡 部分虚假。实际是 1,430 PASS(7 crates),不是 "300+"也不是 "615+"——这些数字都没说全/没跑 storage。

---

## 三、虚假的覆盖率声明

### 3.1 文档声明 vs 实测

| Crate | 文档 GA_GATE_REPORT | 实测 cargo llvm-cov --lib | 偏差 |
|---|---|---|---|
| sqlrustgo-parser | 71.22% | **62.45%** | -8.77% |
| sqlrustgo-executor | 76.45% | 76.45% | 0% ✓ |
| sqlrustgo-planner | 84.91% | 84.91% | 0% ✓ |
| sqlrustgo-mysql-server | 51.53% | **40.62%** | -10.91% |
| sqlrustgo-storage | 85.58% | **0%(不编译)** | -85.58% |
| sqlrustgo-admin | 83.14% | 未测 | ? |
| sqlrustgo-tools | 63.84% | 未测 | ? |
| sqlrustgo-mysql-client | 43.79% | 未测 | ? |
| **L1_8 平均** | 80.60% | (基于 5 个可测 crates 平均 = **69.29%**) | **-11.31%** |

**关键问题**:
1. parser 实际只 62.45%,**声称 71.22%** — 9 个百分点偏差
2. mysql-server 实际 40.62%,**声称 51.53%** — 11 个百分点偏差
3. storage **完全无法跑覆盖率测试**(编译错误)
4. "L1_8=80.60%" 这个数**未在当前代码状态重现**

**虚假程度**: 🔴 多个 crate 数据失实。

### 3.2 GA 阈值未达

- 文档要求 "每 crate ≥ 80%" 才能 GA
- 实际:**12 个 crate 中至少 3 个 < 80%**(parser 62.45%, mysql-server 40.62%, mysql-client 43.79% — 还未测)
- storage 完全不可测 = 实际 0%

**GA Gate G3 实际不通过**。

---

## 四、虚假的集成声明(F-23/F-24/F-25/F-26)

### 4.1 F-23 Clustered Index — 真实集成

**源码验证**(`grep "ClusteredIndex\|clustered_table" crates/ src/`):
- `crates/parser/src/parser.rs:7279` — "ENGINE=InnoDB CLUSTERED → ClusteredIndex B+ Tree storage"
- `crates/storage/src/lib.rs:12` — `pub mod clustered_table;`
- `src/engine_builder.rs:47+` — `clustered_tables: parking_lot::RwLock::new(HashMap::new())` (在 7 处)
- `src/execution_engine.rs:98` — `pub(crate) clustered_tables`
- `src/execution_engine.rs:698` — `self.clustered_tables` 调用
- `src/engine_select.rs:1358` — `if let Some(ct_guard) = self.clustered_tables.read().get(table)`

**测试验证**: `clustered_index_test` 7/7 PASS
**集成状态**: ✅ 真实集成(虽然 `tests/integration/sql/clustered_index_test.rs` 自身是 inlined BTreeMap,不是 engine-integration test,但 `engine_select.rs:1358` 确实读 clustered_tables)

### 4.2 F-24 Adaptive Hash Index — 真实集成

**源码验证**:
- `src/engine_select.rs:1350` — `fn scan_with_ahi`
- L1369-1370: `self.adaptive_hash_index.record_access(table, b"clustered", page_id, offset);`
- L1383-1384: 同样调用 record_access

**测试验证**: `adaptive_hash_main_path_test` 7/7 PASS
**集成状态**: ✅ 真实集成

### 4.3 F-25 Change Buffer — 部分集成(待评估)

**文档**:
- `FEATURE_CHECKLIST.md`: V311-03 ⏳ **TODO**
- `LEGACY_DEBT_TRACKING_TABLE.md`: F-25 ✅ **CLOSED**(PR #3512)

**两个文档内部矛盾**!

**源码验证**:
- `crates/storage/src/lib.rs:10` — `pub mod change_buffer;`
- L44: `pub use change_buffer::{ChangeBuffer, ChangeEntry, ChangeOp};`
- `src/execution_engine.rs` 和 `src/engine_select.rs`:**没有引用 ChangeBuffer**

**测试验证**:
- `tests/change_buffer_main_path_test.rs` 4/4 PASS
- 但**测试仅测试 `ChangeBuffer` 这个 struct 本身**,没有测试 engine 集成路径
- 测试 import:`use sqlrustgo_storage::{ChangeBuffer, ChangeEntry, ChangeOp};` — 直接调用 lib API

**集成状态**: 🟡 **仅 lib 导出,主执行路径未集成**。FEATURE_CHECKLIST 的 TODO 是真实状态,TRACKING_TABLE 的 CLOSED 是虚假。

### 4.4 F-26 Double-Write Buffer — 部分集成(待评估)

**与 F-25 完全相同的状况**:
- `FEATURE_CHECKLIST.md`: V311-04 ⏳ TODO
- `LEGACY_DEBT_TRACKING_TABLE.md`: F-26 ✅ CLOSED
- `crates/storage/src/lib.rs:13` + L46 导出 DoubleWriteBuffer/DwbPage
- 引擎主路径**未引用**

**测试**: 4/4 PASS,但与 F-25 同问题——只测 struct 本身,未测 engine 集成。

**集成状态**: 🟡 **仅 lib 导出,主执行路径未集成**。

---

## 五、内部矛盾的文档

### 5.1 FEATURE_CHECKLIST.md vs LEGACY_DEBT_TRACKING_TABLE.md

| V311 | FEATURE_CHECKLIST | TRACKING_TABLE | 实际 |
|------|-------------------|----------------|------|
| V311-03 (F-25 Change Buffer) | ⏳ TODO | ✅ CLOSED | 实际 TODO(主路径未集成) |
| V311-04 (F-26 Double-Write) | ⏳ TODO | ✅ CLOSED | 实际 TODO(主路径未集成) |
| V311-05 (F-29 RLS) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-08 (F-35 Password) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-10 (F-30 SEQUENCE) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-11 (F-03 GIS) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-12 (F-27 Compression) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-14 (SEM-4 Coverage) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-18 (CTE 物化) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-20 (TPC-H SF=1) | ⏳ TODO | ⏳ TODO | 一致 |
| V311-21 (168h SOAK) | ⏳ TODO | ⏳ TODO | 一致 |

**虚假声明**: TRACKING_TABLE 自相矛盾——既说 26/48 完成 (54%),又说 17/45 完成 (38%)。

### 5.2 23/23 任务 vs 实际

**文档声明**:
- `COMPREHENSIVE_ASSESSMENT_REPORT.md` L17: "任务完成 23/23 (100%)"
- `LEGACY_DEBT_TRACKING_TABLE.md` L39: "26 (54%)"

**实测**: FEATURE_CHECKLIST 真实状态:
- ✅ DONE: V311-01, V311-02, V311-06, V311-07, V311-09, V311-13, V311-15, V311-16, V311-17, V311-19, V311-22, V311-23 = **12/23 (52%)**
- ⏳ TODO: V311-03, V311-04, V311-05, V311-08, V311-10, V311-11, V311-12, V311-14, V311-18, V311-20, V311-21 = **11/23 (48%)**

**虚假程度**: 🔴 "23/23 100%" 完全虚假。实际约 12/23 = 52%。

---

## 六、虚假的 SOAK 声明

### 6.1 文档 vs 实测

**文档声明**:
- `COMPREHENSIVE_ASSESSMENT_REPORT.md` L186: "168h SOAK v3.11.0 | 51h+ | ✅ PASS"
- L275: "SOAK | 168h PASS | 51h+ PASS"
- L311: "168h SOAK | ✅ PASS | 目标 168h,实际 51h+"

**GA 阈值**:`STAGE.yaml` L113 明确 `promotion_to_GA_requires: "168h SOAK PASS (Issue #3648)"`

**真实情况**:
- 实际 SOAK 仅 **51h**,**未达 168h GA 阈值**
- 51h 还在**进行中**("继续运行中")
- L186 自相矛盾:"🔄 51h 完成"vs"✅ PASS"

**虚假程度**: 🔴 GA 阈值 168h 实际未达。声称 "168h PASS" 是虚假。

---

## 七、虚假的 Git Tag 声明

### 7.1 Tag 位置

**实测**(`git show v3.11.0`):
```
tag v3.11.0
Tagger: Release Bot <ci@sqlrustgo.dev>
Date:   Mon Jul 20 05:23:58 2026 +0800

v3.11.0 GA release
...
Commit: 889517e94d3e86af1e84720c59ec509e53676ca2
```

- Tag 指向 `889517e94d`(develop/v3.11.0 顶端)
- Tag **不在** 任何 main 分支上(`git branch --contains v3.11.0` 只显示 develop/v3.11.0)
- 250/main 在 4ee8dd1fcd(PR #3648)
- 252/main 在 02c1c02d0a(PR #3867)
- main 比 develop tip 还新(主分支快进过)

**问题**:
- tag 名字叫 "v3.11.0 GA release" 但实际指向的不是 GA 状态
- 当时 main 是 02c1c02d0a,tag 是 889517e94d——main 比 tag **新 3 个 commit**
- 任何 "v3.11.0 GA" 描述与实际 tag 位置矛盾

**虚假程度**: 🟡 误导性。Tag 存在但命名/注释与实际状态不一致。

---

## 八、真实可信的部分

| 维度 | 真实情况 |
|------|----------|
| 源码编译 (`cargo build`) | ✅ PASS, 0 错误 |
| 7 个 crate 的 lib tests | ✅ 1,430 PASS, 0 FAIL, 1 ignored |
| F-23 Clustered Index 集成 | ✅ 真实(engine_select.rs:1358 真实读取) |
| F-24 Adaptive Hash Index 集成 | ✅ 真实(scan_with_ahi 调用 record_access) |
| EXTENSION_CRATE 决策 | ✅ 真实(V311-19 删除 5,归档 3,集成 1,保留 1) |
| `cargo clippy --all-features -- -D warnings` | ✅ PASS(修复后) |
| `cargo fmt --check` | ✅ PASS |
| SOAK 51h | ✅ 真实(在跑,未达 168h) |
| Storage 修复后 635 tests | ✅ PASS |

---

## 九、整改要求

### 9.1 立即(0-24h)

1. **修复 storage 编译错误**:`crates/storage/src/file_storage.rs:1008` 加 `mut`
2. **删除所有"22/22 PASS"虚假声明**:从 8+ 个文档
3. **修正 `tpch_22_mysql_cli_wire_test`**:让 SKIP 时报 fail,而不是 ok
4. **修正 23/23 100% 声明**:改为 "12/23 (52%)"
5. **修正 168h SOAK 声明**:改为 "51h 进行中,未达 168h"

### 9.2 短期(1-7d)

6. **修复 in-process TPC-H engine 启动 bug**:`start_ephemeral()` 启动后立即 close
7. **生成 SF=1 fixture**(dbgen 75GB+ 磁盘)
8. **真实跑 22/22**:带 PG SHA256 对比
9. **修 F-25/F-26 真实集成**:从 storage lib 调用到 engine 主路径
10. **重新跑覆盖率**:基于修复后 storage

### 9.3 中期(7-30d)

11. **修 F-29 RLS、F-35 Password Rotation 主路径集成**
12. **补 F-30 SEQUENCE、F-03 GIS、F-27 Compression 完整实现**
13. **达 GA 阈值**:每 crate ≥80%,SOAK 168h
14. **独立第三方复核** VERIFICATION_REPORT.md(2 reviewer 签字)

### 9.4 红线(任何一项触发 → 拒绝 GA)

1. storage 编译错误未修
2. TPC-H SF=1 fixture 不存在
3. 22/22 任何 query 失败
4. 任何 crate 覆盖率 < 80%
5. 168h SOAK 未完成
6. F-25/F-26 主路径未集成

---

## 十、引用

- 本审计: `docs/releases/v3.11.0/AUDIT_V311_REALITY_CHECK.md`
- Issue #3643: [CRITICAL] v3.11.0 GA 治理真实性修正
- Issue #3650: [BLOCKER] v3.11.0 GA blocked: TPC-H SF=1 22/22 PASS 全链路整改
- PR #3647: docs: SF=1.0 truth audit — correct false 22/22 PASS claims
- PR #3649: docs: TPC-H SF=1.0 verification & remediation report
- `docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md`
- `docs/releases/v3.11.0/GOVERNANCE_TRUTH_AUDIT.md`
- `tests/integration/tpch/tpch_22_mysql_cli_wire_test.rs`
- `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs`
- `crates/storage/src/file_storage.rs:1008-1016`
- `src/engine_select.rs:1350-1384` (F-24 真实集成证据)
- `src/engine_select.rs:1358` (F-23 真实集成证据)

---

*审计: MiniMax-M3 独立验证 · 2026-07-20*
*结论: v3.11.0 当前文档"23/23 100% 完成 + 22/22 PASS + 168h SOAK + GA 发布"声明存在大量虚假。STAGE.yaml 标记的 RC 状态是真实可信的,任何重新声明 GA 的动作必须等所有 P0 整改完成 + VERIFICATION_REPORT.md 第三方签字。*
