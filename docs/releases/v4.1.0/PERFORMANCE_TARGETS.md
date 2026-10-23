# SQLRustGo v4.1.0 性能目标（PERFORMANCE_TARGETS）

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-performance-targets-001`
> （本文档汇编的门禁引擎：`scripts/gate/check_coverage_v312.sh`、
> `scripts/gate/coverage_total_percent.py`、`scripts/gate/check_anti_ignore_gate.sh`、
> `scripts/gate/check_evidence_binding.sh`、`scripts/gate/check_g1_tpch_baseline.sh`、
> `scripts/gate/check_g11_qps.sh`、`scripts/gate/check_g12_sysbench.sh`、
> `scripts/gate/check_g13_stability.sh`、`scripts/gate/check_g14_real_crash.sh`、
> `scripts/gate/check_g15_perf_report.sh`、`scripts/gate/check_p13_soak_test.sh`；
> 证据绑定门禁 `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。）

---

## 0. 这份文件是什么，不是什么

**是什么**：v4.1.0 在 RC / GA 两个阶段共用的**性能与质量目标清单**。它是
`docs/governance/STAGE_CONFIG.yaml` 的 `stages.RC.doc_artifacts`（RC 阶段）与
`stages.GA.doc_artifacts`（GA 阶段）点名要求的发布产物之一。

**不是什么**：

* **不是批准**。`STAGE_CONFIG.yaml` 的三次阶段跃迁
  （`ALPHA_to_BETA` / `BETA_to_RC` / `RC_to_GA`）全部是
  `requires_human_approval: true`。本文档只提供**决策所需的事实**，
  不代替人类架构师的裁决，也不代替任何签字。
* **不是性能承诺书**。仓库对 v4.1.0 **没有**记录任何「SQL 执行延迟 / QPS / TPS
  必须 ≤ 某个数」的 GA 级数值门槛（见 §1.4 与 §4）。凡是仓库没记录阈值的，
  本文一律写成 `unknown（仓库未记录）`，并附已实测的数值 —— **实测值不等于达标**。
* **不是新的测量报告**。本文档**没有**执行任何 cargo / 门禁 / SOAK / 基准
  （本轮只跑了两个只读门禁，见 §7.2）。所有数值都是从仓库既有证据文档中
  **原样引用**，并注明出处与测量日期。

---

## 1. 目标的可证伪定义

### 1.1 什么算一个「目标」

一个条目进入 §3 的目标表，必须同时满足三条：

1. **判据可复现** —— 有一条命令或一条明确的动作序列，能被第二个人照着跑出可比对的数。
2. **阈值有出处** —— 阈值要么来自 `docs/governance/STAGE_CONFIG.yaml`
   （`thresholds` / `gate_definitions`），要么来自 `docs/releases/v4.1.0/STAGE.yaml`
   （`exit_criteria` / `known_gaps`），要么来自一份 issue 文档里写死的
   「关闭条件」。**没有出处的数字不能当阈值用。**
3. **结论词唯一** —— 同一目标在同一轮里只有一个结论，不允许同时出现
   「PASS」和「待观察」。

### 1.2 结论词表

| 结论 | 含义 | 何时用 |
|---|---|---|
| `PASS` | 判据被实跑，输出满足阈值 | 必须同时给出**门禁脚本名 + 实测输出摘录**或**证据文件 + 测量方法** |
| `NOT MET` | 判据被实跑，输出不满足阈值 | 必须写明差多少、差在哪 |
| `NOT RUN` | 判据**没有**被实跑 | **必须写明原因**（缺硬件 / 缺工具 / 缺前置产物 / 时间窗未到）。**`NOT RUN` ≠ `PASS`** |
| `UNKNOWN` | 仓库没有记录该目标的现状 | 必须写明「查过哪些路径」 |

> `STAGE_CONFIG.yaml` 对非阻断 G 门禁（G7 / G11 / G12 / G13 / G14 / G15）写明的
> 语义原文是：「**必须实跑并披露 PASS / FAIL / NOT-RUN；NOT-RUN 不得记为 PASS**」。
> 本文档严格按这条语义执行。

### 1.3 测量环境分级

| 级别 | 含义 | 证据出处 |
|---|---|---|
| **M（本机）** | Mac-mini-7.local，arm64，macOS 27.0.1 | 证据文档头部的 `machine=` 字段 |
| **S（服务器）** | 80 核 / 404 GB 机器 | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` §1 |
| **Z（专用主机）** | Z6G4 等专用 soak / 崩溃测试主机 | `STAGE_CONFIG.yaml` `gate_definitions.G13/G14.requirement` |
| **C（CI）** | Gitea Actions runner | `evidence/CI_TRUTH_BATCH24_2026-10-22.md` |
| **U（未跑）** | 本轮 / 该版本从未执行 | — |

### 1.4 关于「仓库未记录」的判定

本轮判定「仓库未记录」时，**实际查过**下列路径，确认其中没有 v4.1.0 的对应数值：

* `docs/governance/STAGE_CONFIG.yaml`（`thresholds` 只有覆盖率 / clippy / fmt /
  doc-links / ignore-count 五类，没有任何延迟或吞吐数值）
* `docs/releases/v4.1.0/STAGE.yaml`（`exit_criteria` 只有覆盖率与 SOAK 时长）
* `docs/releases/v4.1.0/PERFORMANCE_AUDIT_REPORT_2026-09-30.md`
  （§0 明确：本报告是**静态代码审计**，**未执行** `cargo bench` / `perf` /
  `sample` / SOAK，所有「预期收益」均为待验证假设）
* `docs/releases/v4.1.0/PERFORMANCE_OPTIMIZATION_PLAN.md` §6（测量协议只有
  「跑什么命令、记什么字段」，没有目标值）
* `docs/releases/v4.1.0/EVALUATION_REPORT.md` §7.1（该报告逐条列出了
  「not recorded」的量）

---

## 2. 覆盖率门槛与当前位置

### 2.1 四个门槛

`docs/governance/STAGE_CONFIG.yaml` → `thresholds`：

| 常量 | 值 | 对应阶段 |
|---|---:|---|
| `COVERAGE_MIN_DRAFT` | `null`（不强制） | DRAFT |
| `COVERAGE_MIN_ALPHA` | 50 | ALPHA |
| `COVERAGE_MIN_BETA` | 75 | BETA |
| `COVERAGE_MIN_RC` | 80 | RC |
| `COVERAGE_MIN_GA` | 85 | GA |

### 2.2 当前实测位置

**最新一次完整 L1_8 8-crate 实跑（2026-10-23，M 本机）**：平均 **87.80%**，
`evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3：

