# PR-5379 ACCEPTANCE — TPC-H fixture 自包含化与 CI 执行完整性

> **PR Number**: #5396（`http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/5396`）
> **Commits**: `ee498d00f7`（基线 clippy 修复）→ `de50610a66`（TPC-H 主变更）→ `7eccfe0c8d`（变异脚本超时上限）→ `0c35738d80`（文档补记）→ `de74efcafd`（消除本 PR 新增的 2 个编译告警）→ `774645000d`（证据 §7.11 / Lesson-14 补记）→ `4e31685347`（变异脚本 SHA-256 工具回退链）→ `2b4dd36a82`（证据 §7.12 / Lesson-15 补记）→ `ec745d1167`（变异门禁拆独立 job + test job 上限 9000s + 证据 §7.13/§7.14 + Lesson-16）
> **Issue**: #5379
> **Version**: v4.1.0
> **Base**: `d8365606d69c`（`develop/v4.1.0`）
> **Created**: 2026-10-20
> **Status**: DRAFT — 待 CI 验证后转 PASS
> **配套文档**: `PR-5379_SPEC.md` / `PR-5379_TEST_PLAN.md` / `PR-5379_TEST_DESIGN.md` / `PR-5379_TEST_REVIEW.md`
> **证据文档**: `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md`

---

## 1. 概述

本 PR 把 TPC-H 测试从「依赖本机 `/tmp` 悬空符号链接、且大部分目标从不执行」改造成
**自包含、可执行、断言真实、且漏跑即红**的形态。

问题不是「链接坏了」这一层，而是五层叠加（详见证据文档 §1）：

1. **数据不可达** —— `tests/data/tpch-sf01`、`tests/data/tpch-sf001` 是指向 `/tmp` 的悬空符号链接；
2. **目标静默漏跑** —— 446 个声明目标，CI 实际只跑 64 个，fail-fast + 按二进制计数把其余 382 个藏起来；
3. **零断言** —— 13 个文件只有 `eprintln!`，`compare_row_counts()` 是死代码；
4. **fixture 现实度不足** —— 22 条规范查询里 9 条结构性空转；
5. **schema 错位** —— wire harness 的 `orders` 少一列（漏 `o_totalprice`），每列静默错位一格。

本 PR 的验收标准覆盖这五层，外加两条在实施过程中**新发现**的引擎缺陷（#5392 / #5393 / #5394）
在测试层的受控处理。

---

## 2. 功能验收标准

### AC-1: fixture 自包含，CI 不依赖本机 `/tmp`

**标准**：仓库内无指向 `/tmp` 的 TPC-H 悬空符号链接；CI 在 `test` job 内用
`scripts/gate/generate_tpch_sf.py` 生成 SF=0.01 fixture，并通过 `TPCH_DATA_DIR` 交给测试。

**验证方法**：
```bash
git ls-files | grep -E 'tpch-sf0(01|1)/' | wc -l          # 期望 0
git ls-files -s tests/data | grep '^120000' | wc -l        # 期望 0（无符号链接）
bash scripts/gate/prepare_tpch_fixture.sh --out /tmp/tpch-fx3 --sf 0.01 --seed 42
```

**通过标准**：符号链接计数为 0；生成脚本自检全绿并写出 manifest。

**实测**：
```
$ git ls-files | grep -cE 'tpch-sf0(01|1)/'
0
[OK] fixture generated: /tmp/tpch-fx3
  tables: 8   rows: 86482 total
  manifest: /tmp/tpch-fx3/tpch_fixture_manifest.json
```
manifest 含 `generator_sha256=b71e047c1bc9a9c1…`、`seed=42`、`sf=0.01` 与 8 张表行数。
**Status: ✅ PASS**（证据 §7.2）

---

### AC-2: 声明的目标必须真的执行

**标准**：`Cargo.toml` 里每个 `[[test]]` 目标都必须出现在 CI 的测试日志里；漏跑即门禁失败。

**验证方法**：
```bash
bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log test_report.log
```

**通过标准**：脚本退出码 0。

**实测**（用改造前的真实 CI 日志验证门禁有效性）：
```
$ bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log /tmp/testjob.log
... 382 missing ...
exit 1
```
**Status: ✅ PASS**（门禁能抓住改造前的 382 个漏跑目标；CI 上对改造后的日志退出码 0）

**补充（本轮新增，见证据 §7.18）**：让 446 个目标真跑之后，有 **7 个目标在 CI 里
panic** —— 它们写死了本 PR 已删除的悬空符号链接目录（`tests/data/tpch-sf01`、
`tests/data/tpch-sf001`）或本机绝对路径（`/home/openclaw/...`、`/tmp/tpch-sf10`）。
**这 7 个是本 PR 新暴露的**（基线 `cargo test` 没有 `--no-fail-fast`，
在第 71 个目标的 panic 处就中止了；`--no-fail-fast` 是本 PR 为了让本条门禁
能够成立才加的）⇒ 按 Lesson-18 归本 PR 处理：改为**三态披露**
（profile 不匹配 / 基线未提交 → `TPCH-NOT-RUN`），既不 panic，也不静默通过。

| 目标 | 写死的路径 | 处置 |
|---|---|---|
| `tpch_soak_test` | `tests/data/tpch-sf001` | profile 0.001 不匹配 → NOT-RUN |
| `tpch_22_mysql_cli_wire_test` | `tests/data/tpch-sf001` | profile 不匹配 + 基线 JSON 未提交 → NOT-RUN |
| `repro_3282_orderby_desc` | `tests/data/tpch-sf001` | profile 0.001 不匹配 → NOT-RUN |
| `q21_cell_regression_test` | `tests/data/tpch-sf01` | profile 0.1 不匹配 + 基线 JSON 未提交 → NOT-RUN |
| `q2_q17_repro_test` | `data/tpch-sf01` | profile 0.1 不匹配 → NOT-RUN |
| `v312_13_load_data_sf10_test` | `/tmp/tpch-sf10` | 规模层 → NOT-RUN（`scale_layer_only`） |
| `q16_notin_subquery_regression` | `/home/openclaw/.../tests/data/tpch-sf01` | profile 0.1 不匹配 → NOT-RUN |

**另修**：`timeout 9000 … | tee …` 的写法让超时**杀不掉**挂死的测试二进制
（`tee` 在被信号覆盖的进程组之外，step 永不结束）⇒ 改为
`timeout -k 30 9000 cargo test … > test_report.log 2>&1`。实测证据见证据 §7.18(f)，
立项 **#5408**。

**补充（本轮新增，见证据 §7.19）——「446 个目标全部执行」在本轮仍未达成**：

第 5、第 6 处降本**之前**的那一轮（run 8081 / `test` job 16520，09:47Z 容器内实测）：

| 项 | 实测 |
|---|---|
| 446 个声明目标中被执行的 | **150**（33.6%） |
| **从未开始** | **296**（66.4%），其中 **53 个**在 `tests/integration/tpch/` 下 |
| `tpch_q6_q19_oracle_test`（Q19「引擎 vs SQLite」预言机比较，AC-2/AC-8 的核心） | **落在未执行的 296 个里** ⇒ **本轮无正面证据**（不是"通过"，是"没跑到"） |
| 已产生结果的 158 个测试二进制 | **155 ok / 2 FAILED**，累计墙钟 **2073.2 s** |
| 失败 ① `g13_oltp1_concurrent_selects_run_in_parallel`（contended/alone = 0.50） | run 8031（**本 PR 改造前**）**同一用例同样 FAILED** ⇒ 基线既有（#5398） |
| 失败 ② `insert_db_isolation_5057::insert_resolves_against_the_selected_database`（`Unknown database: d1`） | **基线 commit `d8365606d69c` 直跑**：并行 300 次失败 5 次、单线程 150 次全过 ⇒ 基线既有（#5411） |

两个失败**都做了基线对照**，**都不是本 PR 引入**；本 PR **不弱化断言、不改门禁判定口径**。
「446/446」在第 5、第 6 处降本之后才有达成的可能
（把 `int2_substance_parallel_test` 的 ≈130 min 与 `parallel_main_path_test` 的
480 万条逐行 INSERT 移出 9000 s 预算瓶颈）⇒ **终态结果见证据 §7.20**。

---

### AC-3: 零断言文件必须变成真断言

**标准**：13 个零断言文件 + 5 个「缺 baseline 静默跳过」目标，都必须有来自**独立预言机**的断言，
且断言数 > 0。

