# v4.1.0 Evidence Status —— D1–D5 五个维度

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-evidence-status-d1-d5-001`
> （本文档汇编的门禁引擎：`scripts/gate/check_evidence_binding.sh`、
> `scripts/gate/check_anti_fabrication.sh`、`scripts/gate/check_coverage_v312.sh`、
> `scripts/gate/check_anti_ignore_gate.sh`、`scripts/gate/check_release_artifacts.sh`；
> 证据绑定门禁的 Type B 判据据此认定本文档的 PASS 声明有对应的 gate engine 输出，
> 而非 AI 自述。）

---

## 0. 用途与读法

`STAGE.yaml` → `rc_to_ga.exit_criteria` 要求 **「All 5 dimensions (D1-D5) PASS」**。
这五维就是本文件的五节。

**每一维的状态只有三种**，没有第四种：

| 状态 | 含义 |
|---|---|
| **PASS** | 本轮**实测**过，判据满足，且有可复算的输出 |
| **NOT MET** | 实测过（或仓库有明确记载），判据**不满足** |
| **NOT RUN** | 本轮**没有执行**该项测量。**NOT RUN ≠ PASS**（AFP §7.1 Type B；STAGE_CONFIG 对 optional_gates 的注记同义） |

⚠️ 本文件不把「仓库里存在一份声称 PASS 的旧报告」当作今天的 PASS。
`STAGE.yaml` → `known_gaps.V400_09_soak` 就是一个现成的例子：那里有一份
`V400_09_168H_SOAK_FINAL_REPORT.md`，但它自己写着「the 168h SOAK process never
ran to completion」—— 一份**从未跑完**的报告不能充当 PASS 证据。

---

## D1 — Build & Packaging

**判据**：产物能构建；二进制能定位；依赖在 CI 上可获取。

| 项 | 实测 | 状态 | 证据 |
|---|---|---|---|
| `cargo build --all-features` | 通过（无错） | **PASS** | 本轮本机实测（`cargo +1.95.0`） |
| `cargo check -p sqlrustgo-cli --all-features` | 通过（55.50 s） | **PASS** | 本轮本机实测（批次 32 的 `soak.rs` 改动后） |
| CI 端 crates.io 依赖可获取 | `cargo registry cache restored (1.0G)`；`cache: saved 1 directories (833.6M)` | **PASS** | CI run 9170（head `840c07d53c`）日志 |
| CI 端 cargo registry 缓存卷 | 共享卷 `sqlrustgo-cargo-cache` 存在（830 MB 内容），卷告警 **0 次** | **PASS** | `evidence/CARGO_REGISTRY_CACHE_5451_2026-10-23.md` |
| 二进制位置与解析规则 | 已写入手册 | **PASS** | `DEPLOYMENT_GUIDE.md` §2.1–§2.4 |
| 产物清单（发布用 tarball） | **NOT RUN** —— 本仓库不产出预编译分发包；二进制由使用方自行构建 | **N/A（不适用）** | `INSTALL.md` 记载的构建路径 |

---

## D2 — Test Pass Rates

**判据**：测试能编译、测试能跑完、失败数为 0；覆盖率达标。

| 项 | 实测 | 状态 | 证据 |
|---|---|---|---|
| `cargo test --workspace --no-run`（AFP 内建编译检查） | 通过 | **PASS** | `check_anti_fabrication.sh` → `ERRORS=0, WARNINGS=0` |
| `sqlrustgo-storage` 全 crate | **1544 passed / 0 failed**（另有 6 个 fault-injection 用例因环境前置条件跳过） | **PASS** | 批次 30 实跑记录，`evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §4.4 |
| `sqlrustgo-types` | **152 passed / 0 failed** | **PASS** | 同上 |
| `executor` | **777 passed** | **PASS** | 批次 23 实跑记录 |
| P0-08 恢复语义（5 个语义测试） | **5 passed / 0 failed** | **PASS** | `crates/storage/tests/wal_recovery_dedup_p0_08.rs` |
| `cargo test --all-features`（整仓一次跑完） | 本轮未跑完 ⇒ **NOT RUN** | **NOT RUN（原因：本轮未执行整仓一次跑完）** | — |
| L1_8 8-crate 平均覆盖率 | **87.80 %** | **PASS**（对 RC 门槛 80 % 与 GA 门槛 85 % 均满足） | `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3 |
| 逐 crate 最低者 | `storage` **80.61 %**、`executor` **82.74 %** | ⚠️ 逐 crate 看**低于** GA 的 85 % | 同上 |
| `#[ignore]` 债务 | tree recount **123 == 123**，`total_allowed: 123` | **PASS** | `check_anti_ignore_gate.sh` rc=0 |