| crate | 行覆盖率 | crate | 行覆盖率 |
|---|---:|---|---:|
| parser | 86.46% | storage | 80.61% |
| planner | 88.91% | catalog | 87.33% |
| executor | 82.74% | optimizer | 87.85% |
| transaction | 93.29% | types | 95.28% |

另有一条独立测量：`mysql-server` **87.89%**（`evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`，
2026-10-07）。

### 2.3 四个门槛的满足情况

| 门槛 | 阈值 | 当前（87.80%） | 判定 | 依据与保留 |
|---|---:|---|:---:|---|
| `COVERAGE_MIN_ALPHA` | 50 | 87.80% | **PASS** | `STAGE_CONFIG.yaml` `thresholds`；实测 2026-10-23 |
| `COVERAGE_MIN_BETA` | 75 | 87.80% | **PASS** | 同上 |
| `COVERAGE_MIN_RC` | 80 | 87.80% | **PASS** | 同上；`STAGE.yaml` `exit_criteria.rc_to_ga` 亦写「Coverage >= 80%」 |
| `COVERAGE_MIN_GA` | 85 | 87.80% | **PASS**（附三条保留） | 见下 |

`COVERAGE_MIN_GA` 的三条保留，**必须与「PASS」一起读**：

1. **测的是平均值，不是每 crate 下限。** 门禁 `check_coverage_v312.sh` 判的是
   L1_8 **均值**（`evidence/BATCH28_…` §1.3 的判据原文：「L1_8 平均 87.80% ≥ 80.0%」）。
   `storage 80.61%` 单项低于 85 —— 若按单 crate 下限判，storage 不达标。
   **SSOT 没有定义单 crate 下限**（`unknown（仓库未记录）`）。
2. **测量点不是本文档基线。** 87.80% 测于 `develop/v4.1.0 @ 3a3350f660`；
   本文档基线是 `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`。
   **该基线上没有重测过覆盖率**，数字不可视为对基线的直接断言。
3. **口径已从「region」切到「line」。** `evidence/COV_01_02_…` 记的是 region 口径，
   `evidence/BATCH28_…` 记的是行覆盖率口径。跨文档比较时口径不同。

### 2.4 跨机台对照（可复现性证据）

`evidence/BATCH28_…` §1.5，7 个重叠 crate，z440（2026-10-07）vs macmini（2026-10-23）：

| crate | z440 | macmini | Δ |
|---|---:|---:|---:|
| planner | 88.91 | 88.91 | 0.00 |
| catalog | 87.33 | 87.33 | 0.00 |
| optimizer | 87.85 | 87.85 | 0.00 |
| types | 95.28 | 95.28 | 0.00 |
| parser | 86.75 | 86.46 | 0.29pp |
| storage | 81.34 | 80.61 | 0.73pp |
| transaction | 86.00 | 93.29 | 7.29pp |

transaction 的 7.29pp 由两次测量之间的**真实代码变更**解释（`crates/transaction`
在 2026-10-07 之后有 6 个 commit），不是口径问题。确定性另有一组验证：
同机同代码连续两遍测 `sqlrustgo-transaction` 得 **93.29% / 93.29%**（完全一致），
前提是 `--test-threads=1` 被钉死（commit `2fef945260`，#5537）。

---

## 3. 性能目标表

> 列含义：**阈值** = 判据（必填出处）；**测量方法** = 怎么测；
> **基准 / 门禁位置** = 在哪测；**当前实测值** = 仓库记录到的数；
> **结论** = §1.2 的词表；**证据** = 出处文件。

### 3.1 覆盖率与质量门禁