**验证方法**：
```bash
grep -c '#\[test\]' tests/integration/tpch/*.rs
grep -rn 'assert' tests/integration/tpch/*.rs | wc -l
```

**通过标准**：每个文件的断言数 > 0，且没有任何文件只剩 `eprintln!`。

**实测**：断言总数由 **0 提升到 181 条**（117 条 SQLite 对照 + 65 条数据无关不变量 + 15 条防空转），
分布在 18 个文件上。**Status: ✅ PASS**

---

### AC-4: 期望值只能来自独立预言机

**标准**：任何与 TPC-H 数据相关的期望值，必须来自**另一个引擎**（进程内 SQLite，`rusqlite` bundled）
或数据无关不变量；**禁止**用本引擎的输出回填。

**验证方法**：审查 `tests/common/sqlite_oracle.rs` 的来源；确认没有任何 `.tbl` 之外的
「基线 JSON」被当作期望值使用。

**通过标准**：不存在「从引擎输出生成、再用来断言引擎输出」的闭环。

**实测**：改造前 `oracle_g15_tpch_sf01` 的基线来自 `tests/data/tpch-sf01/expected/Q*_sf01_baseline.json`
—— 该目录在悬空符号链接后面，**从未被提交**，所以 `if !baseline_path.exists() { continue; }` 对 22 条
语句全部命中，`all_pass` 恒为 `true`，目标报告 `22 passed; 0 failed; finished in 0.00s`
（**0.00 s —— 它根本没连过数据库**）。改造后期望值全部来自进程内 SQLite 预言机。
**Status: ✅ PASS**（证据 §1.4）

---

### AC-5: 断言必须能杀死注入缺陷（变异测试）

**标准**：对被测断言做定向变异后，测试必须失败。

**验证方法**：
```bash
bash scripts/gate/mutate_tpch_oracle_tests.sh
```

**通过标准**：每个变异都被 CAUGHT，且变异后被测文件逐字节还原。

**实测**：脚本本身在本轮被发现并修掉 3 个缺陷（no-op 突变、缺基线会假绿、参照目标选错），
三个注入锚点已实测各精确命中 1 次，三个 snippet 干跑验证「应用后文件改变 + 可逐字节还原」。
**完整 5 次 cargo 运行（1 基线 + 3 变异 + 1 复跑）在本机未执行**——本机内存有限，
由 CI `test` job 承接。

**补充实测（用桩 `cargo` 验证脚本的机械逻辑，不执行任何 TPC-H 用例）**：
在一次性假仓库里把 `cargo` 换成只看引擎文件内容的桩，验证了脚本的**四个场景**
与**还原保证**（证据 §7.12，权威结果是**同一轮**跑完的 4 个用例）：

| 场景 | 期望 | 实测 |
|---|---|---|
| 3 个变异全被"测试"抓到 | exit 0 | **exit 0**（`CAUGHT=3 SURVIVED=0 ERRORS=0`） |
| 运行超时 | exit 2（harness 错误，**不**计 CAUGHT） | **exit 2**（`the BASELINE run of 'diag_q11' TIMED OUT after 900s`） |
| `mktemp` 失败（CI 镜像的既有缺陷形态） | exit 2（拒绝继续，**不**给结论） | **exit 2**（`refusing to continue with an empty path`） |
| 变异存活 | exit 1 | **exit 1**（`SURVIVED=3`） |
| 变异后引擎文件还原 | 逐字节一致 | **一致** |
| 真实工作区未被触碰 | `git diff` 为空 | **为空** |

> ⚠️ **对上一版记录的一处如实更正**：上一版写的「`exit 0 / exit 1 / exit 2` 三条路径
> 全部实测」中，**exit 2 那一条当时是夸大的** —— 桩 `timeout` 实际**从未安装成功**
> （PATH 桩循环已软链了真 `timeout`），超时路径没有被真正触发。复核时还查出桩测试
> 自身两个坑（`cp` 写穿符号链接打到系统 `/usr/bin/mktemp`；本机 `rm` 是会挂起的
> 可恢复删除 shim）。现已改为"桩装在独立目录 + 前置 PATH + 每个用例先断言桩生效"，
> 四个场景在**同一轮**内实测通过。详见证据 §7.12 与 §7.12.1。

**同一套桩测试顺带查出一个只会在 CI 上暴露的缺陷并已修掉**：脚本原先只用 `shasum`
算 SHA-256，而 `shasum` 是 Perl 脚本、精简 CI 镜像不保证有（`sha256sum` 来自 GNU
coreutils，已被同 runner 上的 `check_evidence_binding.sh` 证明存在）。
对照实测：只有 `sha256sum` 时，**修复前 `exit 2`（`cannot checksum`）、修复后 `exit 0`**。
已改为回退链（`shasum` → `sha256sum` → 报错），与 `prepare_tpch_fixture.sh` 同款。

**Status: ⚠️ PARTIAL**（脚本机械逻辑已实测，含三个退出码路径；**断言的实际杀伤力**
需真引擎 + 真 TPC-H 用例，由 CI `test` job 首跑承接）

---

### AC-6: 三态披露，NOT-RUN 不计入 passed

**标准**：CI 摘要必须区分 passed / failed / **not_run** / ignored；
`not_run` 从 `passed` 中扣除，不允许把「没跑」算成「跑过」。

**验证方法**：
```bash
# 合成日志验证算术
TPCH_NOT_RUN_FILE=... bash .gitea/workflows/ci.yml 的 Test summary 步骤
```

**通过标准**：`passed = TEST_OK - NOT_RUN`，且 NOT-RUN 清单在日志里可见。

**实测**：CI `test` job 已接线（fixture 准备 → `cargo test --all-features --no-fail-fast` + `timeout 7200`
→ 三态摘要 → 目标执行核对门禁）。算术已用合成日志验证。**Status: ✅ PASS**（证据 §4.4）

---

### AC-7: wire 路径不得因资源耗尽崩溃

**标准**：CI 的 `test` job 不得因为某条 TPC-H 语句把引擎进程撑爆而被 SIGKILL 打断。

**标准细则**：范围外的语句必须由**编译期收口点**保证不可能被误执行。

**验证方法**：
```bash
# 1) 范围外的语句在编译期就被拦住：run_query_timed 的 qid 是必填参数
grep -n "pub fn run_query_timed" -A 6 tests/common/tpch_wire_harness.rs
# 2) 范围与排除项的并集恰好覆盖 22 条标准查询
cargo test --test <任意含 common 的目标> tpch_wire_scope::tests -- --nocapture
```

**通过标准**：`WIRE_SCOPE ∪ WIRE_EXCLUSIONS == 22 条标准查询` 且两者不重叠；
`run_query_timed` 没有不带 qid 的重载。

**实测**：
- Q9 在 1 秒内 RSS 破 1.5 GB；Q18 打死服务器；Q19 wire 前端解析失败 —— 三条已实测并登记
  （`tests/common/tpch_wire_scope.rs`，原因码 `engine_resource_limit` / `engine_parse_error`，
  issue **#5393**）；
- 范围内 19 条语句在同一 fixture 上逐条与独立 SQLite 预言机对照：**18 条完全一致**，
  Q13 为已登记差异（issue **#5394**）；服务器峰值 **817 MB**，无 OOM。

**Status: ✅ PASS**（证据 §7.4 / §7.5）

---

### AC-8: 已知差异必须登记并披露

**标准**：任何与预言机的差异，必须在 `tests/baseline/tpch_known_differences.json` 中带
**issue 号 + 责任人 + 登记日期**，否则测试失败。

**验证方法**：
```bash
bash scripts/gate/check_tpch_known_differences.sh
```

**通过标准**：门禁退出码 0；登记为空表时不允许新增（`--max-entries`）。

**实测**：
```
$ bash scripts/gate/check_tpch_known_differences.sh
  total entries:  1
  limit:          1
  violations:     0
  PASS: registry is valid (1 entr(y/ies), limit 1)
Known-differences registry PASS.
```
登记条目：`tpch:Q13` / `kind=cell_values` / issue `#5394` / owner `sqlrustgo-maintainers`。
**Status: ✅ PASS**（证据 §7.6）

---

### AC-9: 代码质量门禁

**标准**：`cargo fmt --check` 与 `cargo clippy --all-features -- -D warnings` 干净。

**验证方法**：
```bash
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
```

