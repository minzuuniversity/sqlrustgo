## Why

Issue #5379 的标题是「TPC-H fixture 悬空符号链接导致 CI `test` job 恒红」。实测证明**这个标题只描述了表层症状**——
真正的问题面是三层叠加，且三层互相掩护，使得 CI 长期显示"接近全绿"的假象。

### 第 1 层：fixture 是指向仓库外的绝对路径符号链接

```
$ git ls-files -s tests/data/
120000 9fc4c45a5446d48bc2467529d86d82ef2280eeb8 0  tests/data/tpch-sf001   -> /tmp/tpch-sf001
120000 c58145a819ca46e7e783d64b3a20e8a356d250de 0  tests/data/tpch-sf01    -> /tmp/tpch-sf01
```

两个符号链接都指向**仓库外**的 `/tmp` 绝对路径，在任何其他机器（含 CI）都无法解析。

引入提交 `41fa140773`（2026-09-29）的说明是「Tracked the tpch-sf01 symlink so the 27 tests that reference it can find it after a clone」——
**该推理错误**：绝对 `/tmp` 路径只在作者本机成立。

`git ls-files | grep -c '\.tbl$'` = **0**——仓库里从来没有过任何 TPC-H 数据文件。
`.gitignore:28` 全局禁 `*.tbl`，`.gitattributes:3` 把 `*.tbl` 路由到 Git LFS，而该文件自己的注释写着
「Gitea (origin) has no LFS endpoint (has_lfs: None), so LFS-tracked pushes fail」——**数据在这条链路上根本进不了仓库**。
`scripts/generate_tpch_data.sh` 头部声称「committed fixtures in tests/data/tpch-sf001/ and tpch-sf01/ are LFS-tracked snapshots」，
与事实矛盾（入库的只有符号链接本身）。

### 第 2 层：fail-fast 把 382 个漏跑目标藏了起来

`cargo test --all-features` 默认 fail-fast，第一个失败目标就中止整个运行。
迭代 3 保存的 CI 完整日志（`/tmp/testjob.log`，2026-10-19 19:50–20:10，`test` job）实测：

| 指标 | 实测值 |
|---|---|
| Cargo.toml 声明的 `[[test]]` 目标 | **446** |
| 日志中实际 `Running <path>` 的目标 | **70**（其中 64 个是 `[[test]]` 声明目标） |
| **从未执行的目标** | **382（85.7%）** |
| CI 摘要打印 | `Tests: passed=70 failed=1` |

摘要里的 `70` 是**测试二进制个数**，不是测试数；它既没有说明 382 个目标从未运行，也把"1 个失败"呈现成几乎全绿。

违反仓库自身的两条契约：
- `AGENTS.md §7` 开发闭环第 7 步明确要求 `cargo test --all-features --no-fail-fast`——**CI 没加**
- `AGENTS.md §8` 第 5 条禁止「SKIP/TIMEOUT/XFAIL 计 PASS」

### 第 3 层：就算数据齐全，这些测试也证明不了任何事

- **13 个文件 `assert!` 数为 0**，只有 `eprintln!` 打印期望值。例如 `tests/integration/oracle/diag_q11.rs:73,84`
  打印「expect ~1060」「expect 80」却不断言；`diag_q11_having.rs` 同理。**查询结果全错它们也会通过。**
- `tests/integration/sql/four_way_harness.rs:174-185` 定义了 `compare_row_counts()`（跨引擎行数比对），
  但**全仓零调用点**——跨引擎相等性从未被断言过。唯一的 `assert_eq!`（`four_way_compare_test.rs:118-121`）
  只检查「sqlrustgo 自己不报错」，而 `passed()` 只看 `error.is_none()`——**返回 0 行也算 passed**。
- `scripts/gate/check_g1_tpch_baseline.sh:139-147` 第 4 步自称 "independent verification"，
  实际是 `tpch_hash_compare.py --check` → 内部再跑一次**同一个** `cargo test --test tpch_hash_test`——**典型自证**。
- `tests/oracle/baselines/tpch_sha256.json` 22 条里 **15 条 row_count = 0**，
  其 sha256 是常量 `sha256(0u64.to_le_bytes())` = `af5570f5a1810b7af78caf4bc70a660f0df51e42baf91d4de5b2328de0e83dfc`（已实测复现），
  对这 15 条查询**无鉴别力**；而 `tests/common/oracle_framework.rs:135-166` 的 `compare_to_baseline()` **只比较 row_count，从不读 sha256**。
- 另有 3 处**失效路径**必然导致失败：`scripts/gate/check_g1_tpch_22_22.sh:48,82` 引用两个已不存在的文件路径；
  `tests/integration/tpch/tpch_sf1_gate_contract_test.rs:39` 引用 MISSING 文件后 `.expect()` panic。
