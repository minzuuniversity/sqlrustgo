# v4.1.0 遗留问题全集台账（v3.6.0 → v4.1.0 跨版本补全）

> **日期**: 2026-09-30
> **范围**: v3.6.0 ~ v4.1.0 全部版本的遗留/债务类文档
> **状态**: 待治理确认（本台账为考古结果，未替代任何实跑验证）
> **provenance**: generated_by=ai (mavis/claude-macmini), generated_at=2026-09-30,
> source_run=cross-version-legacy-archaeology-2026-09-30, source_head=ff34478830,
> policy=Anti-Fabrication-Policy-v1.0
>
> ⚠️ **本台账的定位**：`docs/releases/v4.1.0/ISSUES_PLAN.md` 当前只继承
> **v4.0.0 → v4.1.0 这一跳**的 41 条**带 Gitea 号**的项。本台账证明：
> **v3.6.0 ~ v3.11.0 的遗留链在版本交接中基本断裂**，且 v4.0.0 自身还有
> 一批被错误标记为 DONE 的项。
>
> **本文档不判定代码层真实状态**（未运行任何测试/门禁）。
> 所有条目均为**文档层**结论，需后续实跑或 Gitea 状态核实。

---

## 0. 审计方法与可信度

| 项 | 说明 |
|---|---|
| 覆盖版本 | v3.6.0 / v3.7.0 / v3.8.0 / v3.9.0 / v3.10.0 / v3.11.0 / v3.12.0 / v4.0.0 / v4.1.0 |
| 阅读文档 | 34 份（各版本 LEGACY/DEBT/CAVEAT/DISPOSITION 类） |
| 方法 | 三个并行只读代理逐版本提取 + 主代理抽验 4 项关键结论 |
| 抽验结果 | 4/4 属实（见 §7） |
| 局限 | 未核对 Gitea 实际 issue 状态；未运行测试；`debt-registry.yaml` 为 2026-07-16 的 v3.11.0 快照，**自 7 月起未更新** |

---

## 1. 核心结论

### 1.1 传递链在 v3.11 → v3.12 之间就已断裂

| 交接批次 | 传递情况 |
|---|---|
| v3.6.0 → v3.7.0 | 部分传递（INT-1~4 延续，但每版本重新编号） |
| v3.7.0 → v3.8.0 | 部分传递 |
| v3.8.0 → v3.9.0 | **断裂** — v3.9.0 顶层无任何 gap/debt/legacy 文档 |
| v3.9.0 → v3.10.0 | 断裂（仅 `debt-registry.yaml` 承接） |
| v3.10.0 → v3.11.0 | 部分传递（debt-registry 更新） |
| v3.11.0 → v3.12.0 | 断裂 — `BLOCKER_DISPOSITION_V311.md` 记录 23 项 CARRIED，但其中 3 项（Window/GIS/JSON）**无 Gitea 号可追溯** |
| v3.12.0 → v4.0.0 | **部分传递** — v4.0.0 `LEGACY_ISSUES.md` 只收录带号项 |
| v4.0.0 → v4.1.0 | 部分传递 + **误继承 DONE**（见 §2） |

### 1.2 实测交叉比对：v3.6.0–v3.11.0 的 issue 号在 v4.x 几乎零命中

对 v3.10/v3.11 的 9 个核心债务 issue（#3493~#3499、#3431、#3501）、
v3.10/v3.12 的架构债（#4027、#4032、#4033、#4444、#3136、#3146）、
v3.9 的 GA 遗留（#2948、#3423、#3648、#3266、#3312、#3102、#3103）：

**在 `docs/releases/v4.0.0/*.md` + `v4.1.0/*.md` 中命中文件数均为 0。**

（`#3302` 唯一 1 次命中经核为数字误匹配 —— 匹配到 `COVERAGE_ANALYSIS_REPORT.md:78`
的 `13302/13302` 文本。）

### 1.3 无 Gitea 号的项是系统性黑洞

`historical-backlog-disposition.yml`（89 项历史积压）与
`v312-24_deferred_items_status.md`、`storage-index-wal-backlog-report.md` 中，
**共 35 项没有 Gitea issue 号**，仅有内部 ID 或中文功能名。

这类项在版本交接时**没有任何检索机制保证被继承**，实测命中率接近 0。

---

## 2. 被错误继承为 DONE 的项（本次新发现，P0）

### 2.1 WP-A 四个 issue：v4.1.0 直接丢弃

v4.1.0 `LEGACY_ISSUES.md:16` 写：
```
| WP-A | parser legacy #4708 #4696 #4710 #4720 | DONE | None (inherited DONE) |
```

**但 v4.0.0 自身文档对 WP-A 有三种互相矛盾的陈述**：