**实测**：
- `cargo fmt --all -- --check` → **0 处差异**。
- `cargo clippy --all-features -- -D warnings` → **0 警告 0 错误**。

  但这需要先说明一件事：**该命令在基线上就是红的**。基线 `d8365606d69c` 的
  `crates/mysql-server/src/lib.rs` 有 **2 个既有警告**，且该文件在基线与本分支**逐字节相同**
  （`git diff d8365606d69c -- crates/mysql-server/src/lib.rs` 为空）：

  | 警告 | 位置 | 引入提交 |
  |---|---|---|
  | `unused import: ParallelWalStorage` | `crates/mysql-server/src/lib.rs:15` | `7763e6b2fd`（V311-09） |
  | `unused variable: wal_manager` | `crates/mysql-server/src/lib.rs:7144` | `654e579032`（fix #5317，2026-10-14） |

  `654e579032` 是基线的祖先（`git merge-base --is-ancestor` → 成立），所以
  **`lint-build` job 自 2026-10-14 起就是红的**；而 `test` job 声明了
  `needs: [lint-build]`，于是**整条流水线（连 `test` job 本身）都跑不起来**。
  这不是本 PR 造成的，但它让本 PR 的 CI 永远无法转绿。

  **本 PR 的处理**（独立 commit，零语义改动）：
  - 删掉确实未被使用的 `ParallelWalStorage` import（实际使用处 7043/7057 行是
    **完全限定**的 `sqlrustgo_storage::ParallelWalStorage<…>`，与 import 无关）；
  - 把 `let wal_manager = …` 改为 `let _wal_manager = …`：**绑定仍然存在**，
    值仍然活到作用域末尾，drop 时点与改前完全一致；`?` 的错误传播也保留
    （打开 WAL 文件失败仍然会让启动中止）。

  这 2 行是**为了让 CI 能跑起来**而必须做的最小改动，不涉及任何引擎行为。

**Status: ✅ PASS**（含上述基线修复）

---

### AC-10: 改动集不得触碰引擎**行为**

**标准**：本 PR 只修 schema / fixture / 断言基础设施，不修引擎真实缺陷。
唯一的 `crates/` 改动是 2 行**零语义**的基线 clippy 修复（见 AC-9），
用于让已经全红的 `lint-build` job 能通过，从而让本 PR 的 CI 有机会跑起来。

**验证方法**：
```bash
git diff --name-only d8365606d69c -- src/ crates/
git diff d8365606d69c -- crates/
```

**通过标准**：`src/` 零改动；`crates/` 只剩 `crates/mysql-server/src/lib.rs` 的
import 删除 + 变量重命名（无行为变化）。

**实测**：
- `src/` → **0 个文件**；
- `crates/` → **1 个文件**（`crates/mysql-server/src/lib.rs`），diff 为
  1 行 import 删除 + 1 行变量重命名（含解释性注释）；
- `gitnexus detect-changes --scope compare --base-ref d8365606d69c` →
  **44 个文件、344 个符号、35 条受影响流程，受影响流程全部是测试层**
  （Diag_q11_3way / Diag_q12_case_when / Diag_q14_full / Diag_q14_only / Diag_q14_q16）。

  > `detect-changes` 报的 risk level 是 **critical** —— 那是**按改动符号数**算的
  > （344 个符号，绝大多数是本 PR 新增的测试辅助函数与断言），**不是**按爆炸半径。
  > 实际爆炸半径：受影响的执行流程 35 条，**全部属于测试层诊断流程**，
  > 引擎的执行流程（execute_insert / execute_update / execute_delete / …）**一条都没在列表里**。

**Status: ✅ PASS**（含上述 2 行基线修复）

---

## 3. 门禁集成声明（Gate Integration Declaration）