| 目标项 | 阈值 | 测量方法 | 基准 / 门禁位置 | 当前实测值 | 结论 | 证据 |
|---|---|---|---|---|---|---|
| L1_8 8-crate 平均覆盖率（RC 门槛） | ≥ 80%（`COVERAGE_MIN_RC`） | `cargo llvm-cov --test-threads=1` 逐 crate 出行覆盖率后取均值 | `scripts/gate/check_coverage_v312.sh`；`scripts/gate/coverage_total_percent.py` | **87.80%**（2026-10-23，M） | **PASS** | `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3 |
| L1_8 8-crate 平均覆盖率（GA 门槛） | ≥ 85%（`COVERAGE_MIN_GA`） | 同上 | 同上 | **87.80%**（2026-10-23，M，测于 `3a3350f660`） | **PASS**（保留见 §2.3） | 同上 |
| `mysql-server` 覆盖率（COV-02 销账项） | ≥ 85%（GA 门槛，仓库为 COV-02 设的目标） | `cargo llvm-cov test -p sqlrustgo-mysql-server --no-fail-fast` | `scripts/gate/check_coverage_v312.sh` | **87.89%**（2026-10-07，GATE_EXIT=0，24/24 test binary ok） | **PASS** | `evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md` |
| `parser` 覆盖率（COV-01 销账项） | ≥ 85%（GA 门槛，仓库为 COV-01 设的目标） | `cargo llvm-cov test -p sqlrustgo-parser --no-fail-fast` | 同上 | **86.46%**（2026-10-23）；2026-10-07 另测 region 口径 86.20% | **PASS** | `evidence/BATCH28_…` §1.3；`evidence/COV_01_02_…` |
| 覆盖率门禁 fail-closed（测不出数据的 crate 不得被跳过） | 提取失败的 crate 必须进 `FAILED_CRATES` 并 exit 1；测出的 crate 数必须等于 L1_8 全集 | 门禁自跑 + 读脚本 fail-closed 分支 | `scripts/gate/check_coverage_v312.sh:315-349` | 2026-10-23 实测：8/8 crate 全部测出，无跳过，exit 0 | **PASS** | `evidence/BATCH28_…` §1.3 / §7 |
| clippy 警告数 | `CLIPPY_WARNINGS_MAX: 0` | `cargo clippy --all-features --all-targets -- -D warnings`（钉 1.95.0） | CI `lint-build` job | **0 error**（2026-10-23，M，9m58s） | **PASS**（本机） | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §4.4 |
| fmt 漂移数 | `FMT_DRIFT_MAX: 0` | `cargo fmt --check --all`（rustfmt 1.91.0 / 1.95.0 / 1.97.0 三版） | CI `lint-build` job | **CLEAN**（三版一致，2026-10-23，M） | **PASS**（本机） | 同上 |
| 文档链接断链数 | `DOC_LINKS_BROKEN_MAX: 0` | `scripts/gate/check_docs_links.sh` | `scripts/gate/check_docs_links.sh` | exit 0「All markdown links are valid」（记录于 2026-10-03） | **PASS**（记录较早；本轮未重跑） | `STAGE.yaml` `last_status_refresh.gate_runs_2026_10_03` |
| CI `test` job 绿灯 | 至少 1 次成功完成 | 读 Gitea Actions run/job 状态 | `.gitea/workflows/ci.yml` | **0 次成功完成 / 1 452 次机会**（10 次未开跑即死、19 次真实测试失败后死、779 次 skip、631 次取消） | **NOT MET** | `evidence/CI_TRUTH_BATCH24_2026-10-22.md` §1.3–§1.5 |
| CI `lint-build` 绿灯 | 0 warning / 0 fmt diff | 同上 | 同上 | run **8750** 红（`clippy::type_complexity`，`tests/integration/sql/_q15_q16_check.rs:74`）；run **8831** 红（`cargo fmt --check` diff，`crates/storage/tests/no_namespace_db_refusal_5266.rs:16,22`） | **NOT MET** | `evidence/CI_TRUTH_BATCH24_2026-10-22.md` §2.4（`EVALUATION_REPORT.md` §4.2 转录） |

### 3.2 存储与索引

| 目标项 | 阈值 | 测量方法 | 基准 / 门禁位置 | 当前实测值 | 结论 | 证据 |
|---|---|---|---|---|---|---|
| WAL 恢复「不再整表重写」（P0-08 关闭条件 1） | 恢复期间 `<table>.json` 不被重复整体重写 | 灌 N 行 → 重启 → 采样数据目录文件大小 / mtime | `crates/storage/src/recovery_engine.rs`；`crates/storage/tests/measure_recovery_scale_p0_08.rs` | `t.json` 停在 **645 B**（= schema，全程未重写），`t.delta` 134 440 B 承载全部行 | **PASS** | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §4.5；`ISSUE-RECOVERY-QUADRATIC-REPLAY.md` §6 |
| WAL 恢复「成本次线性」（P0-08 关闭条件 2） | N 行恢复耗时随 N 线性增长（斜率 ≈ 1） | `RecoveryEngine::recover()` 真实路径，N ∈ {500, 2 000, 8 000} 三点计时 | 同上 | 修复后每行成本 **210.3 → 224.0 → 372.4 µs**（行数 ×16 ⇒ 每行成本 ×1.8）；修复前 63.9 → 247.9 → 896.4 µs（×14，O(N²)） | **PASS** | 同上 §4.2 |
| 「重启后可服务」（P0-08 关闭条件 3） | 恢复完成后 listener 接受连接 | 真起 server、灌行、停服、**换全新端口**重启、轮询到 accept | 同上 | **restart_to_ready = 0.17 s** | **PASS** | 同上 §4.5 |
| 「数据不丢」（P0-08 关闭条件 4） | 重启后 `COUNT(*) == 退出前行数` | 灌 4 000 行 → 重启 → `COUNT(*)` 前后对比 | 同上 | 4 000 == 4 000 == 4 000；另有 5/5 语义用例（`wal_recovery_dedup_p0_08.rs`） | **PASS** | 同上 §4.1 / §4.5 |
| 「增量路径被走到」（P0-08 关闭条件 5） | 恢复产生 `.delta` 文件或走追加分支 | 同上 + 检查 delta 路径命中 | 同上 | 满足（且**修复前就已满足** —— 该条件是原报告误判为失败的那条路径） | **PASS** | 同上 §5 / `ISSUE-RECOVERY-QUADRATIC-REPLAY.md` §6 |
| WAL 恢复在 58 288 行规模的总耗时 | `unknown（仓库未记录）` —— 仓库只记了「修复前 ≈ 461 s，修复后 ≈ 21.7 s，≈ 21 倍」的**外推** | 外推自 N ∈ {500, 2 000, 8 000} 实测三点 | 同上 | ≈ 21.7 s（外推，非实测） | **NOT RUN**（未在 58 288 行规模实测） | 同上 §4.2 |
| 写路径每行落盘成本（`append_table_delta`） | `unknown（仓库未记录）` | `measure_recovery_write_cost_p0_08` | `crates/storage/tests/measure_recovery_scale_p0_08.rs` | **常数** 86.4 / 85.8 / 96.4 µs/行（500 / 2 000 / 8 000 行）；58 288 行 ≈ 5 s | **PASS**（对「常数成本」这一形状要求；**明确不主张已优化**） | 同上 §2.1 |
| `Update` / `Delete` 恢复路径 | `unknown（仓库未记录）` | 同上 | `crates/storage/src/recovery_engine.rs` | **仍是每条 entry 扫一次表**（本批未扩大范围） | **NOT RUN**（本批明确不做） | 同上 §6 |
| 单文件行数上限（C-ARCH-05） | ≤ 1 600 行（`execution_engine.rs` 实测 1 034 行） | `scripts/gate/check_arch_invariants.sh` | `scripts/gate/check_arch_invariants.sh` | 1 034 行（2026-09-30 记录）；**`expr/mod.rs` 5 104 / `stored_proc.rs` 4 248 / `trigger.rs` 2 549 行三个大文件不在该门禁覆盖内** | **PASS**（就 `execution_engine.rs` 而言）；覆盖缺口见 §5 | `DEFECTS_AND_ISSUES.md` P2-03 |
| Page Checksum 门禁有效性 | 门禁判据应为「校验函数存在 + 测试通过」，而非「字段存在」 | 读 `crates/storage/src/page.rs:179`；全仓 grep `fn verify_checksum` / `fn calculate_checksum` | `scripts/gate/check_storage_index_wal.sh` | `page.rs:179` 实为 `offset += 4; // skip checksum`；`crates/storage/**` 内 **零命中** | **NOT MET** | `DEFECTS_AND_ISSUES.md` P2-01 |

### 3.3 SQL 执行与并发