| 来源 | 陈述 |
|---|---|
| `CLAIM_DOWNGRADE_MANIFEST.md:28` | `✅ DONE` — "Covered by 280 v400 parser tests" |
| `ALPHA_GATE_REPORT.md:180` | `🟡` — "test work merged (PR #3752, #3753); **fixes pending**" |
| `RC_GATE_REPORT.md:136` | "13 tests merged; **fixes partial — full closure tracked in v4.0.1**" |

**实测反证**（`crates/parser/tests/wp_a_legacy.rs`）：

```
:68:    #[ignore] // TODO: 多行注释解析需要修复
:69:    fn test_chinese_comment_multi_line() {
```

该文件 28 个 `#[test]` 中有 1 个被 `#[ignore]`，注释明写
"多行注释解析需要修复" —— 属 #4708（Chinese identifier / comment）范围。

且该 `#[ignore]` **未登记在 `tests/baseline/ignore_registry.json`**
（125 条登记项中含 `wp_a` 的为 0 条）。

**判定**：WP-A **未真正闭环**。#4708（中文注释，含多行）、#4696、#4710、#4720
四条应重新进入 v4.1.0 backlog。

> 补充：`#4710` / `#4720` 在 `tests/` 目录中被引用的文件数为 **0**。

### 2.2 其他"DONE 但有反证"的存疑项