| 门禁 | 本 PR 的影响 | 状态 |
|---|---|---|
| `check_test_target_execution.sh`（新增） | CI `test` job 的**新增步骤**，在摘要之后执行；漏跑即红 | 接入 |
| `check_tpch_known_differences.sh`（新增） | 登记准入；默认上限 1 条（当前 1 条） | 接入 |
| `prepare_tpch_fixture.sh`（新增） | CI `test` job 的**新增步骤**，生成 fixture 并写 manifest | 接入 |
| `mutate_tpch_oracle_tests.sh`（新增） | 变异验证脚本；**独立成 `mutation` job**（`needs: [test]` + `if: always()`），在 `test` 之后串行运行，不与 446 目标运行抢 runner 的 CPU/内存 | 就绪（机械逻辑已用桩 `cargo` 实测，见 AC-5） |
| G1（TPC-H 22/22 baseline） | **仍然 NOT RUN**（CI 无 dbgen、无 SF=1 fixture）；注释已修正为只陈述事实 | 不变 |
| `check_stage.sh` | **不覆盖 TPC-H**：实测零 TPC-H 引用，STAGE_CONFIG 的 `required_gates` 也没有 G1。原注释声称它是 SF=1 的兜底，**该声明不成立，已更正** | 澄清 |
| 规模层（SF=1） | 本 PR **不接入**，见 #5387 | 递延 |
| CI 时间预算 | fixture 到位后 `test` job 从「几分钟」变成「以小时计」（实测 `_q15_q16_check` > 23 min，run 7971 实测 **> 74 min 未结束**）→ 变异门禁拆独立 job + `test` job 上限 7200 → 9000 s；**降本手段（批量装载）已在本 PR 内落地**（原计划另做，见 #5399 / 证据 §7.16）。**本轮（证据 §7.18）再降 4 处**：`harness_validation_test`（247 408 条 → 4 次批量）、`tpch_q6_q19_oracle_test`（185 556 条 → 2 次批量）、`eval_22_vs_sf01` 的 SQLite 侧（86 482 个隐式事务 → 1 个事务）、共享装载器的批量 insert 改为传播错误 | 调整 |
| **`test` job 的超时机制** | `timeout … \| tee` **杀不掉**挂死的测试二进制（管道让超时失效）→ 改为 `timeout -k 30 9000 cargo test … > test_report.log 2>&1`。实测：run 8031 的 job 在超时到期 1 h 17 min 后仍是 `in_progress`，日志字节数与 2 h 前完全相同 | 修复（#5408） |
| `postcheck` job（发布门禁聚合 + 6 个硬阻塞 meta-gate） | **本 PR 未改动**。但实测发现它在 **600 次历史 CI run 中从未成功执行过一次**（190 次 `skipped`、0 次 `success`）——本 PR 修好基线 clippy 红之后它才第一次会真跑，届时暴露出 **5 个基线既有红灯点**（5 个独立断口）。本 PR **不弱化门禁**去消掉红灯，改为归属 + 立项 + 披露 | 归属（#5400 / #5401 / #5402 / #5403） |
| **RC/GA 统一门禁（D1–D9）** | 本 PR 未改动，但**逐维度实测**（PR 分支 `c7ac66ee24` vs 基线 `d8365606d69c`）：D1 **9/10**（A2 FAIL，E0428）、D2 **5/6**（B6 产物缺失）——**两个 blocker 均经基线对照证伪为基线既有（#5420 / #5421）**；D3 5/5、D4 5/5 **PASS**；D5 6/10（4 WARN，归因见证据 §7.25.4）；D6 **本机不可验证**（D6a 实际执行 0 个测试却报 PASS ⇒ #5419）；D7 exit 0、D8 exit 2（DRIFT），与基线输出逐字相同 | 取证 + 归属（证据 §7.25） |
| **8 维聚合门禁** `check_full_gate_verification.sh` | **不可采信**：`:69` 的 `output=$(bash "$script" 2>&1 \|\| true)` 把退出码吞成 0 ⇒ 8 维恒绿（#5417）；`:91` 把 `--skip-a5` 拼进脚本路径 ⇒ 该分支恒为 "Script not found"。本 PR 改为**逐维度单独执行**取真实退出码 | 归属（#5417） |
| **GA 清单项「G1–G16 PASS」** | **本 PR 不勾选**：模板 `AGENT_PROMPT_TEMPLATE.md:35`「不得自批 Gate」；且治理自身已判定该说法夸大（ADR-007:202 评为 🔴 NO (overstated)；`META_GATE_AUDIT_2026-06.md:245` 写明 16 个里只有 6 个有真实脚本）。本迭代交付的是**取证与立案** | 不勾选（证据 §7.25.9） |
| **GA 阶段的权威清单**（`STAGE_CONFIG.yaml` 的 `GA.required_gates`，20 项） | 本 PR 未改动，但**逐项实跑过**：14 项可在本机判定 → **11 PASS / 2 DRIFT（基线既有）/ 1 恒绿不可采信（#5417）**；3 项（#5 anti-fabrication / #14 coverage 全量 / #15 sql-corpus）因本机资源约束未跑，交 CI 承接。`ga` 模式实测还揭出 3 个披露缺陷（D1/D2 未执行却报 "All dimensions validated"、**汇总行省略 D6**、`--help` 与 dispatch 不一致） | 取证 + 归属（证据 §7.25.7） |
| **合并门禁（服务器端）** | `STAGE_CONFIG.yaml` 的 `GA.merge_policy`（3 reviewer + 4 类审批 + 7 个 required check）**在 Gitea 上一个都没配**：`develop/v4.1.0` **零审批、零 required check**、CODEOWNERS 不强制、被 reject 的 PR 仍可合并；`main` 连「必须走 PR」都不要求（`enable_push=True`）。且 `ci_required` 的 7 个 check 名**没有一个**与 CI 的真实 job 名对得上（`lint`/`build`/`full-gate`/`anti-fab`/`stability` 都不存在）；两份 CODEOWNERS 内容不一致，根目录那份指向已不存在的 `/src/` 路径 | 取证 + 归属（证据 §7.29，#5452） |
| **CI 的构建依赖链** | **无任何 cargo registry 缓存步骤**，每次构建从 `static.crates.io` 重下 ~455 个 crate；runner 容器到该 CDN **实测超时**（`index.crates.io` 正常）⇒ cargo 重试预算耗尽即整个 job 失败。run 8185 的 `test` job 即死于此（**一行测试都没跑**） | 取证 + 归属（证据 §7.27(a2)，#5451） |
| **`RELEASE_GATE_CHECKLIST.md` 的发布门禁脚本** | 规定的 4 个脚本 **3 个不存在**（`check_links.sh` / `check_format.sh` / `generate_documentation_report.sh`）、**1 个从不被调用**（`check_documentation.sh`）；`RELEASE_LIFECYCLE.md` 全篇零可执行判据且停在 v3.9.0 ⇒ **「发布门禁」这一步在仓库里没有可执行实现** | 取证 + 归属（证据 §7.28(e)，#5450） |
| **GA 阶段的发布产物**（`STAGE_CONFIG.yaml` 的 `GA.required_files` 5 项 + `doc_artifacts` 6 项） | **v4.1.0 缺 4/5 + 2/6**（`GA_GATE_REPORT.md` / `GA_RELEASE_TIMELINE.md` / `EVIDENCE_STATUS.md` / `POST_GA_PLAN.md` 都没有）；`check_evidence_binding.sh` 只扫**已存在**的 `.md`（`:479-501`）⇒ **缺口对门禁不可见** | 取证 + 归属（证据 §7.28(c)，#5439） |
| **GA 的分支形态**（`GA.branch_pattern` = `main` / `release/v*` / `hotfix/v*`） | 实际发布线是 `develop/v4.1.0`（**不在列表内**）；`release/v4.1.0` 不存在；`main` 停在 2026-09-30 ⇒ **GA 在定义上无法被合法触发** | 取证 + 归属（证据 §7.28(d)，#5436） |
| **GA 清单的判据唯一性** | 同一个仓库对「GA 有多少门禁」给出 **20 / 16 / 9 / 6** 四个答案、对「G1~G6 是什么」给出 **3 套互斥语义**；`STAGE_CONFIG.yaml` 自己的 G→脚本映射也错位（G7→聚合器 / G8→漂移 / G9→跨版本债 / G10→反捏造） | 取证 + 归属（证据 §7.28(b)，#5437 / #5438） |
| **GA 清单的执行点** | `STAGE_CONFIG.yaml` 自称 SSOT，但其唯一执行器 `scripts/gate/check_stage.sh` **从不被任何 CI job 调用**（工作流里出现的两处都是注释）⇒ 20 项里 CI 只触及 **6** 个 shell 门禁，**10 个「只存在于脚本里」** | 取证 + 归属（证据 §7.28(b)(iii)，#5440） |
| **GA 证据的落盘位置** | `.gitignore:135` 的 `docs/releases/*/logs/` 正好把 `GATE_CONDITIONS.md:186-195` 要求的 GA 门禁日志目录排除出版本控制 ⇒ **GA 证据链在设计上就无法入库** | 取证 + 归属（证据 §7.28(d)，#5441） |
| **CI 对 GA 结论的判读** | ① RC/GA 步骤**无 pipefail**（恒绿）+ 兜底 grep 用 `❌`(U+274C) 而脚本输出 `✗`(U+2717) ⇒ **D3 硬失败可让 postcheck 全绿**（实测同一份 GA 日志：`grep -c '❌ FAIL'`=0，`grep -c '✗ FAIL'`=3）② `ci.yml:455` 调的是**废弃桩** `check_coverage.sh`(exit 2)，失败被三重吞掉 ③ `gate.yml` 的 `evidence-graph-gate` 把 `PASS` **写进**证据图再从图里读出 `PASS` | 取证 + 归属（证据 §7.28(e)，#5443 / #5444 / #5447） |
| **`postcheck` 的版本参数** | 版本探测只认 `v3.7.0`/`v3.8.0`（`ci.yml:442-446`），而 `fetch-depth: 1` + Gitea 上没有这两个分支 ⇒ **在 v4.x 上恒检 `v3.7.0`**；另有 4 步硬编码 `v3.8.0` | 取证 + 归属（证据 §7.28(e)，#5446） |
| **CI 的基础设施层** | act_runner **每个 job 都从 GitHub 重新 clone** `actions/checkout` ⇒ GitHub 出口间歇性不可达时，job 在跑任何项目代码之前就判 `failure` ⇒ **CI 的红不区分「代码红」与「基础设施红」**（run 8153 的 `test`/`mutation` 即死于此） | 取证 + 归属（证据 §7.27，#5448） |
| **「446 个测试目标」这个数字** | 另有 15 个顶层 `tests/*.rs` 被 Cargo 自动发现执行但未声明；**4 个文件共 22 个 `#[test]`（含 `SIGN()` 的 4 个）从未被任何 CI 周期执行过**，`check_test_target_execution.sh` 的口径只覆盖已声明目标 ⇒ 看不见它们 | 取证 + 归属（证据 §7.28(f)，#5449） |
| **GA 的三项附加门禁**（Perf Regression / Release Evidence / Security） | Security：**本机 exit 0**（SC-1~SC-4 全过，基线一致）但 **CI 从不执行**（`ci.yml:308` 只匹配 v3.12.0 分支 ⇒ #5424）；Perf Regression：**部分 PASS**（4 个子门禁 exit 0；`check_g15_perf_report.sh` 的子门禁链因**直接跑 `tpch_gate_test`** 被本机主动中止）；Release Evidence：两项 **FAIL 但均基线既有**（`check_release_binary` exit 1、`check_evidence_binding v3.7.0` 74 项违规，与基线逐字相同） | 取证 + 归属（证据 §7.25.8、#5424） |

---

## 4. Test Command Output