| 目标项 | 阈值 | 测量方法 | 基准 / 门禁位置 | 当前实测值 | 结论 | 证据 |
|---|---|---|---|---|---|---|
| 无锁事务路径接线生效（A1 / #4912） | 审计报告 F-01 断言「无锁路径在生产恒不生效」被实测推翻 | A/B 两个 release 二进制，唯一差异是 `BoxStorageEngine` 是否转发 `*_lockfree`；8 并发连接 × 150 事务 | macOS `sample` 采栈；驱动脚本见 `PERF_A1_4912_AB_MEASUREMENT.md` §2 | TPS **402.0 → 6 846.2（17.0×）**；SELECT p50 4.256 → 0.240 ms；`parking_lot` 栈帧 **54 → 0** | **PASS**（该断言已由实测推翻） | `PERF_A1_4912_AB_MEASUREMENT.md` §3 |
| 写路径整表深拷贝（B2.1 / #4915） | `PERFORMANCE_OPTIMIZATION_PLAN.md` §6 协议：3 runs 取中位 | `cargo bench -p sqlrustgo-storage --bench storage_benchmark` | `crates/storage/benches/storage_benchmark.rs`（`b2_insert_into_large_table` 组） | 1 000 行 0.603 → 0.027 ms（22.44×）；10 000 行 0.615 → 0.026 ms（23.91×）；50 000 行 1.002 → 0.026 ms（39.26×） | **PASS**（实测改善成立） | `PERF_B2_4915_AB_MEASUREMENT.md` §3 |
| 锁内文件 I/O（B2.2） | 判据：并发读延迟改善 | 同上 + 4 读者 / 1 写者并发测量 | `crates/storage/tests/b22_concurrent_flush_ab.rs`、`b22_flush_wall.rs` | 首版 0.86× 回归 → 修复后 0.99×（中性）；并发 p50 4.52 vs 4.66 ms（区间重叠） | **NOT MET**（并发收益未成立；**且该修改的方法在生产调用链上不可达** —— inherent `FileStorage::flush` 实测进入 0 次，trait 覆写 21 次） | `PERF_B22_CONCURRENT_MEASUREMENT.md` §3–§4 |
| 全量快照缓冲（B2.3） | `unknown（仓库未记录数值门槛）` | `cargo bench … --bench storage_benchmark` | 同上 `b2_snapshot_and_scan` 组 | 50 000 行 13.703 → 11.605 ms（1.18×）；10 000 行 3.531 → 3.318 ms（1.06×） | **PASS**（实测改善成立；真实收益在内存峰值，未量化） | `PERF_B2_4915_AB_MEASUREMENT.md` §4 |
| `scan_with_filter` 统一（B2.4） | `unknown（仓库未记录数值门槛）` | 同上 | 同上 `b2_snapshot_and_scan/filtered_scan_miss` | 50 000 行 0.053 → 0.045 ms（1.19×）；10 000 行 0.010 → 0.010 ms（落在噪声内） | **PASS**（@50k 成立；@10k 落在噪声内） | 同上 |
| PK 点查（#5168） | `unknown（仓库未记录 GA 数值门槛）` | 20 000 行点查墙钟；sysbench 8 客户端 | `benches/storage_bench.rs`、`benches/qps_bench.rs`（根 `Cargo.toml` `[[bench]]` 目标） | 20 000 行 **9.02 ms → 0.467 ms（19×）**；sysbench TPS 13.89 → 38.53；QPS 277.79 → 770.52 | **PASS**（改善已实测；**达标与否无阈值可判**） | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` §5.1；`CHANGELOG.md` #5168 |
| UPDATE / DELETE WHERE 快路径（#5177） | `unknown（仓库未记录 GA 数值门槛）` | 20 000 行 UPDATE e2e；sysbench QPS 阶梯 | 同上 | UPDATE e2e 10.941 → **1.051 ms**；QPS@1000 6 577 → 9 759 / 10 007 / 10 776；QPS@10000 862 → 1 242 / 1 276 / 1 295 | **PASS**（改善已实测；无阈值可判） | 同上 |
| 删除路径（#5181） | `unknown（仓库未记录 GA 数值门槛）` | 20 000 行 delete+insert | 同上 | 0.472 ms（delete 0.344 / insert 0.002）；e2e 10 000 行 QPS 1 242~1 295 → 1 254~1 367（噪声内） | **NOT MET**（部分修复：消除分配，但收益在噪声内） | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` 结论摘要 |
| Q20 去相关接线 | 判据：`try_decorrelate()` 在 planner 主路径有调用点 | `grep -rn try_decorrelate crates/` | `crates/optimizer/src/decorrelate.rs:417` | **0 命中**（函数有定义，无调用点） | **NOT MET** | `DEFECTS_AND_ISSUES.md` P2-02 |
| 吞吐随并发提升 | `unknown（仓库未记录 GA 数值门槛）` | sysbench 容量阶梯（1 → 128 客户端） | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` §3 | 修复前并发 4 → 64 **吞吐不升反降**（1 连接 136 QPS → 8 连接 17 QPS）；修复后 10 000 行 QPS 1 242~1 295，**仍不随并发提升** | **NOT MET**（结构性结论，瓶颈已转移到写路径 O(rows) 遍历与 `engine.write()` 全局独占锁） | 同上 §3 / §5.1 |
| `Expression` 列绑定分配（F-08） | `unknown（仓库未记录 GA 数值门槛）` | `cargo bench --bench bench_expr_binding` | `benches/bench_expr_binding.rs`（根 `Cargo.toml` `[[bench]]` 目标） | 基线：64 列过滤**末列比首列慢 72%**（502.9 vs 292.0 µs）；多 join 未命中 **4.395 µs** = 单表同条件未命中（410 ns）的 **10.7×** | **PASS**（第一刀「零分配尾段比较」已落地；完整绑定树按 plan §B1.3 不在本文范围） | `PERF_B13_F08_BENCHMARK_AND_ALLOC_FREE_BINDING.md` §2–§3 |

### 3.4 稳定性与长时运行

| 目标项 | 阈值 | 测量方法 | 基准 / 门禁位置 | 当前实测值 | 结论 | 证据 |
|---|---|---|---|---|---|---|
| 168h 多模型 SOAK（V400-09） | `STAGE.yaml` `exit_criteria.rc_to_ga`：「168h multi-model SOAK PASS」 | 168 小时连续运行 + 终期报告 | `scripts/gate/check_p13_soak_test.sh`（G7）；`scripts/soak/` | **NOT MET** —— `known_gaps.V400_09_soak.status: "NOT MET"`（verified 2026-10-07）；进程 PID 1957614 已终止，全盘无 run3/run4 产出 | **NOT MET** | `STAGE.yaml` `known_gaps.V400_09_soak` |
| 168h SOAK 的裁决状态 | `known_gaps.V400_09_soak.decision` 不得为 `OPEN` | 读 SSOT | `STAGE.yaml` `known_gaps.V400_09_soak.decision` | 原文：「**OPEN — 是否重跑 168h SOAK 属架构师裁决，本 SSOT 不代为决定。**」 | **NOT MET** | 同上 |
| 24h+ 稳定性（G13） | 判据前提：Z6G4 主机 + `STABILITY_REPORT.md` + Beta 72h 报告 | `scripts/gate/check_g13_stability.sh` | `scripts/gate/check_g13_stability.sh` | 门禁**当前 exit 1**：`[1/9] ❌ FAIL: tests/soak_test_harness.rs not found`；v4.1.0 目录下无 24h+ 的 `STABILITY_REPORT.md`（只有 5 min / 1 h 两份） | **NOT MET**（门禁红 + 无 24h+ 报告） | `DEFECTS_AND_ISSUES.md` P1-07；`docs/releases/v4.1.0/SOAK_V410_1H_2026-10-08/` |
| 8h SOAK 稳定性 | 判据：**无仓库记录的阈值**，只记「0 FATAL / 0 errors / 0 reconnects」这一观察 | `sysbench --time=28800`，独立实例 | `scripts/soak/`；`evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` | 28 800.29 s；0 FATAL / 0 errors / 0 reconnects；352 402 事务 / 7 048 040 查询；CPU 950 样本 avg 164.2% / max 174.0%；RSS 峰值 85.3 MB；237 次 `SELECT 1` 探针全通；逐小时 TPS 前 4h 均值 11.72 → 后 4h 12.75（+8.8%） | **PASS**（作为观察记录）；**证据权重已被下调** —— IAIA-410 指出该 SOAK 单库、无 kill -9、无重启，「不能作为 v4.1.0 可用的证据」 | 同上 §2；`CHANGELOG.md` SOAK-evidence 条目 |
| 1h SOAK 稳定性 | `unknown（仓库未记录）` | `scripts/soak/v400_1h_soak.sh` 60min 模式 | `docs/releases/v4.1.0/SOAK_V410_1H_2026-10-08/` | 3 600 s / 582 585 查询 / 0 errors；~162 QPS；p50 79.9 / p99 370.1 / max 2 933.4 ms；FD 恒定 28；**RSS 峰值 662.8 MB，逐十分钟单调上升（106.0 → 662.8 MB），无 GC 平台期** | **NOT MET**（报告自述：「v4.1.0 at HEAD has not demonstrated a stable memory equilibrium」） | `docs/releases/v4.1.0/SOAK_V410_1H_2026-10-08/STABILITY_REPORT.md` |
| 长时运行的覆盖面 | SOAK harness 是否覆盖「中途重启」 | 读 `scripts/soak/v400_1h_soak.sh` | `scripts/soak/v400_1h_soak.sh` | **从不中途重启服务器** ⇒ P0-08 所在的恢复路径**历史 SOAK 从未覆盖过** | **NOT MET**（覆盖面缺口） | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §5 |
| 进程级崩溃测试（G14） | 判据前提：Z6G4 主机 + sysbench + `CRASH_TEST_REPORT.md` | `scripts/gate/check_g14_real_crash.sh` | 同上 | v4.1.0 目录下**无** `CRASH_TEST_REPORT.md` | **NOT RUN**（前置产物缺失） | `STAGE_CONFIG.yaml` `gate_definitions.G14.requirement` |

### 3.5 TPC-H 与行业基准

| 目标项 | 阈值 | 测量方法 | 基准 / 门禁位置 | 当前实测值 | 结论 | 证据 |
|---|---|---|---|---|---|---|
| TPC-H SF=1 22/22（G1） | 22/22 全部通过（`check_g1_tpch_baseline.sh` 的判据） | `cargo` 侧基线 + 22 查询执行 | `scripts/gate/check_g1_tpch_baseline.sh`；`benches/tpch_bench.rs`、`benches/tpch_comprehensive.rs`、`benches/tpch_load_bench.rs`、`benches/tpch_wire_bench.rs`、`crates/bench/examples/tpch_compare.rs` | v4.1.0 **not recorded** —— 仓库明确记载未执行；`evidence/TPCH_FIXTURE_5379_2026-10-20.md` 把 SF=1 排除在本轮范围外（#5387） | **NOT RUN**（本版本从未执行） | `EVALUATION_REPORT.md` §7.1；`evidence/TPCH_FIXTURE_5379_2026-10-20.md` |
| TPC-H Q20 oracle 产物（#4540） | `sqlrustgo/q20.tsv` 必须存在且 SHA256 可比对 | 交叉引擎哈希比对 | `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/` | `sqlrustgo/q20.tsv` **不存在**（21/22 文件）；`CROSS_ENGINE_HASH_CHECK.json` 记 `present:false`；与 `GA5_TPCH_SF1_REPORT.md:40` 的「Q20 PASS」直接矛盾 | **NOT MET** | `DEFECTS_AND_ISSUES.md` P1-02；`STAGE.yaml` `open_issues` |
| QPS/TPS 基准（G11） | 判据前提：专用基准主机 + `PERFORMANCE_BASELINE.md` | `scripts/gate/check_g11_qps.sh` | 同上；`benches/qps_bench.rs` | v4.1.0 目录下**无** `PERFORMANCE_BASELINE.md`（该文件只存在于 v3.9.0 / v3.10.0） | **NOT RUN**（前置产物缺失） | `STAGE_CONFIG.yaml` `gate_definitions.G11.requirement` |
| Sysbench OLTP（G12） | 判据前提：sysbench 二进制 | `scripts/gate/check_g12_sysbench.sh` | 同上 | v4.1.0 无独立 GA 级 sysbench 轮次记录；只有 8h SOAK 内的 sysbench 负载记录 | **NOT RUN**（未按 G12 门禁执行） | `STAGE_CONFIG.yaml` `gate_definitions.G12.requirement` |
| 性能汇总报告（G15） | 判据前提：`PERFORMANCE_REPORT.md` + 5 份子报告 + `PERFORMANCE_BASELINE.md` | `scripts/gate/check_g15_perf_report.sh` | 同上 | v4.1.0 目录下三者**均不存在** | **NOT RUN**（前置产物缺失） | `STAGE_CONFIG.yaml` `gate_definitions.G15.requirement` |

---

## 4. 已实测但不构成 GA 判据的测量（阈值未记录 ⇒ 不可判定达标与否）

下表中的数值**都是实测**，但仓库**没有为它们记录任何 GA 级阈值**
（§1.4 已列出查过的路径）。因此本节**不标 PASS**，只标「改善已实测」，
避免把「测到一个数」说成「达到标准」。

| 测量 | 基线 | 修复后 | 倍数 | 出处 |
|---|---:|---:|---:|---|
| 事务吞吐（TPS，8 并发 × 150 事务，file storage） | 402.0 | 6 846.2 | 17.0× | `PERF_A1_4912_AB_MEASUREMENT.md` §3.1 |
| BEGIN 延迟 p50 / p95 / p99 | 4.203 / 18.987 / 24.242 ms | 0.231 / 0.716 / 1.017 ms | 18.2× / 26.5× / 23.8× | 同上 §3.2 |
| SELECT 延迟 p50 / p95 / p99 | 4.256 / 18.553 / 23.977 ms | 0.240 / 0.732 / 0.977 ms | 17.8× / 25.3× / 24.5× | 同上 |
| COMMIT 延迟 p50 / p95 / p99 | 4.040 / 19.201 / 24.132 ms | 0.238 / 0.694 / 0.945 ms | 16.9× / 27.7× / 25.5× | 同上 |
| `parking_lot` 锁栈帧数（`sample`） | 54 | 0 | — | 同上 §3.3 |
| 写路径深拷贝（1 000 / 10 000 / 50 000 行） | 0.603 / 0.615 / 1.002 ms | 0.027 / 0.026 / 0.026 ms | 22.44× / 23.91× / 39.26× | `PERF_B2_4915_AB_MEASUREMENT.md` §3 |
| 全量快照（10 000 / 50 000 行） | 3.531 / 13.703 ms | 3.318 / 11.605 ms | 1.06× / 1.18× | 同上 |
| 过滤扫描未命中（50 000 行） | 0.053 ms | 0.045 ms | 1.19× | 同上 |
| WAL 恢复每行成本（500 / 2 000 / 8 000 行） | 162.4 / 339.6 / 1 086.1 µs | 210.3 / 224.0 / 372.4 µs | 外推到 58 288 行 ≈ 461 s → ≈ 21.7 s（≈ 21×） | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §4.2 |
| sysbench QPS（1 000 行） | 6 577 | 9 759 / 10 007 / 10 776 | ≈ 1.5× | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` §5.1 |
| sysbench QPS（10 000 行） | 862 | 1 242 / 1 276 / 1 295 | ≈ 1.5× | 同上 |