| 项 | 声称 | 反证 | 证据 |
|---|---|---|---|
| `F-24` AHI | `CLOSED` | progress_metric 内自列 "REMAINING: Wire AHI.lookup() into BTreeIndex::scan"、"replace synthetic page_id with actual B+ Tree page handle" 两项**明确未做**；且自述 "CLOSED is **feasible** if hermes **accepts** the v2 scope" | `debt-registry.yaml:365-378` |
| `F-23` Clustered Index | `CLOSED` (PR #3461) | 审计判"⚠️ 部分实现"、"测试的是孤岛代码，不是主路径"、"主路径未验证"；修正 PR #3516 **未登记进 debt-registry** | `LEGACY_DEBT_AUDIT_REPORT.md:50,138,145` |
| `F-31` Performance Schema | `CLOSED` | "InstrumentationHook trait 存在但**无任何执行算子实际调用**"；原计划要求集成到 8 个核心算子，未达成 | `AUDIT:105`、`V311_DEBT_CLOSURE_PLAN:60` |
| `F-32` MySQL Admin | `CLOSED` | `debt-registry` 内部自相矛盾：`f_xx_isolated.F-32 = CLOSED` 但 `extension_crates.admin = PARTIAL`；审计判"未在 mysql-server 中被使用" | `debt-registry.yaml:433-438, 613-616` |
| `SEM-4` Coverage | `CLOSED` | 同文件头注释写 "SEM-4 coverage **IN_PROGRESS**"；`closing_pr: []` **空数组**；note 中根因仍为占位符 "XYZ" | `debt-registry.yaml:15, 279-293` |
| `ARCH-2/3`、`SEM-1` | `CLOSED` | 同版本 `ARCHITECTURE_DEBT_ANALYSIS.md` 判 `❌ OPEN` / `⚠️ PARTIAL` —— **同版本三方冲突** | `ARCH_DEBT:311,322,333` vs `yaml:196,209,225` |
| F-xx 23/36 统计 | "64% 已关闭" | **文档自我推翻**："23/36 'closed' claim is **misleading**; 10 are isolated tests and 3 are not actually implemented" | `v3.8.0/debt/INT5_PLUS_DEBT_INVENTORY.md:78` |
| v3.9.0 GA | "门禁全闭环，可投产" | 同目录 `GA_GATE_REPORT.md:129`："**GA cut NOT recommended** until G13 re-run + 72h soak complete" | 两份 GA 文档直接冲突 |
| v3.11.0 RC | RC_BLOCKERS 13 项 OPEN/IN_PROGRESS | 同目录 `debt-registry.yaml:658-666` 已宣称 "v3.11.0 GA 6/6 gates PASS" | `RC_BLOCKERS:3` vs `yaml:658-666` |

---

## 3. v4.1.0 剩余遗留问题全集

### 组 A — 已在 v4.1.0 backlog（20 条，`ISSUES_PLAN.md` §4）

| 类别 | 数量 | issue |
|---|---|---|
| WP-C DDL/完整性 | 6 | #4682 #4652 #4672 #4669 #4709 #4703 |
| WP-D Join/子查询 | 4 | #4668 #4656 #4649 #4636 |
| WP-B 类型/函数 | 6 | #4721 #4674 #4716 #4676 #4675 **#4670** |
| WP-F Schema | 1 | #4848 |
| WP-G 类型/比较 | 1 | #4846 |
| WP-E 事务 | 1 | #4626 |
| WP-H | 1 | #4639 |
| **小计** | **20** | 全部 NOT STARTED |

**另需加入**（本次新发现）：**WP-A 4 条**（#4708 #4696 #4710 #4720）→ backlog 变为 **24 条**

### 组 B — 继承自 v4.0.0 的覆盖/架构债（已在 `STAGE.yaml` 登记）

| ID | 范围 | 优先级 |
|---|---|---|
| COV-01 | parser 74.32% → ≥85% | P0 |
| COV-02 | mysql-server 75.41% → ≥85% | P0 |
| REF-01 | 拆 `expr/mod.rs`（5104 行） | P1 |
| REF-02 | 拆 `stored_proc.rs`（4248 行） | P1 |
| REF-03 | 拆 `trigger.rs`（2549 行） | P2 |

### 组 C — v3.6.0~v3.11.0 失联遗留（本次补全，全部无 v4 归属）

#### C1. F-xx 功能缺口（13 项）

| ID | 功能 | 最后状态 | 建议归属 |
|---|---|---|---|
| F-03 | GIS 空间类型 | ❌ NOT IMPLEMENTED（"40 files" 声明为假） | v3.9 计划 #3103 → 未见闭环 |
| F-30 | CREATE SEQUENCE | ❌ NOT IMPLEMENTED | #3103 / #3496 → 未闭环 |
| F-36 | 列级权限 | ❌ NOT IMPLEMENTED（"4 files" 声明为假） | #3103 / PR #3457 声称 CLOSED 但无条目 |
| F-16 | Gap Locking | ⚠️ ISOLATED（自包含 mock） | #3102 |
| F-23 | Clustered Index | ⚠️ ISOLATED（BTreeMap 自包含） | #3102 / #3516 |
| F-24 | Adaptive Hash Index | ⚠️ ISOLATED（in-memory mock） | #3102 / #3146 |
| F-25 | Change Buffer | ⚠️ ISOLATED（测试内联） | #3102 |
| F-26 | Double-Write Buffer | ⚠️ ISOLATED（in-memory mock） | #3102 |
| F-27 | Table Compression | ⚠️ ISOLATED（仅 RLE，非 LZ4/zstd） | #3102 / #3498 |
| F-29 | Row-Level Security | ⚠️ ISOLATED（in-memory catalog） | #3102 / #3494 |
| F-31 | Performance Schema | ⚠️ ISOLATED（in-memory mock） | #3102 |
| F-32 | mysqladmin | ⚠️ ISOLATED（in-test mock） | #3102 / #3495 |
| F-35 | Password Rotation | ⚠️ ISOLATED（in-memory only） | #3102 / #3495 |

#### C2. T-xx 测试缺口（5 项）

| ID | 内容 | 状态 |
|---|---|---|
| T-19 | Disk I/O delay 故障注入 | ❌ NOT IMPLEMENTED（0 commits, 0 SPEC） |
| T-20 | process kill -9 mid-transaction | ❌ NOT IMPLEMENTED；声称的 `e2e_crash_recovery_proof` 文件**不存在** |
| T-15 | 死锁注入 | ⚠️ ISOLATED（mock） |
| T-17 | 网络 30% 丢包 | ⚠️ ISOLATED（mock） |
| T-18 | 内存故障注入 | ⚠️ ISOLATED（mock） |

#### C3. INT-x 集成债务（4 项，跨 v1.2.0 起）

| ID | 内容 | 链条 |
|---|---|---|
| INT-1 | DML 不过 WAL/TransactionManager | v1.2.0 → v3.6 开放 → v3.7 未修复 → v3.8 文档自相矛盾 → 未见闭环 |
| INT-2 | ParallelVolcanoExecutor 主路径不调用 | v2.6.0 → v3.8「0% 进展」→ #3146 |
| INT-3 | expr crate 双实现（1/15 委托，~7%） | v3.0.0 → #3146 |
| INT-4 | mysql-server 与主 server 双路径 | v2.6.0 → 部分改善 → 未闭环 |

#### C4. ARCH / SEM 架构债（4 项）

| ID | 内容 | 状态 |
|---|---|---|
| ARCH-2 | 双路径（mysql-server vs bench-cli） | ❌ OPEN |
| ARCH-3 | VTU 主路径剩余 5% | ⚠️ PARTIAL |
| SEM-1 | ROLLBACK MVCC（记录但不 fence 未提交数据） | ❌ OPEN |
| SEM-3 | ALTER TABLE 不完整 | ❌ OPEN（仅 MODIFY 已修） |

#### C5. 覆盖率债（4 条链）

| 标识 | 内容 | 状态 |
|---|---|---|
| `SEM-4` | 覆盖率测量差异（Z6G4 82% vs Z440 32%），根因至今是占位符 "XYZ" | IN_PROGRESS，debt-registry 无 expiry |
| `#3302` | parser 覆盖率 ~60%，差 -20pp | v3.9 GA 承诺 v3.10 GA 前达 80% |
| `#3943` | v3.12.0 覆盖率债 | 未闭环 |
| `V310-10` | workspace 覆盖率 ≥80% | v3.10.0 实测 **14.71%** |

> 口径矛盾：v3.10.0 文档记 **14.71%**，另一文档记 **~67%**，debt-registry 记
> v3.12.0 L1_8 **84.44%** —— 三处互不相同（`V310:192` / `REPORT:313` / `yaml:284`）。

#### C6. TPC-H 债（6 项）

| 标识 | 内容 | 状态 |
|---|---|---|
| `#3423` | SF=1.0 跨引擎 baseline | SUPERSEDED，硬件阻塞（需 75GB+） |
| `#3648` | 混合负载 SOAK 跨平台验证 | SUPERSEDED，需 Z6G4/Z440 |
| `#2948` | TPC-H SF≥1（Track 3） | deferred（无 SF1 fixtures） |
| `#4432` | Q17 SF=1 性能 | deferred-to-v3.13，Expiry 2027-03-31 |
| `#4429` / `#4426` / `#4435` | Q20 性能 / decorrelation / BINT profile | deferred-to-v3.13 |
| `#4540` | Q20 oracle sha256 验证 | still pending verification |

#### C7. 扩展 crate（5 项非终态）

| ID | 状态 | 备注 |
|---|---|---|
| `gmp` | SCOPE_DEFERRED | "external project, **product decision pending**" |
| `rag` | SCOPE_DEFERRED | "no mysql-server dependency" |
| `graph` | ARCHIVED | 已归档到 `archive/v3.11/archived-crates/graph/` |
| `admin` | PARTIAL | V310-14 PR #3795 发了 CLI，但与 `F-32` 状态矛盾 |
| `vector` | SCOPE_INTERNAL | 内部使用，非独立主线 |

#### C8. 门禁债（4 项，从 v3.10 deferred 后失联）

| ID | 内容 | 备注 |
|---|---|---|
| `FIX-GATE-ARCH-CARCH05` | C-ARCH-05 gate fix | **本次已顺手修复**（fail-closed），但该 deferred 记录本身仍未销账 |
| `FIX-GATE-COVERAGE-THRESHOLD` | 覆盖率阈值/排除门禁修复 | 仍失联 |
| `FIX-GATE-G13-STABILITY` | G13 稳定性门禁误报修复 | 仍失联 |
| `FIX-GATE-TEST-INVENTORY` | test-inventory bash3 修复 | 仍失联 |
| `FIX-G1-TPCH-CLI` | G1 TPC-H CLI harness 修复 | 仍失联 |
| `B-AUTH` / `B-PREP` | 空密码认证 / prepared stmt 分词边界 | 仍失联 |

#### C9. 存储/索引/WAL（6 项，#3910，**无 Round 反转章节**）

| 内容 | 状态 |
|---|---|
| WAL Checkpoint | DEFERRED — 仅手动 |
| Page Checksum | PARTIAL — `page.rs:179` 跳过校验 |
| Index Statistics | PARTIAL — 框架在，自动采集 deferred |
| Vector SQL Surface | PARTIAL — 存储有，SQL 集成未完成 |
| Index Rebuild | NOT IMPLEMENTED |
| WAL Verification Tool（GMP 适配） | tool exists but redesign may be needed |

### 组 D — v3.12.0 特有失联（35 项无 Gitea 号，节选高影响）

| 内容 | 状态 | 证据 |
|---|---|---|
| TLS handshake（服务端） | ❌ Deferred to v3.13 | `v312-24_deferred_items_status.md:17` |
| zlib compression | ❌ Deferred | 同上 `:18` |
| COM_RESET_CONNECTION 全会话重置 | ⚠️ handler 存在但不重置会话状态 | 同上 `:20` |
| LOAD DATA SF=10 | ⚠️ Env-gated（dbgen 未跑） | 同上 `:16` |
| Window Functions | CARRIED 至 V312-16，**V312-16 无 Gitea 号映射** | `BLOCKER_DISPOSITION_V311.md:164-166` |
| GIS 扩展 | 同上 | 同上 |
| JSON Type | 同上 | 同上 |
| #3903 JSON | "JSON NOT IMPLEMENTED，需明确延期到 v3.13" | `v312_issue_comments.md:430-431` |
| #3909 Hash Semi Join | "NOT IMPLEMENTED，需明确延期" | 同上 `:436-437` |
| #3910 torn page protection | "NOT IMPLEMENTED，需明确延期" | 同上 `:439-440` |
| #3911 test-runner gate | "PR #3918 只有 openspec，需实现实际 gate 脚本" | 同上 `:442-443` |
| Prometheus 指标 / Slow Query Log | DEFERRED 至 V312-18（#3905） | `BLOCKER_DISPOSITION_V311.md:169-170` |
| AuditRecord hash chain 缺失 | 随 #3895 关闭放行的已知限制，**无号** | `v312_issue_comments.md:249-250` |
| 8 项"需要补充证据"的 issue | #3896~#3907 | 同上 `:398-424` |

### 组 E — 文档间状态矛盾（19 项，本次审计确认）

除 §2.2 已列 9 项外，另有：

| 矛盾 | 双方 |
|---|---|
| GA-claim-caveat 清单规模 | triage 17 项 vs manifest 9+3 项，**8 项无去向说明** |
| YAML 状态自洽性 | 6 项 `current_state: CLOSED` 但 `disposition: deferred` |
| #3909 | `v312_issue_comments` 要求延期 vs `execution-architecture-debt-report:250` "can be CLOSED" |
| v3.8.0 覆盖率 | `V310_TASK_CLOSURE:192` 14.71% vs `REPORT:313` ~67% vs `yaml:284` 84.44% |
| V311 计划目标 | 计划称"GA 时 debt-registry 0 OPEN / CLOSED 45 (100%)"，验收项 `- [ ]` **仍未勾选** |

---

## 4. 汇总统计

| 组 | 项数 | 是否在 v4.1.0 backlog |
|---|---|---|
| A. 已有 backlog（WP-A 4 条待加入 → 24） | 20（+4） | ✅ 是 |
| B. v4.0.0 覆盖/重构债 | 5 | ✅ 是（STAGE.yaml 已登记） |
| C1. F-xx 功能缺口 | 13 | ❌ **否** |
| C2. T-xx 测试缺口 | 5 | ❌ **否** |
| C3. INT-x 集成债 | 4 | ❌ **否** |
| C4. ARCH/SEM 架构债 | 4 | ❌ **否** |
| C5. 覆盖率债 | 4 链 | ⚠️ 部分（COV-01/02 覆盖，SEM-4 根因未查） |
| C6. TPC-H 债 | 6 | ❌ **否** |
| C7. 扩展 crate | 5 | ❌ **否** |
| C8. 门禁债 | 8 | ⚠️ 1 项已顺手修复但未销账 |
| C9. 存储/索引/WAL | 6 | ❌ **否** |
| D. v3.12.0 无号失联 | 35 | ❌ **否** |
| E. 状态矛盾 | 19 | — |
| **合计（去重前）** | **约 154 组条目 / 300+ 原始条目** | **约 24/154 有归属** |

**核心结论：v4.1.0 目前只闭环了约 15% 的历史遗留问题。**

---

## 5. 建议的下一步

| 优先级 | 动作 |
|---|---|
| **P0** | 把 §2.1 的 WP-A 4 条（#4708 #4696 #4710 #4720）正式加回 v4.1.0 backlog，backlog 20 → 24 |
| **P0** | 为组 C/D 全部条目在 v4.1.0 建立**显式归属**（哪怕结论是"接受为已知边界"），消除"SSOT 里没写 = 不存在"的静默丢失 |
| **P1** | 对组 E 的 19 项矛盾逐一定责：要么补证据转 CLOSED，要么承认未闭环 |
| **P1** | `debt-registry.yaml` 自 2026-07-16 起未更新（仍是 v3.11.0 快照），需重建为当前 SSOT |
| **P2** | 建立**无号项的登记规范**：强制要求每个 deferred 项至少有内部 ID + 来源文件行号 |
| **P2** | 在 `BRANCH_GOVERNANCE.md` 或新建 `LEGACY_INHERITANCE.md` 中固化"版本交接必须逐项确认传递"的流程 |

---

## 6. 本台账**不能**回答的问题

1. 这些项在**代码层**是否真的已修复 —— 本次只做文档考古，未运行任何测试/门禁
2. Gitea 上这些 issue 的**实际状态** —— 未访问 Gitea API
3. 哪些项已被 v3.12.0 之后的提交修复但文档未更新 —— 需要跑测试 + 查 git 历史
4. `debt-registry.yaml` 中 9 项非 CLOSED 的最新状态 —— 该文件已 2 个多月未更新

---

## 9. 2026-09-30 代码层核实结果（部分组已完成）

2026-09-30 启动了 3 个并行只读取证子代理 + 主代理自核 WP-A/E 组。**仅 1/3
子代理（C5-C9，24 项）跑完，另 2 个（C1-C4 26 项 + D 14 项）因 token 预算
耗尽中断。** 因此本次"逐项核实"完成的是：

| 组 | 项数 | 状态 | 来源 |
|---|---|---|---|
| WP-A (4 issue) | 4 | ✅ 实跑 Gitea API + cargo test + 探针 | 主代理自核，证据 `evidence/wp-a-reverify-2026-09-30/wp_a_reverify.txt` (sha256 `e5f2820f…`) |
| E 文档矛盾（5 项） | 5 | ✅ 实测 | 主代理自核，证据 `evidence/legacy-item-reverify-2026-09-30/doc_contradiction_reverify.txt` (sha256 `db3e26bd8…`) |
| C5 覆盖率债 (4 链) | 4 | ✅ 实跑 `cargo llvm-cov` 测 parser | 子代理 C5-C9 + 主代理记录 |
| C6 TPC-H (6 项) | 6 | ✅ 实测 oracle sha256 + 翻 deferral 文件 | 同上 |
| C7 扩展 crate (5 项) | 5 | ✅ `cargo metadata` + workspace 校验 | 同上 |
| C8 门禁债 (7 项) | 7 | ✅ 实跑 2 个只读 gate 脚本 | 同上 |
| C9 存储/索引/WAL (6 项) | 6 | ✅ grep 关键实现 + 跑 `check_storage_index_wal.sh` | 同上 |
| **小计** | **37** | | |
| C1 功能缺口 13 项 | 13 | ⏸ 待核（子代理中断） |  |
| C2 测试缺口 5 项 | 5 | ⏸ 待核（同上） |  |
| C3 集成债 4 项 | 4 | ⏸ 待核（同上） |  |
| C4 ARCH/SEM 4 项 | 4 | ⏸ 待核（同上） |  |
| D v3.12 无号失联 14 项 | 14 | ⏸ 待核（子代理中断，已见若干反转但未收齐） |  |
| **未核实合计** | **40** | |  |

**未核实的 40 项必须在下一轮补足**，不得在 "130+ 全部已核" 的叙述下提交。
台账 §2.1/§3/§5 等"未闭环"的措辞适用于此 40 项；本 §9 给出的 37 项
有具体 verdict（见下）。

### 9.1 WP-A（4 issue）实跑结果

| Issue | Gitea | 真实状态 | 证据 |
|---|---|---|---|
| #4708 | closed 2026-09-03T18:46:27Z | ❌ **NOT FIXED** — 块注释 `/* */` parser 完全不支持 | `cargo test -p sqlrustgo-parser --test wp_a_legacy` → 27 pass / 0 fail / **1 ignored**；临时探针：`SELECT 1 /* comment */` → `Expected expression` |
| #4696 | closed 2026-09-03T15:22:19Z | ⚠️ **PARTIAL** — parser 回归测试通过，E2E 未验证 | 同上 27 项测试全断言 `parse(sql).is_ok()`，无 binder/executor 覆盖 |
| #4710 | closed 2026-09-03T14:27:32Z | ⚠️ **PARTIAL** — parser 回归测试通过，E2E 未验证 | 实现 `parser.rs:5778-5812` 真实存在；token.rs:177 / lexer.rs:539-542 有 token |
| #4720 | closed 2026-09-03T14:27:33Z | ⚠️ **PARTIAL** — parser 回归测试通过，E2E 未验证 | 7 项回归测试；`parser.rs:19786` 有 `parse("SET @x = 1")` |

**附带的 AFP 违规**：

- `crates/parser/tests/wp_a_legacy.rs:68` 的 `#[ignore]` **未登记**
  `tests/baseline/ignore_registry.json`，违反 `ADR-008-test-claim-transparency`。
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md:28` 写
  "WP-A | ✅ DONE | Covered by 280 v400 parser tests"。实测：v400_*
  测试实际 **336** 个（不是 280），**无一**覆盖块注释，
  且 `crates/parser/src/token.rs` 没有 BlockComment token。两条都属
  AFP Type B（claim 无证据）。

### 9.2 门禁空转（独立发现）

`scripts/gate/check_anti_ignore_gate.sh` 全文**无任何对源码树的扫描**。
只读 `tests/baseline/ignore_registry.json` 自带的两个数字，与硬编码上限
比：ACTUAL 状态见 `evidence/anti-ignore-gate-hollow-2026-09-30/anti_ignore_gate_hollow.txt`
(sha256 `fedcb27f2a…`)。三重空转：(a) 不扫源码；(b) `active=0` 因
registry 条目无 `status` 字段永远空；(c) 33/125 条 `issue_link` 为空/TBD。
**实测**：新增 50 个未登记 `#[ignore]` 后门禁仍 exit 0（探针已删）。
该发现直接使 `PHASE_1_SCOPE.md §2.1.1` 由"✅ 已解决"订正为
"exit 0 但门禁空转"，并已登记在 `STAGE.yaml` `hollow_gate_anti_ignore`。

### 9.3 E 组 5 项实测摘要

详见 `evidence/legacy-item-reverify-2026-09-30/doc_contradiction_reverify.txt`。
要点：

- **E-2** `current_state=CLOSED` + `disposition=deferred` 6 条**与 C8 完全重合**
  （FIX-GATE-ARCH-CARCH05 / FIX-GATE-COVERAGE-THRESHOLD / FIX-GATE-G13-STABILITY
  / FIX-GATE-TEST-INVENTORY / FIX-P3-1-DEALLOCATE / FIX-G1-TPCH-CLI）—— 是同一
  缺陷的两面，应只在台账中算 6 条，不是 12 条
- **E-4** 三处覆盖率数字实为**不同测量范围**（v3.10 lib / v3.10 平均 /
  v3.12 L1_8 平均），不是同一指标重测。订正 ledger §3.5 的"三处互不相同"措辞
- 早前会话说 debt-registry"自 2026-07-16 起未更新"是**错的**——最后 commit
  `76dae7465e` @ 2026-08-15

### 9.4 C5-C9 子代理完整结果

子代理源：`bg_cbf76a15-df09-4b04-871e-4493d26adc8b` 完整 final_result。
**24 项全部有 file:line 或命令输出证据**。关键反转：

| 反转 | 真相 |
|---|---|
| `#3302` parser 覆盖率 60% → 80% | 实测 `cargo llvm-cov` = **60.58%**（目标未达），且 `debt-registry.yaml:285` 声称的 75.16% 不可复现 |
| `#4540` Q20 oracle sha256 验证 | oracle 侧（postgres/sqlite）SHA256 实际**校验通过**；但 `sqlrustgo/` 下**无 q20.tsv**（22 文件缺 1），`CROSS_ENGINE_HASH_CHECK.json` 记 `present: false`，`GA5_TPCH_SF1_REPORT.md:40` 声称 Q20 PASS 与产物缺失**直接矛盾** |
| `graph` ARCHIVED | 错——`Cargo.toml:44` 仍是 workspace member，`mysql-server/Cargo.toml:58` 仍作生产依赖，归档仅一份副本 |
| `vector` "used in storage" | 错——`sqlrustgo-storage` 依赖集**不含** vector；真实 rdeps 是 `sqlrustgo` + `gmp` |
| `check_coverage.sh` | 已 deprecated，`:2-6` 硬 `exit 2`；AGENTS.md 仍指引开发者跑它 |
| `G13` 门禁 | 当前 **exit 1**，缺 `tests/soak_test_harness.rs`，非"失联"而是真红 |
| C9 Page Checksum 门禁 | 假阳性——`check_storage_index_wal.sh` 判据仅"字段存在"；`page.rs:179` 实为 `offset += 4; // skip checksum` |
| C9 Q20 decorrelation | `try_decorrelate` 有定义但**零调用点**，planner 未接线 |

其余项的 verdict 在子代理 final_result 中以表格逐行列出。

### 9.5 GA 门禁补跑收尾（后台 task `bg_ed3fa344`）

收尾时（2026-09-30 23:51 后台回收）8 个 gate 中 7 个 exit 0，仅
`check_rc_ga_gate=1` 红；其余 `check_arch_invariants=0`、
`check_arch3_no_bypass=0`、`check_integration_gate=0`、
`check_anti_fabrication=0`、`check_arch_sem_debt=0`、
`check_cross_version_debt=0`、`check_int_debt=0`。本会话外其余 gates
的最近一次人工实跑证据见 `docs/releases/v4.1.0/evidence/gate-runs-2026-09-30/`
(2026-09-30 完成)。

---

## 7. 抽验记录（AFP §5.4 可追溯性）

主代理对子代理的关键结论做了独立抽验。**两轮抽验**：

### 第一轮：台账写作前（2026-04 段，4 项）

| # | 待验结论 | 抽验方式 | 结果 |
|---|---|---|---|
| 1 | v3.8.0 自认 23/36 统计误导 | `sed -n '78p' INT5_PLUS_DEBT_INVENTORY.md` | ✅ 原文一致 |
| 2 | v3.11.0 RC 从未达成 | `sed -n '3p' RC_BLOCKERS_REPORT.md` | ✅ "Status: RC gate not yet reached" |
| 3 | v3.12.0 RC 审计 3 项仍 FAIL | `grep '仍 FAIL' RC_ISSUE_CLOSURE_AUDIT.md` | ✅ `:105 ### 4.3 仍 FAIL (3/11)` |
| 4 | v3.9.0 两份 GA 文档冲突 | `sed -n '16p' COMPREHENSIVE` + `sed -n '129p' GA_GATE_REPORT` | ✅ "门禁全闭环，可投产" vs "GA cut NOT recommended" |

### 第二轮：WP-A + 门禁空转核实后（2026-09-30）

| # | 待验结论 | 抽验方式 | 结果 |
|---|---|---|---|
| 5 | `crates/parser/tests/wp_a_legacy.rs:68` 有 `#[ignore] // TODO: 多行注释解析需要修复` | `sed -n '68p'` | ✅ 原文一字不差 |
| 6 | 该 ignore 不在 `ignore_registry.json` | python3 脚本扫登记文件清单 | ✅ 全库命中 2 文件，本文件不在 |
| 7 | 实跑 cargo test wp_a_legacy 出 27 pass / 0 fail / 1 ignored | `cargo test -p sqlrustgo-parser --test wp_a_legacy` | ✅ |
| 8 | 块注释 `SELECT 1 /* comment */` parser 失败 | 临时探针文件 zz_wp_a_probe.rs（已删） | ✅ "Expected expression" |
| 9 | `CLAIM_DOWNGRADE_MANIFEST.md:28` 写 "280 v400 parser tests" | 实际数 v400_* 测试 + 块注释覆盖搜索 | ✅ 实际 336 个、零块注释覆盖；lexer 无 BlockComment token |
| 10 | `check_anti_ignore_gate.sh` 不扫源码 | 50 个新 `#[ignore]` 后门禁仍 exit 0（探针已删） | ✅ |
| 11 | registry 125 条无 `status` 字段 | python3 字段分布统计 | ✅ `{None:124, 'RETIRED':1}` |
| 12 | `debt-registry.yaml` 最后 commit 是 `76dae7465e` 2026-08-15 | `git log -1 --format` | ✅ 早前会话"自 2026-07-16 起未更新"说法有误 |
| 13 | `cargo llvm-cov --lib -p sqlrustgo-parser` = 60.58% | 子代理实跑输出 | ✅ |
| 14 | `sqlrustgo/q20.tsv` 不存在 | 子代理 `ls` + `CROSS_ENGINE_HASH_CHECK.json` 解析 | ✅ |

### C5-C9 子代理报告的子代理抽验

主代理对子代理 C5-C9 的 24 项结论逐条交叉：所有关键反转（#3302 60.58%、
#4540 缺 q20.tsv、graph ARCHIVED 假、vector 存储假、G13 真红、Page Checksum
假阳性、Q20 decorrelation 零调用点）均通过主代理的 grep / `cargo metadata` /
file:line 独立命令验证或与 STAGE.yaml / debt-registry 内部一致。可采信。

### 未抽验 / 未跑项

- 全部 `cargo test` 跑 mysql-server、parser 全套 —— token/时间预算限制
- `B-AUTH` 空密码认证 —— 需编译整个 mysql-server
- `FIX-GATE-TEST-INVENTORY` 当前 exit code —— 跑全 `cargo test` 超时
- D 组 14 项 —— 子代理 token 中断，本批**不主张完成**

---

## 8. 参考

- `docs/releases/v3.6.0/LEGACY_ISSUE_ANALYSIS.md`、`INTEGRATION_DEBT_REPORT.md`
- `docs/releases/v3.7.0/INTEGRATION_DEBT_REPORT.md`、`legacy/gate-chaos-audit/*`
- `docs/releases/v3.8.0/CHATGPT_ASSESSMENT_AND_CLOSURE_REPORT.md`、`debt/INT5_PLUS_DEBT_INVENTORY.md`、`debt/CROSS-VERSION-DEBT.md`
- `docs/releases/v3.9.0/ga/COVERAGE_GAP_RATIONALE.md`、`V390_COMPREHENSIVE_ASSESSMENT.md`、`GA_GATE_REPORT.md`、`GA_GATE_STATUS_REPORT.md`
- `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md`、`ARCHITECTURE_DEBT_ANALYSIS.md`、`V310_TASK_CLOSURE_VERIFICATION.md`
- `docs/releases/v3.11.0/LEGACY_DEBT_AUDIT_REPORT.md`、`LEGACY_DEBT_TRACKING_TABLE.md`、`RC_BLOCKERS_REPORT.md`、`plans/V311_DEBT_CLOSURE_PLAN.md`
- `docs/releases/v3.12.0/historical-backlog-disposition.yml`、`BLOCKER_DISPOSITION_V311.md`、`RC_ISSUE_CLOSURE_AUDIT.md`、`v312_issue_comments.md`、`storage-index-wal-backlog-report.md`、`CLAIM_DOWNGRADE_MANIFEST.md`
- `docs/governance/debt/debt-registry.yaml`
- `docs/releases/v4.0.0/{LEGACY_ISSUES,WP_LEGACY_TRIAGE,CLAIM_DOWNGRADE_MANIFEST,RC_GATE_REPORT,ALPHA_GATE_REPORT}.md`
- `docs/releases/v4.1.0/{LEGACY_ISSUES,ISSUES_PLAN,STAGE.yaml,ALIGNMENT_AUDIT_2026-09-30}.md`
- `crates/parser/tests/wp_a_legacy.rs`、`tests/baseline/ignore_registry.json`