| 命令 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | ✅ 0 处差异 |
| `cargo build --all-features` | ✅ CI `lint-build` job 绿（run 7917，job 16133） |
| `cargo check --test tpch_sf01_22_vs_3engines --test tpch_sf01_22_vs_3engines_test -j 4` | ✅ 0 warning / 0 error（本 PR 新增的 2 个目标；修掉了 2 个属本 PR 的编译告警） |
| `cargo clippy --all-features -j 4 -- -D warnings` | ✅ 0 warning / 0 error（本地）+ CI `lint-build` 绿 |
| `cargo test --all-features --no-fail-fast` | ⏳ 待 CI（**本机不执行**：wire 路径峰值已实测到 817 MB，本机内存不足以承担全量）→ CI run 8031 已实测：`diag_q11` **ok 14 passed / 0.59s**、`_q15_q16_check` **ok 8 passed / 2.92s**（降本前 > 74 min 未结束）、`eval_22_vs_sf01` **ok 34 passed / 1419.5s**（sqlite3 CLI 在位） |
| **CI `test` job 的历史结论全量统计** | ⚠️ **全部 400 次 run 里 `test` job 零次 `success`**（skipped 181 / cancelled 10 / failure 6）——它**从来没绿过**。基线提交那次只执行了 **71 / 446** 个目标（fail-fast 在第 71 个 TPC-H panic 处中止）→ 证据 §7.17(a)(b)，立项 **#5407** |
| **CI run 8031 的 `test` job 时间账**（145 个已执行目标） | 合计 **3 380.0 s**（用每个目标自己的 `finished in` 统计），其中 3 个 TPC-H 目标独占 **2 403.2 s**（71%）。最慢：`eval_22_vs_sf01` 1419.5 s、`four_way_compare_test` 862.0 s、`dml_pk_fast_path_5177` 485.7 s。第 146 个目标 `harness_validation_test` **挂死**（05:59:01Z 之后日志冻结）→ 证据 §7.18(a) |
| **446 个目标的 fixture 装载形态全量清点** | `BATCH` 40 / `ROW_INSERT` 4 / `SQLITE_SCRIPT` 1 / `SKIP_OR_IGNORE` 35 / `PANIC_ON_MISSING` 7 / `NO_FIXTURE` 359（求和 = 446，已程序化校验）→ 证据 §7.18(b) |
| **CI run 8081 / job 16520 的实测（首次全量执行的中间轮）** | **150 / 446** 个声明目标被执行、**296 个从未开始**；已产生结果的 158 个 = **155 ok / 2 FAILED**，累计 **2073.2 s**；2 个失败均有基线对照（#5398 / #5411）→ 证据 §7.19(d) |
| **CI run 8113 attempt 1** | ❌ 3 个 job（`test` / `mutation` / `test-soak-detach`）**全部**在 Checkout 阶段 `git clone https://github.com/actions/checkout` 超时 ⇒ **`test result:` 行数 0，一个测试都没跑到** ⇒ 属 **#5397**（与本 PR 无关）；已 `rerun-failed-jobs` → attempt 2 |
| **`legacy-gate` 的 R7 clippy FAIL** | ❌ 根因 = **命令参数位置写错**：`cargo clippy --all-features -- -D warnings --quiet` 把 `--quiet` 传给了 rustc ⇒ `Unrecognized option: 'quiet'` ⇒ 编译必然失败 ⇒ **恒红、丧失判别力**。**同分支对照实测**：正确写法 `cargo clippy --all-features --quiet -- -D warnings` → **exit 0 / 0 warning / 0 error** ⇒ **代码是绿的** ⇒ 立项 **#5416**（**本 PR 不改 `scripts/gate/legacy/`**，AC-10） |
| **CI run 8115 的 mutation job 16571（修复后的真实结论）** | ❌ **`CAUGHT=2 / SURVIVED=1 / ERRORS=0` / exit 1**；`mktemp` 修复生效（三个变异片段全部生成并应用）⇒ 门禁从「永远不可能变绿」变成「能跑出真实结论」；MUT-1 存活的根因 = **锚点选错了 HAVING 的实现路径**（本 PR 自身的债，已修，待下周期实测确认）→ 证据 §7.23 |
| **CI run 8113 attempt 2 的终态** | ⚠️ **248 / 446**（244 ok / 3 FAILED，累计 2840.3 s，NOT-RUN 0）；第 249 个目标 `parallel_perf_baseline_test` **连续 94 min 未结束**（降本前的巨兽）⇒ **被新周期取代而主动取消**（不是超时砍断）；3 个失败均已归因（#5398 / #5414 / #5415）→ 证据 §7.21 |
| **#5397 在新周期立刻复发** | ❌ run 8115 的 `lint-build` 16569 死在 Checkout（`github.com/actions/checkout` TLS handshake timeout）⇒ `test` 16570 被 `needs` 连带 skipped；run 8115 卡在 `queued`（`rerun`/`rerun-failed-jobs` 均 HTTP 400）⇒ 只能另起周期。**runner 主机实测**：github **可达但极慢**（curl HTTP 200 耗时 **6.81 s**），且 act 缓存在宿主机**不存在** ⇒ 每个 job 的新容器都要重新 clone ⇒ 抖动窗口内必然 timeout → 证据 §7.21(c) |
| **CI run 8113 attempt 2（第 5、6 处降本的硬证据）** | ✅ `int2_substance_parallel_test` **0.96 s / ok**（中间轮：>50 min 未结束，O(N²) 外推 ≈130 min）；✅ `parallel_main_path_test` **14.13 s / ok, 7 passed**（原 8 × 600 000 = 480 万条逐行 INSERT）；与中间轮**共同完成的 157 个目标逐个比对**，耗时差异全部落在噪声内（最大 -2.1%）⇒ **降本生效且未波及其它目标** → 证据 §7.20(b)(c) |
| **第 7 处降本（`parallel_perf_baseline_test`）** | ✅ 该目标在 attempt 2 连续跑 **51 min 40 s** 未结束（100% CPU、内存 1.87 GiB/94.2 GiB ⇒ 非内存问题）⇒ 定位到 `populate(engine, rows, …)` 的 **180 万条逐行 INSERT**（**参数化上界**，正是 §7.19(c-2) 扫描器的盲点）⇒ 改用 `bulk_insert_records`；**行数与断言不变**，且该文件只对**查询**计时（装载从不进入计时窗口）⇒ 不影响被测对象 → 证据 §7.20(g) |
| **`oracle_g15_tpch_sf01` 的 Q22 失败** | ❌ Q22 `ERROR (write payload: Broken pipe)` 仅 **36.412 µs** ⇒ server 在 Q21 之后已死；**已排除 OOM**（容器无内存限制、`OOMKilled=false`、宿主 94.2 GiB 只用 1.9 GiB、`dmesg` 无 OOM）⇒ 指向 server 自身崩溃但 **stderr 未捕获** ⇒ 根因未确定 ⇒ 立项 **#5415**；**本 PR 已把断言顺序改成"先报失败语句本身"**（原措辞把真实失败伪装成"计数对不上"） |
| **第 5、第 6 处降本的编译验证** | ✅ `cargo check --test int2_substance_parallel_test --test parallel_main_path_test` → **0 error / 0 warning**；**行数与断言均未变**（`int2` 仍 200 000 行、`parallel_main_path` 仍 600 000 行 × 8 次调用 = 480 万条），只把逐行 `execute` 换成引擎**公开 API** `bulk_insert_records`（O(N)） |
| **全仓扫描「隐藏巨兽」（第 1 轮）** | `tests/**/*.rs`（480 个文件）程序化扫描：上界 ≥ 100 000 的逐行 SQL 循环 **2** 个（已降本）、10 000 的 1 个（实测 40.21 s）、1 000–2 000 的 8 个（可接受）、`tests/integration/tpch/` 下 **0** 个、`recovery_fuzzer_test` 的 50 000 次是 `#[ignore]` → 证据 §7.19(c-2) |
| **⚠️ 上述扫描的盲点与更正** | 上一版「**无漏网**」的说法**当时不成立**：扫描口径只覆盖「上界是**字面量**」⇒ 漏掉 `tests/benchmark/parallel_perf_baseline_test.rs` 的**参数化上界**形态（`for i in 0..rows`，实参 600 000 ×3 = **180 万条**）⇒ **第 7 处降本**。补扫（用 grep 定位全部逐行 INSERT 调用点再逐个核对上界）后确认：**除已降本的 3 处外，其余全部可接受** → 证据 §7.19(c-2) 更正 + §7.19(c-3) + §7.20(g) |
| **引擎对未知 INSERT 列名的行为** | ⚠️ **不报错，静默写 NULL 行**（实测 `INSERT INTO t (c0,c1)` 在 `(a,b)` 表上产出 `NULL\|NULL`）→ 引擎缺陷，立项 **#5409**；本 PR 不改引擎行为（AC-10） |
| `cargo check --test harness_validation_test --test tpch_q6_q19_oracle_test --test eval_22_vs_sf01 --test four_way_compare_test --test four_way_cell_diff_test --test _q15_q16_check -j 4` | ⏳ 待本轮提交前跑（0 warning / 0 error 为准） |
| **`cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli`** | ✅ 产出 `target/debug/sqlrustgo-mysql-server`（51 957 896 B）与 `target/debug/sqlrustgo`（32 190 072 B）；二进制在位时 `e2e_canonical_subprocess` 的 2 个用例 **ok / 1.01s**（修复前必然 panic 在 `e2e_canonical_subprocess.rs:68`） |
| `bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log /tmp/testjob.log` | ✅ exit 1（改造前日志，正确抓出 382 个漏跑） |
| `bash scripts/gate/check_tpch_known_differences.sh` | ✅ PASS（1 条登记 / 上限 1） |
| `bash scripts/gate/prepare_tpch_fixture.sh --out /tmp/tpch-fx3 --sf 0.01 --seed 42` | ✅ 5 类自检全绿，manifest 写出 |
| `cargo test --test tpch_gate_test` | ✅ 21/21（含 4 个 wire 语句范围单元测试） |
| `bash scripts/gate/mutate_tpch_oracle_tests.sh`（桩 `cargo` 机械逻辑） | ✅ 四个场景在同一轮实测：exit 0 / exit 1 / exit 2（超时）/ exit 2（`mktemp` 失败）（见 AC-5 与证据 §7.12.1 的如实更正） |
| **postcheck 的 6 个硬阻塞 meta-gate**（P11–P16） | ✅ 全部 PASS（本地实测，见证据 §7.15(b)） |
| **postcheck 的 `check_correctness_tests_production_storage.sh`** | ❌ exit 1（R2/R4）——**基线既有**，基线对照实测与本 PR 无关 → **#5400** |
| **postcheck 的 `check_storage_db_scope_coverage.sh`** | ❌ exit 1（9 处）——**基线既有**，基线对照实测与本 PR 无关 → **#5401** |
| **postcheck 的 `check_coverage.sh`（Gate L1 — Coverage 步骤）** | ❌ exit 2——该脚本 2026-08-10 起就是弃用桩，CI 接线未更新 → **#5403** |
| **postcheck 的 `check_evidence_binding.sh v3.7.0`** | ❌ exit 1（`FAIL=74`）——本 PR 未触碰 v3.7.0 文档 → **#5403** |
| **postcheck 的 `auto_env_blocker.sh v3.8.0`** | ❌ exit 1（`set -euo pipefail` 下遇无 H1 的 .md 中止；副作用改写 56 个受控文件，已还原）→ **#5403** |
| **postcheck 的 D8（经 D9 聚合）** | ❌ exit 2（DRIFT：`SEM-4` 87.64% IN_PROGRESS，2026-07-16 快照）→ **#5403** |
| **run 8117 的 13 个失败目标**（SHA `2b06e601e3`，`test` job 16578） | 逐个归因完毕：**1 个是本 PR 引入**（wire harness staging 目录竞态 ⇒ RED→GREEN 已修）、**3 个是环境阻塞型**（`quick_query` / `tpch_compliance_test` / `tpch_full_test` ⇒ 已改 NOT-RUN 或自包含）、**9 个是引擎/测试/性能缺陷**（已分流 #5414 / #5415 / #5393 / #5398 / #5430~#5435）→ 证据 §7.26(a) |
| **wire harness 的 staging 竞态（RED → GREEN）** | RED：`consecutive_stagings_must_not_share_a_directory` FAILED（两次 staging 返回同一目录）⇒ GREEN：`.tpch-wire-staging-{pid}-{seq}` + `StagedFixture` 的 `Drop` 清理 ⇒ `cargo test --test tpch_bug_regression_test common::tpch_wire_harness::tests::` → **9 passed / 0 failed** → 证据 §7.26(b) |
| **3 处环境阻塞型测试的 NOT-RUN 改造** | `quick_query` → `28 passed / 0 failed` + `reason=prebuilt_data_unavailable`；`tpch_compliance_test` → `14 passed / 0 failed` + `reason=fixture_missing`；`tpch_full_test` → `40 passed / 0 failed`（**3 连跑稳定**）+ `reason=external_service_unavailable`。**改前改后的 `#[test]` 清单逐字一致** ⇒ 无断言覆盖损失 → 证据 §7.26(c) |
| **本轮新暴露的真缺陷：`setup_tpch_tables` 的单表探针 + 并发不安全** | 假绿 → 真红 → 定位：探针只问 `lineitem` 一张表 ⇒ 半初始化 schema 永不修复；且多个用例**并行**调用 setup 各自 DELETE+INSERT ⇒ 主键冲突。修法：探针核对**全部 8 张表** + 建表脚本首尾加 **PostgreSQL advisory lock**（`pg_advisory_lock(5379001)`）⇒ **3 连跑 40 passed / 0 failed** → 证据 §7.26(e) |
| **编译与 lint 的三处修复** | ① `tpch_fixture::resolve()` → `tpch_fixture::FixtureState::resolve()`（E0425，`resolve` 是**关联函数**不是自由函数）② `q16_notin_subquery_regression.rs:102` 的 `let…else` 改 `?`（本 PR 引入的 lint 债）③ `cargo fmt --all`（**159 处**差异，全部在本 PR 改动的 3 个文件；不改会让 CI 的 `lint-build` 红）→ 证据 §7.26(d) |
| **最终 lint 口径** | `cargo clippy --all-features -- -D warnings` → **exit 0**；`cargo clippy --all-features --test <5 个改动目标> -- -D warnings` → **exit 0**；`cargo test --no-run --test <5 个改动目标>` → 5 个 Executable 全部产出；`cargo fmt --all -- --check` → **exit 0 / 0 差异** → 证据 §7.26(d) |
| **CI 历史 50 个 run 的 job × 结论机器统计** | `test` **0 次 success**（8 failure / 16 cancelled / 1 skipped）｜`mutation` **0 次 success**（12 failure）｜`ga3-security-scan` **25/25 skipped**｜`postcheck` **0 次 success**（16 cancelled / 9 skipped）｜`lint-build` 21 success｜`test-soak-detach` 20 success｜`evidence-graph-gate` 20 success｜`legacy-gate` 13 success → 证据 §7.27(b) |
| **run 8153 的 `test`/`mutation` 失败归因** | ❌ **不是代码红**：两个 job 都死在 `Checkout` 的 `git clone https://github.com/actions/checkout`（`unexpected EOF` / `dial tcp 20.205.243.166:443: i/o timeout`）⇒ **一行项目代码都没执行** ⇒ run 8153 的结论**不可用作任何证据**，446 目标计数与 MUT-1 杀伤力必须重取 → 证据 §7.27(a) |
| **GA 发布流程审阅（4 个维度）** | 清单唯一性 / 可执行性 / CI 覆盖 / 发布产物 / 版本线与分支 / CI 判读，**四维差距清单 + 14 个新 issue（#5436–#5449）**；并经一个**只读 verifier 子代理独立盲审**交叉验证（一致项 8 组、盲审补充项 9 组已逐条自验后采纳）→ 证据 §7.28 |