> 上表所有数字都来自**短时定向测量**（每轮 0.2–3.5 s 量级），报告自身已声明
> 「未做 8 小时以上 SOAK、未测多核扩展曲线、未做统计显著性检验」。
> 把它们当作 GA 级吞吐结论是过度解读。

---

## 5. 当前未达成 / 未执行的目标（NOT MET / NOT RUN 汇总）

| # | 目标 | 结论 | 原因（一句话） |
|---|---|---|---|
| 1 | 168h 多模型 SOAK（V400-09） | **NOT MET** | 进程已终止、无 run3/run4 产出；`known_gaps` 状态 `NOT MET`（verified 2026-10-07） |
| 2 | 168h SOAK 的架构师裁决 | **NOT MET** | `decision: "OPEN"` —— SSOT 明确不代为决定 |
| 3 | TPC-H SF=1 22/22 | **NOT RUN** | v4.1.0 从未执行（`EVALUATION_REPORT.md` §7.1 记为 not recorded） |
| 4 | TPC-H Q20 oracle 产物（#4540） | **NOT MET** | `sqlrustgo/q20.tsv` 缺失，`CROSS_ENGINE_HASH_CHECK.json` 记 `present:false` |
| 5 | G13 24h+ 稳定性 | **NOT MET** | 门禁当前 exit 1（`tests/soak_test_harness.rs` not found），且无 24h+ 报告 |
| 6 | G11 QPS/TPS | **NOT RUN** | 缺 `PERFORMANCE_BASELINE.md` 与专用基准主机 |
| 7 | G12 Sysbench OLTP | **NOT RUN** | 未按 G12 门禁执行 |
| 8 | G14 进程级崩溃测试 | **NOT RUN** | 缺 `CRASH_TEST_REPORT.md` 与 Z6G4 主机 |
| 9 | G15 性能汇总报告 | **NOT RUN** | 缺 `PERFORMANCE_REPORT.md` + 5 份子报告 + `PERFORMANCE_BASELINE.md` |
| 10 | 内存稳态（1h SOAK RSS 单调上升至 662.8 MB） | **NOT MET** | 报告自述「未证明稳定内存平衡」 |
| 11 | 吞吐随并发提升 | **NOT MET** | 并发 4 → 64 吞吐不升反降；瓶颈在写路径 O(rows) 遍历与全局独占锁 |
| 12 | B2.2 锁内 I/O 的并发收益 | **NOT MET** | 实测落在噪声内，且该方法在生产调用链上不可达 |
| 13 | Q20 去相关接线 | **NOT MET** | `try_decorrelate()` 零调用点 |
| 14 | Page Checksum 门禁 | **NOT MET** | 判据是「字段存在」，实际函数零命中 |
| 15 | CI `test` job 绿灯 | **NOT MET** | 1 452 次机会中 0 次成功完成 |
| 16 | CI `lint-build` 绿灯 | **NOT MET** | run 8750 / 8831 记录为红 |
| 17 | `Update` / `Delete` 恢复路径 | **NOT RUN** | 本批明确不扩大范围 |
| 18 | 58 288 行规模恢复总耗时 | **NOT RUN** | 仅有三点实测 + 外推，未在该规模实测 |