- `--dry-run` 一律绿灯：`check_tpch_sf1.sh` 即使 8 个 .tbl 全缺也 `EXIT=0`；
  `scripts/tpch_sf1_baseline.sh:162` 更无条件打印「fixture: $SF1_DIR (verified)」。

### 依赖面清点

**69 个** `[[test]]` 目标依赖一份 CI 上不存在的 fixture：

| 缺数据行为 | 目标数 | 其中会在默认 `cargo test` 中 panic | 其中带 `#[ignore]`（默认不跑） |
|---|---|---|---|
| P1 直接 panic | 59 | **34** | 25 |
| P2 显式跳过 | 7 | 0 | — |
| P3 其他（静默吞错 / 0 个 `#[test]`） | 3 | 0 | — |
| 合计（去重后 69，含 2 个自造数据） | | | |

fixture 来源不止悬空链接，还有 3 类死路径：仓库根 `data/`（**整个目录不存在**）、外部 `/tmp/tpch-sf1` 等绝对路径、
以及 `/home/openclaw/...` 这类**开发者本机绝对路径**（`q8_8way_date_range_regression.rs:26`、`q16_notin_subquery_regression.rs:25`、
`diag_q7_extract.rs:15`、`q2_5way_comma_limit_regression.rs:32`）。

### 治理契约对照

| 契约 | 要求 | 现状 |
|---|---|---|
| `AGENTS.md §7` 第 7 步 | `cargo test --all-features --no-fail-fast` | CI 用的是 fail-fast |
| `AGENTS.md §8` #3 | 不得删断言/跳过测试/改参考值来 PASS | 13 个文件本就无断言 |
| `AGENTS.md §8` #5 | SKIP/TIMEOUT/XFAIL 不得计 PASS | 7 个 P2 目标静默跳过即算 ok |
| ADR-008 §Policy 1 | 每个 PASS 声明必须披露「实际跑了多少」 | 摘要只给 `passed=70` |
| ADR-008 §Policy 2 | 门禁测试禁止 `#[ignore]` | 25 个 fixture 目标带 `#[ignore]`（2 个有例外，其中 v311 例外已于 2026-09-01 过期） |
| ADR-008 §Policy 3 / P16 | 门禁测试完整性由基线守护 | `tests/baseline/gate_test_baseline.json` 33 个门禁目标 |
| `TEST_PHASES_STANDARD.md §2` | TPC-H 属 **Beta/RC**（Alpha 只要求单元+集成） | TPC-H 目标混在 Alpha 级默认 `test` job 里 |
| `GATE_CONDITIONS.md` G4 | TPC-H SF=1 是**独立门禁**，需外部 `--sf1-dir` | 数据从未接入，却混在默认 job |
| `GATE_SCRIPT_STANDARD.md:160` | Gate 的 oracle 须为 DuckDB/SQLite | DuckDB 全仓未落地；SQLite 依赖已在但未接入门禁 |

## What Changes

### 1. CI 执行完整性（治"假绿"）

- `.gitea/workflows/ci.yml` 的 `test` job 改为 `cargo test --all-features --no-fail-fast`（对齐 `AGENTS.md §7`）
- 新增门禁 `scripts/gate/check_test_target_execution.sh`：解析 Cargo.toml 声明的 `[[test]]` 目标集合，
  与 cargo 日志中实际 `Running <path>` 的集合比对，**任一目标漏跑即 FAIL** 并打印漏跑清单
- CI 摘要按 ADR-008 §Policy 1 改为三态披露：
  `Tests: passed=<真实通过的测试函数数> failed=<N> not_run=<N> (declared_targets=<D> executed_targets=<E>)`
  ——`not_run` 的测试**不计入 passed**

### 2. 运行时自建 fixture（零入库，CI 内真跑）

- 新增 `scripts/gate/prepare_tpch_fixture.sh`：调用仓库既有的确定性生成器
  `scripts/gate/generate_tpch_sf.py`（`random.seed(seed)`、外键自洽）生成小规模 fixture，
  写 `tpch_fixture_manifest.json`（seed / sf / 各表实际行数），幂等
- CI 在 `cargo test` **之前**新增 "Prepare TPC-H fixture" 步骤，导出 `TPCH_DATA_DIR`
- **不向仓库提交任何 .tbl**：`.gitignore:28` 禁止 `*.tbl`、`:153` 记载用户明文规则
  「禁止 log, tbl, json 这类测试输出文件提交」、且有强制门禁 `scripts/gate/check_no_log_tbl_json.sh`

### 3. 统一 fixture 解析 + 三态模型