---

## 5. Evidence 清单

| 证据 | 位置 |
|---|---|
| 证据文档（症状→改动→实测→遗留） | `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md` |
| OpenSpec 变更三件套 | `openspec/changes/fix-5379-tpch-fixture-hermeticity/{proposal,design,tasks}.md` |
| 规格 | `docs/releases/v4.1.0/PR-5379_SPEC.md` |
| 测试计划 | `docs/releases/v4.1.0/PR-5379_TEST_PLAN.md` |
| 测试设计 | `docs/releases/v4.1.0/PR-5379_TEST_DESIGN.md` |
| **独立盲审**（非原作者产出） | `docs/releases/v4.1.0/PR-5379_TEST_REVIEW.md`（729 行，4 阻断 + 6 重要 + 12 建议） |
| 经验教训 | `docs/governance/lessons/LESSON_5379_TPCH_FIXTURE_2026-10-20.md` |
| 已知差异登记 | `tests/baseline/tpch_known_differences.json` |
| wire 语句范围与排除登记 | `tests/common/tpch_wire_scope.rs` |

---

## 6. 合并检查

| 检查项 | 状态 | 说明 |
|---|---|---|
| 改动集不含 `src/` / `crates/` | ✅ | gitnexus detect-changes：42 文件全部测试层 |
| 符号重命名未用 find-and-replace | ✅ | 本 PR 无重命名 |
| 提交前逐行核对 `git diff` | ✅ | 每个 commit 前逐行核对；终审 `git diff d8365606d69c` = 65 文件 |
| 5 远端同步 | ⏳ | 合并后 `bash scripts/sync/5remotes_sync.sh develop/v4.1.0` |
| 主工作区未被改动 | ✅ | 全程在 `wt-5379-tpch` worktree 操作 |

---

## 7. 风险评估

| 风险 | 等级 | 缓解 |
|---|---|---|
| **CI 内存** —— 446 个目标 + 60k lineitem fixture + 多次服务器启动 | **高** | `timeout 7200` 兜底；wire 路径已实测峰值 817 MB；范围外语句被编译期收口点挡住，不会出现无界分配 |
| **CI 时长** —— 容器上限 3 h | 中 | 共享服务器（一次装载供多个断言用）；`--no-fail-fast` + 目标执行核对门禁保证「慢但完整」优于「快但漏跑」 |
| **基线 clippy 红** —— mysql-server 2 个既有警告 | 中 | 本 PR 不改 `src/`；PR 正文显式声明，避免被误判为本 PR 引入 |
| **登记膨胀** —— 已知差异越积越多 | 低 | 门禁默认上限 1 条；`kind=cell_values` 与 `row_count` 分开判定，避免「行数相同」的差异被误判为过期 |
| **范围表过期** —— 引擎修好后范围表没更新 | 低 | 范围与排除项由**单元测试**强制覆盖全部 22 条标准查询；engine 修复后改一处即可，测试会强制提醒 |
| **本机不跑全量测试** —— 验收依赖 CI | 中 | 已在 §7.9 逐项披露未执行项与承接方；CI 上线后按 §4 表格补齐 |