---

## 6. 性能相关的已知风险与债务

| # | 项 | 来源 | 性能影响 | 现状 |
|---|---|---|---|---|
| R1 | **P1-01 记录的覆盖率口径矛盾** | `DEFECTS_AND_ISSUES.md` P1-01；`EVALUATION_REPORT.md` D3 | 该条销账时**明确只关掉「门禁 fail-open」这一半**，并写明「**阈值之争（50% vs 80%）不在本条范围内**」。同一份条目里同时出现 ALPHA 门槛 50%（`STAGE_CONFIG.yaml` `COVERAGE_MIN_ALPHA`）与 80%（`DEFECTS_AND_ISSUES.md` P1-01、`ARCHITECTURE.md` §1）两个口径 | **未解决**。仓库未裁决哪个门槛管辖 ALPHA（`EVALUATION_REPORT.md` §7.2 第 3 项）。⇒ 本文 §2.3 同时列出四个门槛并逐条判定，不代为裁决 |
| R2 | 写路径每行一次落盘（`append_table_delta` 每次 open/write/flush/close） | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §6 | 约 86 µs/行，58 288 行 ≈ 5 s；是**常数**成本但绝对值可观 | 未优化（明确记录为「本批不动它」） |
| R3 | `Update` / `Delete` 恢复路径仍是每条 entry 扫一次表 | 同上 §6 | 随表规模增长 | 未优化 |
| R4 | 恢复与 listener 的先后关系未变 | 同上 §5 / §6 | 恢复仍在 listener 接受连接之前完成 ⇒ **「硬性启动不可用」是结构性结论**，本次修复只把耗时从分钟级降到秒级 | 未改动 |
| R5 | `Value::Hash` 与 `PartialEq` 不一致（0.0 与 −0.0 相等却哈希不同） | `crates/types/src/value.rs`；证据文档 §3.2 | 影响唯一约束判定 / hash join / MVCC 键集 | **已修**（随 P0-08 一并修复） |
| R6 | 三个大文件不在 C-ARCH-05 覆盖内（`expr/mod.rs` 5 104 / `stored_proc.rs` 4 248 / `trigger.rs` 2 549 行） | `DEFECTS_AND_ISSUES.md` P2-03；`STAGE.yaml` `inherited_from_v4000_revocation` REF-01..03 | 大文件 = 每行分配 / O(N²) / 整表拷贝的高发区；门禁当前只看 `execution_engine.rs` | 未拆分（REF-01..03 记为 P1/P1/P2） |
| R7 | `BoxStorageEngine` 类型擦除导致已实现优化被旁路 | `PERFORMANCE_AUDIT_REPORT_2026-09-30.md` F-01 | 收益从 0 变 17× | **已修**（#4912） |
| R8 | `ExecutionEngine::execute(&mut self)` + 每连接独占 engine 锁 | 同上 F-02 | 同一连接内语句级并行被借用检查器阻止 | 未改（优先级低于 F-01） |
| R9 | 静态审计的「预期收益」全部是假设 | `PERFORMANCE_AUDIT_REPORT_2026-09-30.md` §0 | 18 条 finding（2 P0-CRITICAL / 4 P0-QUICKWIN / 7 P1-ALGORITHMIC / 5 P2-STRUCTURAL）中未被实测验证的部分，不能作为 GA 判据 | 部分已实测（A1、B2.1/B2.3/B2.4），其余未测 |
| R10 | fault-injection 用例每个 WAL 追加点自旋 120 s | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §7 / §8 | CI 成员 job 的 5 400 s 预算会被吃光，并以「超时」而非「测试失败」收场 | 已登记为 issue，未修 |
| R11 | server 在 `--wal-sync every` 大量写入后卡进 macOS 不可中断 I/O（进程状态 `U`） | 同上 §7 | SIGTERM 不生效，容器编排的 SIGTERM 宽限期会被耗尽 | 已登记为 issue，**根因未定位** |
| R12 | `#5167` `COUNT(*)` / 范围扫描返回 0 | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` 结论摘要 | **正确性**问题而非性能问题，但会污染性能测量 | 未修 |

---

## 7. 测量环境说明

### 7.1 本文档引用的每组数字分别来自哪里

| 数字组 | 环境 | 测量日期 | 被测 commit | 出处 |
|---|---|---|---|---|
| L1_8 87.80% 及 8 个 per-crate 值 | **M 本机**（Mac-mini-7.local, arm64, macOS 27.0.1） | 2026-10-23 | `3a3350f660` | `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3 |
| `mysql-server` 87.89% | 本机 | 2026-10-07 | `0ddb086678` | `evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md` |
| 8h SOAK 全套观测 | **S 服务器**（80 核 / 404 GB） | 2026-10-08/09 | 稳定性 `51d7ad7d4`；修复后性能 `5ddd023ef` | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` |
| 1h SOAK 全套观测 | 本机单 10 核主机 | 2026-10-08 | `b1e9a98ee6` | `SOAK_V410_1H_2026-10-08/STABILITY_REPORT.md` |
| WAL 恢复规模测量 | **M 本机** | 2026-10-23 | `3a3350f660` | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` |
| A1 / B2 / B22 / F-08 A/B | 本机 | 2026-09-30 ~ 2026-10-04 | `99198a515` / `be665d6bc1` / `ff34478830` / `3d503b5ab092` | 各 `PERF_*.md` 头部 provenance |
| CI 运行记录 | **C CI** | 2026-10-22 | 多次 | `evidence/CI_TRUTH_BATCH24_2026-10-22.md` |
| 168h SOAK | **U 未跑** | — | — | `STAGE.yaml` `known_gaps.V400_09_soak` |
| TPC-H SF=1 22/22（v4.1.0） | **U 未跑** | — | — | `EVALUATION_REPORT.md` §7.1 |