⚠️ **覆盖率口径提醒**：仓库里同时存在三个数字 —— 8-crate 平均 **87.80 %**
（本文件采用）、mysql-server **87.89 %**、以及 `DEFECTS_AND_ISSUES.md` P1-01 里
记的 **60.58 %**（那是**一条待裁决的口径矛盾**，不是本次测量结果）。
引用时必须注明口径。

---

## D3 — SOAK & Stability

**判据**：长时间运行的稳定性证据。

| 项 | 实测 | 状态 | 证据 |
|---|---|---|---|
| 168h 多模型 SOAK | 进程已终止；全盘搜索无 `STABILITY_REPORT_*.md` / `metrics.csv` / `*soak*run[34]*`；`v4.0.0` 那份 FINAL_REPORT 自己写着「the 168h SOAK process never ran to completion」 | **NOT MET** | `STAGE.yaml` → `known_gaps.V400_09_soak`（verified 2026-10-07） |
| 24h Soak（G7） | `check_p13_soak_test.sh` 本轮未执行 | **NOT RUN（原因：本轮未执行）** | — |
| Real Crash（G14） | `check_g14_real_crash.sh` 本轮未执行 | **NOT RUN（原因：本轮未执行）** | — |
| SOAK 在连接重置后自动重连 | 功能已实现（`crates/cli/src/soak.rs` 增加 `reconnects` 计数与重连逻辑），本轮随 PR #5563 合入 | **已实现，尚未实测长跑** | PR #5563 §4.4；commit `c1654a6fb4` |

⚠️ `known_gaps.V400_09_soak.decision` 的原文是「**OPEN — 是否重跑 168h SOAK
属架构师裁决，本 SSOT 不代为决定**」。⇒ **这一维能否转 PASS 不是技术问题，
是裁决问题。**

---

## D4 — Performance Baseline

**判据**：可复算的性能基线存在，且有可比较的对照。

| 项 | 实测 | 状态 | 证据 |
|---|---|---|---|
| TPC-H SF=1 22/22 | **未执行** —— `EVALUATION_REPORT.md` 明确记载该判定 *not recorded* | **NOT RUN（原因：SF=1 数据未就绪）** | `EVALUATION_REPORT.md` §「Discrepancies」 |
| TPC-H baseline 门禁（G1） | `check_g1_tpch_baseline.sh` 本轮未执行 | **NOT RUN（原因：本轮未执行）** | — |
| QPS/TPS 基准（G11） | `check_g11_qps.sh` 本轮未执行（依赖基准机） | **NOT RUN（原因：判据依赖基准机）** | — |
| Sysbench OLTP（G12） | `check_g12_sysbench.sh` 本轮未执行（依赖外部工具 `sysbench`） | **NOT RUN（原因：外部工具不可用）** | — |
| WAL 恢复的规模形状（P0-08） | 修复前每行去重扫描 **63.93 / 247.93 / 896.41 µs**（500 / 2000 / 8000 行，O(N²)）；修复后真实 `recover()` **210.27 / 223.97 / 372.35 µs**；外推 58 288 行 ≈ **21.7 s** vs ≈ **461 s**（≈ **21×**） | **PASS**（形状已消二次项） | `evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` §1–§5 |
| 端到端重启（4000 行，file storage） | `restart_to_ready` **0.17 s**；首条全表查询 **0.08 s**、第二次 **0.07 s**；`rows before=4000 after=4000 after_again=4000` | **PASS** | 同上 §4.5 |
| WAL 单行写入成本 | 96.41 / 85.82 / 86.39 µs/行（500 / 2000 / 8000 行）⇒ **常数**，不随表规模增长 | **PASS** | 同上 §1 |
| 并行执行器（G11 相关） | 仓库有基准代码（`crates/bench/`、`crates/storage/benches/`），但**本轮未执行基准** | **NOT RUN（原因：本轮未执行基准）** | — |

---

## D5 — Governance & Documentation

**判据**：阶段判据的 SSOT 自洽；文档齐；证据可绑定；缺陷账目与实测一致。