- 新增 `tests/common/tpch_fixture.rs`：`Fixture::resolve()` 按 `TPCH_DATA_DIR` → 候选路径解析；
  读取 manifest 得出 **profile**（seed / sf / 行数）；缺数据或 profile 不匹配时
  输出机器可读标记 `TPCH-NOT-RUN: <target> reason=<code>`，**不 panic、也不静默通过**
- **删除 2 个悬空符号链接**（`tests/data/tpch-sf01`、`tests/data/tpch-sf001`）：
  符号链接本身宣称"存在一份 fixture"，实际不存在，是比没有更坏的状态
- 69 个依赖目标统一改为三态：**PASS（已断言）/ FAIL / NOT-RUN（已披露）**；
  NOT-RUN 一律带原因码，计入 `not_run` 而不计入 `passed`

### 4. SQLite 独立预言机 + 13 个零断言文件补真断言

- 新增 `tests/common/sqlite_oracle.rs`：用**进程内** `rusqlite`（`Cargo.toml:252`，`bundled` 特性，
  CI 零外部依赖即可运行）载入同一份 fixture 并执行等价 SQL，产出归一化结果集
- 13 个零断言文件（`diag_q11*` 5 个、`diag_q12*`、`diag_q14*`、`diag_q6_*`、`diag_shipdate_type` 等）
  改为**真断言**，两类并用：
  - **SQLite 对照**：断言本引擎结果 == SQLite 结果（期望值来自独立引擎，**不是本引擎的输出**）
  - **语义不变量**：如「HAVING 阈值 0 保留的分组数 == 单独 GROUP BY 的分组数」「HAVING 阈值极大保留 0 行」
    「各分组聚合之和 == 整体聚合」（期望值来自数据自身的一致性，可杀死变异）
- 方言差异（SQLite 无 `EXTRACT(YEAR FROM ...)` 等）由适配层处理，未适配的查询列入显式清单

### 5. 已知差异登记（不制造假绿，也不掩盖真实缺陷）

- 新增 `tests/baseline/tpch_known_differences.json` + 门禁 `scripts/gate/check_tpch_known_differences.sh`：
  每条差异必须带 **issue 归属 + owner**，条目数**不得超过上限**（阻止无声增长），
  且 `actual_row_count == oracle_row_count`（差异已消失）时强制清理陈旧登记
- 范式对齐既有 P16 基线 `tests/baseline/gate_test_baseline.json`
- 约 20 个**数据依赖型期望值**目标（期望值标定在 dbgen SF=1 上，如「expect 80 rows」）
  在生成的 SF=0.01 fixture 上必然不符 → 由 **profile 守卫**判为 `NOT-RUN: reason=profile_mismatch`，
  **不改参考值、不删断言、不加 `#[ignore]`**

### 6. 变异测试证明断言有效

- 新增变异脚本，对**被测引擎路径**做人工变异（聚合漏项 / HAVING 阈值反向 / join 条件写错），
  断言必须 CAUGHT。变异对象是引擎实现而非测试代码——否则是自证

### 7. 规模层（SF=1）本轮不接入

依据（已实测）：CI **无法构建 dbgen**——仓库无 vendor、无 Makefile、CI 无安装步骤、
`scripts/generate_tpch_data.sh:180-185` 依赖开发者本机 `$HOME/work/tpch-dbgen/dbgen`；
且 `z6g4-tpch-sf1-cell-diff.yml:172-175` 记载 hermes-runner 的 **GitHub 出口被封**，
`actions/upload-artifact` 等 marketplace action 均不可用。
→ 规模层留作独立 issue + ADR-008 例外续期（v311 例外原定 2026-09-01 到期，**已过期**）。

## Impact

| 类别 | 影响 |
|---|---|
| 新增文件 | `scripts/gate/{prepare_tpch_fixture,check_test_target_execution,check_tpch_known_differences}.sh`、`tests/common/{tpch_fixture,sqlite_oracle}.rs`、`tests/baseline/tpch_known_differences.json` |
| 修改文件 | `.gitea/workflows/ci.yml`（test job）、13 个 `tests/integration/oracle/*.rs`（补断言）、约 20 个数据依赖目标（profile 守卫）、`tests/common/tpch_wire_harness.rs`（统一 fixture 解析） |
| 删除文件 | `tests/data/tpch-sf01`、`tests/data/tpch-sf001`（两个 mode 120000 悬空符号链接） |
| CI 行为 | `test` job 从 fail-fast 改为 no-fail-fast → **运行时间显著增加**（需实测），且 382 个此前从未执行的目标会首次暴露问题 |
| 风险 | 首次让 382 个目标真跑，可能暴露**与 TPC-H 无关**的既有失败 → 需在本迭代内分类（fixture 相关 / 真实缺陷 / 环境相关）并分流为 issue |
| 明确不做 | 不 vendor dbgen；不把 .tbl 提交进仓库；不改任何参考期望值来求绿；不把 not_run 计入 passed |