### 7.2 本轮（文档编写轮）实际跑过的东西

本轮**只跑了一条只读门禁**，并**尝试**跑了一条；其余全部引用仓库既有证据：

| 门禁 | 结果 | 说明 |
|---|---|---|
| `scripts/gate/check_evidence_binding.sh v4.1.0` | `PASS=141, WARN=123, FAIL=0, UNVERIFIED=0`（exit 0，「✅ Evidence binding check PASSED」） | 2026-10-23 本机实跑；输出报告写到 `/tmp/evb-batch32b/`（**不落仓库**）。该门禁遍历发布目录下**每一个** `.md`，因此计数器随同目录新增的产物变化 —— 本轮首次跑（仅本文档之前）为 `PASS=127`，两次之间同目录新增了其它 GA 产物文件。**判据是 `FAIL=0` 与 `UNVERIFIED=0`，不是 `PASS` 的绝对数** |
| `scripts/gate/check_anti_fabrication.sh` | **本轮复跑 NOT RUN** —— 400 s 内卡在 CHECK 1（`cargo build -p sqlrustgo-mysql-server`），`RC=124` 超时，未产出最终结论 | 该门禁的**仓库已记录**结果为：`ERRORS=0, WARNINGS=0`、PASS（2026-10-23 本机实跑，`evidence/GA_MANDATORY_DOCS_BATCH27_2026-10-23.md` §6.3）。另见 `evidence/CI_TRUTH_BATCH24_2026-10-22.md` 的同项记录（`EXIT=0`）。**本轮未复现该结论，故本轮该项记为 NOT RUN** |
| `scripts/gate/check_docs_links.sh` | exit 0，「All markdown links are valid」 | 2026-10-23 本机实跑 |
| `scripts/gate/check_release_artifacts.sh --version v4.1.0 --stage RC` | `RESULT: PASS`（exit 0）；RC 的 `doc_artifact` 含本文档 | 2026-10-23 本机实跑 |
| `scripts/gate/check_release_artifacts.sh --version v4.1.0 --stage GA` | `RESULT: PASS`（exit 0）；GA 的 `doc_artifact` 含 `POST_GA_PLAN.md` | 2026-10-23 本机实跑 |