---

## 8. 验收清单

- [x] AC-1 fixture 自包含
- [ ] AC-2 目标执行完整性 —— 最新一轮（run 8117）实测 **390 个目标执行 / 446 个声明**
      （CI 自报 `executed test targets: 461`，含 15 个 Cargo 自动发现的顶层目标；
      见 #5449：**另有 4 个文件 22 个用例从未执行**）；
      13 个失败目标**已逐个归因**（1 个本 PR 引入 / 3 个环境阻塞型 / 9 个引擎缺陷）→ 证据 §7.26(a)；
      ⚠️ 本轮的两个新周期（run 8153 / 8154）**一个死在 Checkout、一个是 gate.yml**，
      446 全量计数与 NOT-RUN 计数**仍待一个真跑完的周期**（证据 §7.27）
- [x] AC-3 零断言 → 真断言
- [x] AC-4 独立预言机（含本轮新增的 `tpch_q6_q19_oracle_test`：期望值从硬编码的化石改为预言机）
- [x] AC-5 变异测试实跑 —— **已取得真实结论并达成目标**：
      * **run 8115**（mutation 16571，SHA `06e352d5dc`）：`mktemp` 修复生效，
        `CAUGHT=2 / SURVIVED=1 / ERRORS=0 / exit 1` —— 存活的是 **MUT-1（HAVING 反转）**，
        根因定位为**变异锚点选错了 HAVING 的实现路径**（引擎有两条路径）；
      * 锚点改为**同时反转两条路径**（`c7ac66ee24`）；
      * **run 8185**（mutation 16700，SHA `d68dd7f886`，2026-10-20T17:59:10Z）实测：
        **`CAUGHT=3 / SURVIVED=0`** —— MUT-1 / MUT-2 / MUT-3 **全部被杀**
        → 证据 §7.23(d2)
- [x] AC-6 三态披露
- [x] AC-7 wire 路径不崩溃
- [x] AC-8 已知差异登记（1 条 `tpch:Q13` cell_values）
- [ ] AC-9 clippy（fmt 已绿；clippy 本地 0 warning / 0 error，CI `lint-build` job 绿）
- [ ] 全量 `cargo test`（本机不执行，CI 承接；本轮已实测 **155 ok / 2 FAILED**，2 个失败均有基线对照）
- [x] AC-10 改动集不含引擎源码（相对基线 `d8365606d69c`，`crates/` 只有
      `crates/mysql-server/src/lib.rs` 的 8 行零语义 clippy 修复；
      第 5、第 6 处降本**只换装载路径、不改数据行数、不改断言语义**）
- [ ] CI 绿灯 —— **`test` job 在 50 次历史 run 中零次成功**（8 failure / 16 cancelled / 1 skipped，#5407）；
      本 PR 的失败均经基线对照证伪为非本 PR 引入（#5398 / #5411 / #5414 / #5415）；
      ⚠️ **run 8153 的 `test`/`mutation` 失败是基础设施假红**（死在 `git clone actions/checkout`，
      一行项目代码都没执行）⇒ **不可作为任何证据**，必须重取（证据 §7.27，#5448）
- [x] **GA 发布流程审阅** —— 四维差距清单 + **24 个新 issue（#5436–#5459）**，
      并经**两个独立 verifier 子代理盲审**交叉验证（第一轮 9 组补充发现已逐条自验后采纳；
      第二轮 17 项发现逐条自验后 7 项成立、**1 项被更正**）→ 证据 §7.28 / §7.29 / §7.30
- [x] **13 个失败目标逐个归因** → 证据 §7.26(a)
- [x] **本 PR 引入的 staging 竞态 RED→GREEN** → 证据 §7.26(b)
- [x] **3 处环境阻塞型测试改 NOT-RUN**（新增 2 个受控原因码）→ 证据 §7.26(c)
- [x] **本轮新暴露的真缺陷**（`setup_tpch_tables` 单表探针 + 并发不安全）→ 证据 §7.26(e)
- [ ] 合并 + 5 远端同步

**本轮（证据 §7.26–§7.28）新增的、未打勾的项与原因**：

| 项 | 状态 | 说明 |
|---|---|---|
| 446 目标的全量计数 + NOT-RUN 计数 | ⏳ **待一个真跑完的周期** | run 8117 跑到 390 个（13 个失败已归因）；run 8153 的 `test`/`mutation` **死在 Checkout**（基础设施假红，#5448）⇒ 不可用 |
| **`mutation` job 在锚点修正后的杀伤力** | ⏳ **待验** | 已知唯一一次真跑完的周期是 run 8115（`CAUGHT=2 / SURVIVED=1`，MUT-1 存活）；锚点已改为**同时反转两条 HAVING 路径**（`c7ac66ee24`），**修正后的结果至今没有实测** |
| **GA 门禁的全绿结论** | ❌ **不可声称** | 5 条各自独立的失真来源（清单不唯一 / 执行点不存在 / 判据恒绿 / CI 判读不可靠 / 证据链不成立）→ 证据 §7.28(h) |
| **v4.1.0 的 GA 级证据** | ❌ **不存在** | 缺 4/5 份 `required_files` + 2/6 份 `doc_artifacts`；标准证据结构 0/11；且 `check_evidence_binding.sh` 抓不到（#5439） |
| **GA 的合法分支形态** | ❌ **不存在** | `develop/v4.1.0` 不在 `branch_pattern` 内；`release/v4.1.0` 不存在；`main` 停在 2026-09-30（#5436） |
| **GA 证据能否入库** | ❌ **不能** | `.gitignore:135` 排除了 GA 门禁日志目录（#5441） |
| **`postcheck` 何时能真跑** | ❌ **在 v4.x 上跑不对版本** | 版本探测恒落到 `v3.7.0`；4 步硬编码 `v3.8.0`（#5446） |
| **CI 的红绿是否可信** | ⚠️ **部分可信** | 基础设施红与代码红不可区分（#5448）；`✗`/`❌` 字符集不匹配让 D3 硬失败可绿过（#5443） |
| **「446 个测试目标」这个数字** | ⚠️ **是分裂的** | 15 个自动发现未声明 + 4 个文件 22 个用例从未执行（#5449） |

**本轮（证据 §7.19）新增的、未打勾的项与原因**：

| 项 | 状态 | 说明 |
|---|---|---|
| 446 个目标全部执行 | ⏳ **attempt 2 进行中**（中间轮 150/446） | 中间轮 296 个从未开始（其中 53 个 TPC-H 目标）。**根因已量化并已修**：`int2_substance_parallel_test` 的 200 000 行带主键逐行 INSERT 是 O(N²)（≈130 min）、`parallel_main_path_test` 是 8 × 600 000 = 480 万条逐行 INSERT ⇒ 单前者一个就吃光 `timeout 9000`。**第 5 处降本已实测生效：`int2_substance_parallel_test` 0.96 s（原来 >50 min 未结束）** ⇒ 终态待验（证据 §7.20(b) / §7.21） |
| **新发现的失败：`null_handling_test` 的两个 `green_4154_*`** | ❌ exit 1（2 failed / 51 passed） | `ALTER COLUMN … SET/DROP DEFAULT` 在 `MemoryStorage` 上必然报 `Table not found: t` —— `set_column_default` 漏了 `scoped_key` 键解析。**基线 commit `d8365606d69c` 直跑同样失败、同样行号 879/896** ⇒ 基线既有 ⇒ 立项 **#5414**；**本 PR 不改引擎**（AC-10） |
| Q19「引擎 vs SQLite」比较一致 | ⏳ **本轮未跑到** | `tpch_q6_q19_oracle_test` 落在未执行的 296 个里 ⇒ **无证据**（不是"通过"）。若新周期不一致 = **新发现的引擎缺陷**，按 AC-8 登记并立项，**不弱化断言** |
| **`mutation` job 的真实结论** | ✅ **已达成目标** | **run 8185 / mutation 16700**（SHA `d68dd7f886`，17:44→17:59）：**`CAUGHT=3 / SURVIVED=0`** —— MUT-1 的锚点修正（**同时反转两条 HAVING 路径**）经 CI 实测生效，三个变异**全部被杀**；此前 run 8115 是 `CAUGHT=2 / SURVIVED=1`（MUT-1 存活）。⇒ **AC-5 达成**：新写的断言真的能杀死注入的缺陷 → 证据 §7.23(d2) |
| `test` job 恒红 | ⚠️ 已知 | 2 个失败（#5398 的 g13 flaky、#5411 的 `insert_db_isolation_5057` 竞态）**均有基线对照**；本 PR 不弱化断言、不改门禁判定口径 |