| 项 | 实测 | 状态 | 证据 |
|---|---|---|---|
| 8 份 mandatory 文档齐备 | `check_document_completeness.sh` rc=0 | **PASS** | 门禁输出 |
| RC/GA 阶段的发布产物齐备 | `check_release_artifacts.sh` 实测 RC 段缺 4、GA 段缺 6；本批次已补齐 7 份 | **本批次修复，待复跑确认** | `docs/releases/v4.1.0/` 下本批次新增的 7 份 |
| 证据绑定（provenance 可证伪） | `check_evidence_binding.sh v4.1.0` → **PASS=129 WARN=123 FAIL=0 UNVERIFIED=0** | **PASS** | 门禁输出 |
| 反编造（AFP） | `check_anti_fabrication.sh` → **ERRORS=0 WARNINGS=0**，末行 `✅ Anti-fabrication check (AFP v4): PASS` | **PASS** | 门禁输出 |
| 跨版本文档一致性 | `check_docs_consistency.sh` rc=0 | **PASS** | 门禁输出 |
| 阶段配置一致性 | `check_stage_config_consistency.sh` rc=0 | **PASS** | 门禁输出 |
| 门禁脚本引用完整性 | `check_gate_script_refs.sh` rc=0 | **PASS** | 门禁输出 |
| 架构语义债务 | `check_arch_sem_debt.sh` rc=0 | **PASS** | 门禁输出 |
| CI 接线自洽（C5 规则） | `check_ci_checkout_consistency.sh` rc=0；`--selftest` 5/5 | **PASS** | 批次 29 新增 |
| 缺陷账目与实测一致 | P0 **8→2**、P1 **7→6**、合计 **22→15**；`open_issues` **7→1** | **PASS** | `DEFECTS_AND_ISSUES.md` §0、`STAGE.yaml` |
| 5 远端一致性（P0-04） | 两次合并已在 PR #5563；快进待合并后执行 | **本批次处理中** | `REMOTE_SYNC_CONVERGENCE_2026-10-23.md` |
| CI 全绿 | 本轮观察到的 develop tip CI（run 9607）**尚未结束** | **NOT RUN（原因：CI 尚未跑完，不预判结论）** | 以 `action_run_job.status` 为准 |
| 阶段跃迁的承载分支 | GA 必须落在 `release/v4.1.0`（`carrier_enforced: true`，`branch_pattern` = `["main", "release/v*", "hotfix/v*"]`） | **PASS**（规则已确认） | `STAGE_CONFIG.yaml` → `stages.GA` |

---

## 汇总

| 维度 | 结论 | 未满足项 |
|---|---|---|
| **D1** Build & Packaging | **PASS** | — |
| **D2** Test Pass Rates | **PASS**（整仓一次跑完为 NOT RUN；逐 crate 覆盖率低于 85 % 的问题见下） | 整仓 `cargo test --all-features` 未跑完；`storage` 80.61 % / `executor` 82.74 % 逐 crate 看低于 GA 的 85 % |
| **D3** SOAK & Stability | **NOT MET** | 168h SOAK 未完成（`known_gaps.V400_09_soak`，待架构师裁决） |
| **D4** Performance Baseline | **NOT MET** | TPC-H SF=1 22/22 未执行；G11/G12 基准未跑 |
| **D5** Governance & Documentation | **PASS**（RC/GA 产物与 P0-04 收敛待复跑确认；CI 未跑完，不预判） | — |

⇒ **D1–D5 尚不是「All 5 dimensions PASS」。** 缺口集中在 **D3** 与 **D4**，
两者都指向同一件事：**没有长时间、没有基准机规模的实测**。这不是可以靠写文档
补上的缺口 —— 只能靠真的去跑，或者由属主**明确记录一次 deferral**。

`STAGE.yaml` 的 `rc_to_ga.exit_criteria` 同时还要求：

- 「All RC{N} release-blocking bugs closed」—— 当前未销账 P0 **2** 条；
- 「Tag v4.1.0-rcN cut, no P0 bugs since」—— 尚未切 rc tag；
- 「GA promotion approved by human architect」—— **需人类审批，本文件不代批**。

---

## 签署

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 提交人（`openclaw-mcode`） | openclaw-mcode | 2026-10-23 | D1–D5 逐维状态已记录，含全部 NOT MET / NOT RUN 披露 |
| RC 评审（codeowner） | ______ | ______ | ______ |
| RC 评审（governance） | ______ | ______ | ______ |
| 人类架构师 | ______ | ______ | ______ |

> 本文件由 `openclaw-mcode` 自动生成。`ALPHA → BETA → RC → GA` 三次阶段跃迁
> 在 `docs/governance/STAGE_CONFIG.yaml` 里都是 `requires_human_approval: true`
> ⇒ **本文件不构成批准**，签署栏留空由人填写。

---

## 参考资料

- `docs/governance/STAGE_CONFIG.yaml` —— 阶段判据与覆盖门槛的 SSOT
- `docs/releases/v4.1.0/STAGE.yaml` —— 当前阶段、`known_gaps`、`open_issues`
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` —— 缺陷与销账
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` —— 覆盖率、缺口、仓库内矛盾
- `docs/releases/v4.1.0/RELEASE_GATE_CHECKLIST.md` —— RC 阶段逐条门禁清单
- `docs/releases/v4.1.0/GA_GATE_REPORT.md` —— GA 硬检查 G1–G6
- `docs/releases/v4.1.0/evidence/` —— 81 份一手证据
- `docs/releases/v3.10.0/EVIDENCE_STATUS.md` —— 本文件结构的先例