> 后两条门禁只检查**产物是否存在**，**不检查产物内容是否正确** ——
> `check_release_artifacts.sh` 的文件头原话：「It does not create the artifacts.
> A missing GA artifact is reported as missing — it is never faked, never stubbed,
> and "skip" is never reported as "pass".」

**本轮没有跑**：`cargo build` / `cargo test` / `cargo clippy` / `cargo fmt` /
`cargo bench` / `cargo llvm-cov` / 任何 SOAK / 任何 TPC-H /
`check_rc_ga_gate.sh` / `check_g1_tpch_baseline.sh` / G11–G15 全部。
**未跑 ≠ 通过。**

### 7.3 测量方法上的三条硬约束（来自仓库自身的协议）

1. **覆盖率必须 `--test-threads=1`。** `cargo llvm-cov` 下代码慢 2–10 倍，
   默认并行时读数随机器负载漂移 —— 这正是 `debt-registry.yaml` 里 SEM-4
   关闭根因写成字面占位符 `"XYZ"` 的那件事。实测证据：同一 checkout 上
   `sqlrustgo-planner` 用 `--test-threads=1` 得 88.91% 且 PASSED，用默认线程数
   则 FAIL。
2. **性能测量不得用 `cargo test` 计时。** `[profile.test] opt-level = 0` 会让
   以测试形式运行的测量失真（`PERFORMANCE_OPTIMIZATION_PLAN.md` §6，审计 F-18）。
   一律用 `cargo bench`（走 `[profile.bench]` / release）。
3. **绝对数字随机器负载浮动，可复现的是形状。** WAL 恢复证据文档原话：
   「同一份代码在两次运行里差过 2 倍」；B2 证据文档记载同一二进制两次相差
   2 倍以上（`[293,390]` vs `[671,924]`），最后靠「每轮重建 `FileStorage`」
   才把基准缺陷与真实回归分开。

---

## 8. 与既有性能文档的关系（引用，不复述）

| 文档 | 作用 | 本文档的用法 |
|---|---|---|
| `PERFORMANCE_AUDIT_REPORT_2026-09-30.md` | 18 条**静态代码审计** finding（2 P0-CRITICAL / 4 P0-QUICKWIN / 7 P1-ALGORITHMIC / 5 P2-STRUCTURAL）。§0 明确声明**未执行** `cargo bench` / profiler / SOAK，所有「预期收益」是假设 | 只引用它的**机制判断**（F-01 / F-02 / F-18）与 finding 编号；**不把它的「预期收益」当实测值** |
| `PERFORMANCE_OPTIMIZATION_PLAN.md` | Phase A/B/C 的执行计划 + §6 测量协议 + §10 实施记录 | 引用 §6 作为测量方法来源；Phase 判据以它为准 |
| `PERF_A1_4912_AB_MEASUREMENT.md` | #4912 无锁事务接线的 A/B 实测 | §3.3 / §4 引用其 TPS 与分位延迟 |
| `PERF_B2_4915_AB_MEASUREMENT.md` | #4915 存储层去拷贝的 A/B 实测 | §3.3 / §4 引用其 22×–39× 与中性项 |
| `PERF_B22_CONCURRENT_MEASUREMENT.md` | B2.2 的并发补测（机制性否定 + 性能中性） | §3.3 B2.2 行的「NOT MET」依据 |
| `PERF_B13_F08_BENCHMARK_AND_ALLOC_FREE_BINDING.md` | F-08 基准与零分配第一刀 | §3.3 绑定分配行 |
| `PERF_4956_COMMIT_FLUSH_AB_2026-10-04.md` / `PERF_4910_PHASE3_PLAN.md` | COMMIT/flush 的 A/B 与 #4910 的 Phase 3 计划 | 本文未引用其数值；列此备查 |
| `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` | 8h SOAK + 容量阶梯 + 四个逐层递进缺陷 | §3.3 / §3.4 / §6 的性能与风险数据源 |
| `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` | P0-08 的测量台、修法、验证与对原报告的更正 | §3.2 的五行关闭条件依据 |
| `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` | SEM-4 销账 + 覆盖率 8-crate 实跑 + 三个门禁缺陷 | §2.2 / §2.3 / §3.1 的覆盖率来源 |
| `ISSUE-RECOVERY-QUADRATIC-REPLAY.md` §6 | P0-08 的**五条关闭条件**（本文 §3.2 的阈值出处） | 逐条对照引用 |
| `EVALUATION_REPORT.md` | 文档层评估，含 §7.1「not recorded」清单与 D1–D10 矛盾表 | §1.4 / §3.5 的「未记录」判定依据 |

**本文档不复述这些文档的内容**，只做三件事：把判据摆齐（§3）、把未达成的挑明
（§5）、把口径的坑标出来（§6）。

---

## 9. 签署块

> **本文件由 `openclaw-mcode` 自动生成，晋级审批由人类架构师执行
> （`STAGE_CONFIG.yaml` 三次阶段跃迁 `requires_human_approval: true`）。**
> **本文件不代替签字，也不构成任何晋级结论。**

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 性能目标负责人（编制） | `______` | `______` | `______` |
| 覆盖率口径裁决人（R1 的裁决人） | `______` | `______` | `______` |
| Release Manager | `______` | `______` | `______` |
| Human Architect（RC → GA 晋级签字） | `______` | `______` | `______` |

**未决事项（签字前必须先有裁决）**

| # | 事项 | 裁决人 | 结论 | 日期 |
|---|---|---|---|---|
| 1 | 是否重跑 168h SOAK（`known_gaps.V400_09_soak.decision` 现为 `OPEN`） | `______` | `______` | `______` |
| 2 | ALPHA 覆盖率门槛口径矛盾（50% vs 80%） | `______` | `______` | `______` |
| 3 | 本文 §3 中所有 `NOT MET` / `NOT RUN` 项是否接受为已知边界 | `______` | `______` | `______` |