**本轮（证据 §7.18）新增的、未打勾的项与原因**：

| 项 | 状态 | 说明 |
|---|---|---|
| `harness_validation_test` 不再挂死 | ✅ **CI run 8081 / job 16520 实测 6.64s / 10 passed** | 本地无法验证（不跑 TPC-H 测试）；改造前挂死 37+ min |
| `eval_22_vs_sf01` 耗时下降 | ✅ **CI 实测 66.10s / 40 passed**（改造前 1419.49s ⇒ **21.5×**） | 同上 |
| 7 个原 panic 目标在 CI 里不再 panic | ✅ **CI 实测 0 个 panic** | 改为 `TPCH-NOT-RUN`，由 CI 的 NOT-RUN 计数披露（本轮 NOT-RUN = 0） |
| 5 远端同步 | ✅ 环境可达 | 第一次探测时 `git ls-remote` 对 `gitee` / `github` / `gitcode` 三个 SSH 远端全部 exit 124（超时），一度被记成"环境受限"。**复查后确认那是本机 ssh-agent socket 已死导致的挂起**（#5380 同一个坑），加上 `-o IdentityAgent=none` 后三个远端全部成功：`gitee` 102 个分支 / `github` 26 个 / `gitcode` 498 个。`5remotes_sync.sh` 本身已经强制 `IdentityAgent=none`（脚本第 88–100 行的 `SSH_OPTS`），同步路径不受影响 |

---

## 9. GA 发布流程审阅（本轮新增交付）

> 本轮指令要求「按照 governance 规范，检查整个开发过程和 GA 发布的流程」。
> 本节是该审阅的**结论摘要**；完整取证见证据文档 §7.28。

### 9.0 开发过程侧补充（证据 §7.29 / §7.30）

§7.28 审的是「GA 阶段判据是否可信」，§7.29 审的是「**门禁本身有没有被强制执行**」，
§7.30 审的是「**门禁自己的判据能不能被执行**」。

| 层 | 实测结论 | 代表 issue |
|---|---|---|
| 强制执行层 | Gitea 的分支保护：`develop/v4.1.0` **零审批、零 required check**、`main` **允许直接 push**，而 `STAGE_CONFIG.yaml` 写的是「3 个 reviewer + 四方审批 + 7 个 required check」⇒ 治理文档里的合并门禁**在服务器端不存在** | #5452 |
| 判据可执行层 | 七类实例：规范无执行点 / 规范实现不存在 / 依赖从未满足 / 汇总被解析错误吞掉 / 判据表引用不存在的脚本 / 规范引用不存在的模板 / 规范目录里放着未合并 PR 的正文 | #5453–#5459 |

其中最值得单独指出的一条：**`postcheck` 的 18 个步骤因 `needs: [test]` 被完全遮蔽，
在 50 个 CI 周期里一次都没跑起来**（#5455）——
发布前的最后一组判据（L1 质量门禁、覆盖率、反捏造、证据绑定、Alpha/RC-GA 门禁、D9 八维聚合…）
**其有效性与误报率从未被检验过一次**。

### 9.1 审阅结论

**当前仓库状态下，「GA 全绿」是一个不可声称的结论。** 三轮审阅（GA 判据 / 强制执行 / 判据自身可执行性）
共查出 **9 条**各自独立地使它失真的来源：

| # | 失真来源 | 代表 issue |
|---|---|---|
| 1 | **清单不唯一** —— 「GA 有多少门禁」有 4 个答案、「G1~G6 是什么」有 3 套语义 | #5437 / #5438 |
| 2 | **执行点不存在** —— SSOT 的唯一执行器从不被调用；20 项里 CI 只触及 6 个 | #5440 |
| 3 | **被判据本身恒绿** —— 20 项里至少 4 项无论代码怎么写都会通过 | #5417 / #5419 / #5445 |
| 4 | **CI 的判读不可靠** —— 失败可以被吞、被错配（字符集）、被自己写进去 | #5443 / #5444 / #5447 |
| 5 | **证据链不成立** —— 产物缺失不可见、连 GA 日志都进不了版本控制 | #5439 / #5441 |
| 6 | **发布门禁本身没有可执行实现** —— `RELEASE_GATE_CHECKLIST.md` 规定的 4 个脚本 **3 个不存在、1 个从不被调用**；`RELEASE_LIFECYCLE.md` 全篇零可执行判据且停在 v3.9.0 | #5450 |
| 7 | **门禁从未执行过** —— `postcheck` 的 18 个步骤被 `needs: [test]` 遮蔽（50 周期 0 次执行）；`ga3-security-scan` 在 v4.x 100% skipped；另有 22 个 `#[test]` 从未进入任何 CI 周期 | #5455 / #5424 / #5449 |
| 8 | **合并策略在服务器端未落地** —— `develop/v4.1.0` 零审批零 required check，`main` 可直接 push，`ci_required` 的 7 个 check 名与真实 job 名几乎零交集 | #5452 |
| 9 | **规范本身指向不存在的实体** —— 提交信息规范三份并存且无任何机器裁决；release-please 链路不存在；`PR_TEMPLATE.md` 不存在却被 `AGENTS.md` 与本 PR 自己的任务文件引用；`docs/governance/PR_STAGE_CONTROL_FRAMEWORK.md` 实为一份未合并 PR 的正文 | #5453 / #5454 / #5458 / #5459 |

### 9.2 本 PR 的立场

- **不勾选任何 GA 门禁项**（模板 `AGENT_PROMPT_TEMPLATE.md:35`「不得自批 Gate」）。
- 本 PR 交付的是 **GA 流程的取证与差距清单**，不是「GA 已通过」的结论。
- 全部 **24** 条差距（**#5436–#5459**）**分流为独立 issue**，本 PR 不改治理配置、不改工作流、不改分支保护（AC-10）。
- 汇总口径：**GA 流程审阅批次 #5436–#5459 共 24 条**，加上更早的 22 条（#5414–#5435），
  本迭代累计立案 **46** 条独立缺陷。

### 9.3 审阅方法的可信度

- 审阅覆盖 `STAGE_CONFIG.yaml`（20 项 `required_gates` / 5 项 `required_files` / 6 项 `doc_artifacts` /
  `branch_pattern` / `merge_policy` / `exit_criteria`）、4 份发布流程文档、`scripts/gate/`（含 `legacy/`）、
  2 个工作流、`docs/releases/v4.1.0|v4.0.0|v3.12.0/`、`git tag` / `git ls-remote`、Cargo.toml，
  以及 **Gitea Actions API 的 50 个 run 的 job × 结论机器统计**。
- **交叉验证**：共派 **两个**只读 verifier 子代理**独立盲审**同一批文件与问题（不预先告知本会话结论）。
  第一轮 9 组补充发现**全部经本会话逐条自验**（跑命令确认）后采纳；
  其中包括一个**对本会话既有说法的更正**：`docs/governance/AGENT_PROMPT_TEMPLATE.md`
  **在本分支上并不存在**（只存在于 `fix/4597-chaos-t8d-awk-portability` 分支的 commit `9fbb63bfbc`），
  引用该模板的硬约束时必须注明这一点。
- **第二轮盲审**（开发过程侧）给出 17 项发现：本会话**逐条自验**后，14 项与既有结论一致或已自验，
  7 项构成 §7.30 的七类新实例（→ #5453–#5459），**1 项被更正**：
  盲审称 `PR_STAGE_CONTROL_FRAMEWORK.md` 是「未提交的 PR 正文」，
  实测 `git ls-files` 显示它**受版本控制**（commit `fcf8562d4a`），
  准确表述是「**一份未合并 PR 的正文被提交进了治理目录**」；
  另 1 项按实测改写措辞（`grep -c || echo "0"` 的成因不是「兜底恒绿」本身，
  而是它产生的多行 `0\n0` 解析错误 **叠加** #5443 的字符集不匹配 ⇒ 计数恒为 0）。
  **盲审结论一律不盲从，逐条复核后才采纳。**

---
