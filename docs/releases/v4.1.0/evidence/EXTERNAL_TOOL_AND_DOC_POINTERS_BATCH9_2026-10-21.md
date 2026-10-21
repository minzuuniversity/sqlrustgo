# 迭代 5 · 批次 9 —— 外部工具定位（C7）与文档指针断链（#5500 / #5499）

> **写作约定（2026-10-21 批次 10 补注）**：本文件中**引用"已知不存在的
> 路径"** 的地方一律**不加行内代码**（backtick）。原因是本仓库的行内代码
> 就是指针约定：`scripts/gate/check_docs_pointer_paths.sh` 把
> 「反引号包裹的仓库相对路径」当作指针并要求它可解析。
> 一段**明说它不可解析**的引用如果写成指针形状，门禁就会判红 ——
> 那会把"如实记录一个坏指针"变成"制造一个新坏指针"。
> 需要给出精确路径时，用普通散文写出（如
> docs/governance/AGENT_PROMPT_TEMPLATE.md 见下），并由旁边的句子说明它不存在。

> **治理框架**：AGENT_PROMPT_TEMPLATE.md v4.0.0
> —— 该文件在本分支上**不存在**（仓库路径 docs/governance/AGENT_PROMPT_TEMPLATE.md
> 只存在于 `9fbb63bfbc`，即
> `fix/4597-chaos-t8d-awk-portability` 分支，非本线祖先）。
> 详见 `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md` §盲审补充
> 第 ⑨ 条。本迭代的「不得自批 Gate」约束照办（约束来源见该文档）。
>
> **日期**：2026-10-21
> **分支**：`fix/external-tool-and-doc-pointers-batch9`
> **基线**：`develop/v4.1.0` @ `1ec554d14d7d9c3674928c15509fa414b152aef8`
> （PR #5497 的 merge commit，5 远端一致）

---

## 0. 本批次一句话

**测试树里「外部工具在哪」「用哪个用户名连库」被写死成了一台机器的事实**，
以及**文档里「完整报告在这儿」的指针指向一个不存在的文件** ——
两者都是「读起来像有据可查、实际上查不到」的同族问题。
本批次把前者变成一条**独立判据 C7**（带自证），把后者变成一条
**文档指针门禁**（带基线与自证），并修掉门禁抓出来的**每一个**真实违规。

---

## 1. 事实核实（基线树上的实测）

### 1.1 #5500 —— 机器专属的外部工具定位

基线树 `tests/integration/sql/four_way_harness.rs` 上实测到 **4 处**
非家目录形态的机器专属定位（issue #5500 已列）：

| 位置 | 内容 | 机器专属在哪 |
|---|---|---|
| `default_data_dir()` | `TPCH_DATA_DIR` 未设时回落到 `/tmp/tpch_sf01` | 只在**曾经**在那台机器上跑过 dbgen 的环境里存在 |
| `run_psql_sql()` | `Command::new("/opt/homebrew/opt/postgresql@16/bin/psql")` + `["-U", "liying", …]` | homebrew 绝对前缀 + **操作系统用户名** |
| `run_mysql_sql()` | `Command::new("/opt/homebrew/bin/mysql")` + `-u tpch -ptpch` | 同上 |
| `setup_external_db()` | 同样硬编码 `/opt/homebrew/bin/mysql`（4 处） | 同上 |

**但这不是全部。** 本批次新写的 C7 门禁第一次对全测试树扫描，
在 **issue 原文没有提到**的另外 3 个文件里，抓出了同类问题：

| 文件 | 抓出的内容 | 性质 |
|---|---|---|
| `tests/integration/tpch/tpch_sf01_22_vs_3engines.rs` | `psql … "-U", "liying"` | **真实违规**：外部 PG 的 OS 用户名被写死 |
| `tests/integration/tpch/tpch_sf01_22_vs_3engines_test.rs` | 同上 | **真实违规** |
| `tests/integration/tpch_full_test.rs` | `env::var("PGUSER").unwrap_or_else(\|_\| "liying".to_string())` | **真实违规**：硬编码的 OS 用户名兜底 |

⇒ **#5500 的实际范围比 issue 描述的大 3 个文件。**
这不是「顺手多改了几处」，而是新判据抓出来的**新事实**。

### 1.2 这些违规造成的假绿，形状是可复现的

`four_way_cell_diff_test.rs` 在基线树上的旧代码：

```rust
Err(e) => {
    eprintln!("[PG] Q{} error: {}", q, e);
    out.insert(*q, vec![]);          // ← 失败 ⇒ 空结果集
}
```

「查询失败」被记成「返回 0 行」。
`Error` 与「空结果」在下游是**同一种东西**，于是：
Q08/Q09/Q11/Q20/Q21/Q22 的 `pg_rows=0` 其实是 **PG 查询失败**，
报告里却写成「PG 返回 0 行」⇒ **6 条假差异**。

而「找不到 psql 二进制」走的是同一条路 ⇒ 在没有 PG 的机器上，
这个目标的产出是「22 条全部 0 行」的报告，**并且它是绿的**。

### 1.3 #5499 —— 文档指针断链

| 断链 | 内容 | 状态 |
|---|---|---|
| A | `BIN_LOAD_PERF.md` 的 `Full report: docs/releases/v3.12.0/perf/JSON_VS_BINT_MEASURED.md` | 目标文件**不存在** |
| B | `tpch_json_vs_bint_compare.rs` 文件头注释引用 docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md（原文如此，行内代码即指针，故此处按上文约定用散文写出） | `v3.13.0` 目录**不存在**（真实目录 `v3.12.0`）—— 已在批次 8 修好 |
| C | **批次 8 自己的证据文档** `TEST_DISCRIMINATING_POWER_BATCH8_2026-10-21.md` 第 68 行引用了同一条 `v3.13.0` 路径 | **本批次新发现** |
| D | 同上文档第 69 行含 **两个 U+FFFD 替换字符**（真实的 UTF-8 损坏，`引用了` 的「了」被写成了 `ef bf bd ef bf bd`） | **本批次新发现** |

**为什么 B/C 一直没被抓到**：`scripts/gate/check_docs_links.sh` 只检查
**markdown 链接语法** `[text](path)`。这 4 条断链全部写在**行内代码**
`` `path` `` 里 ⇒ 链接检查器**根本看不到它们**。
「一个没人维护的指针」和「一个不存在的指针」在门外看起来一模一样。

---

## 2. 判据设计

### 2.1 为什么 C7 必须是**独立**判据，不能塞进 C6

C6 的不变量是**关于路径形状**的：「不得以家目录为根」。
`/opt/homebrew/bin/mysql` 不以家目录为根 ⇒ 按 C6 的既定语义它**本来就不在范围内**。

把两种形态混进一条判据，那条判据的不变量就会变成
「测试树里没有机器专属路径」—— 那是一个**目标**，不是**不变量**
（不可陈述 ⇒ 无法证伪 ⇒ 无法判红）。

⇒ C7 自己陈述两条**可分别证伪**的不变量：

| 规则 | 不变量 |
|---|---|
| **R1** | 测试树里不得出现以机器专属**安装根**为前缀的绝对路径：`/opt/`、`/usr/local/`、`/Applications/`、`/homebrew/`、`/Library/`、`/usr/lib/postgresql/` |
| **R2** | 与**外部**数据库客户端对话的文件里，不得出现**字面量**凭据参数：`"-U"` / `"-u"` / `"--user"` / `"--password"` 紧跟字符串字面量，以及 `mysql` 的紧贴写法 `"-psecret"` |

**R2 里 `-p` 为什么不在「分离式」名单里**：在 `psql` 里 `-p` 是**端口**，
在 `cargo test -p` 里是**包选择器**。只有**紧贴**写法（`-p` 与值之间没有空格）
才是 `mysql` 的口令形态 —— 那才是凭据。

**豁免是显式的，不是推断的**：只有**自己拉起服务器**（`sqlrustgo-mysql-server`
/ `start_ephemeral` / docker）的文件才配拥有字面量凭据，
且必须在**前 20 行**写明 `c7-allow-local-credentials`。
启发式豁免会把它本该让人看见的东西藏起来。

### 2.2 文档指针门禁的不变量

**「正在交付的发布线里，每一个写在行内代码中的仓库相对路径都必须解析到真实存在的文件。」**

作用域刻意收窄到 **`docs/releases/v4.1.0/`**（正在交付的发布线），
因为仓库里另有约 **2 300** 条指向「从未提交过的文件」或
「在 3.9→4.1 重构中移动过的文件」的历史指针。那些是**过去的记录**，
改写它们会让证据描述一个从未存在过的状态。

⇒ 历史债务用**基线**记账：已知的失效指针逐条登记（带**分类**，
让人能逐条审阅），门禁对**新增**的失效指针判红。
债务变得**可见且不增长**，而不是被悄悄改写。

---

## 3. 实施

### 3.1 #5500 —— 外部工具定位

**`tests/common/bin_resolver.rs`（共享 resolver）**

新增：

* `PSQL_BIN_ENV = "SQLRUSTGO_PSQL_BIN"` / `MYSQL_BIN_ENV = "SQLRUSTGO_MYSQL_BIN"`
* `resolve_psql_binary()` / `resolve_mysql_binary()`
  —— 顺序：**显式环境变量 → `PATH`**。
  ⚠️ 与 `resolve_binary` 的差别是刻意的：外部数据库客户端**不是 cargo 的
  构建产物**，去 `CARGO_TARGET_DIR` / 包根 `target/` 里找它只会制造一条
  永远走不通的候选路径。
* `resolve_external_tool(env_var, names)` —— 通用定位逻辑
* `missing_external_tool_message(tool, env_var)` —— 错误文本必须点名**该设哪个环境变量**
* C7 判据的 Rust 侧等价物：`machine_specific_install_paths()`、
  `hardcoded_client_credentials()`、`inline_password_literals()`

**`tests/integration/sql/four_way_harness.rs`**

* 删除全部 12 处 `/opt/homebrew/...` 字面量与 `-U liying` / `-u tpch -ptpch`
* 用户名：**`psql` 完全交给 libpq**（`psql` 自己读 `PGUSER`/`PGHOST`/`PGPORT`，
  未设时回落到操作系统用户 —— 这才是每台机器上都对的默认）；
  **`mysql` 从 `SQLRUSTGO_MYSQL_USER` 读**，未设就不传 `-u`
* 口令：经**子进程环境变量 `MYSQL_PWD`** 传递，**不进命令行**
  （既不进进程列表，也不产生凭据字面量）
* 新增**类型化错误** `ExternalToolError`：
  * `Unavailable { tool, detail }` —— 客户端不存在 / 起不来 ⇒ 调用方走 **NOT-RUN**
  * `Failed { tool, detail }` —— 客户端**跑了但报错** ⇒ **真实失败**，绝不可折叠成空结果集
* `default_data_dir()` **整体删除**：数据目录改由共享的
  `tpch_fixture::FixtureState` 解析（`TPCH_DATA_DIR` → 仓库内候选目录 → **缺失**），
  `/tmp/tpch_sf01` 这个「只在一台机器上存在」的回落随之消失
* `setup_external_db()` 的「是否已装载」判据：从写死的 `"60000"`
  改为**与夹具实际的 `lineitem` 行数比对**
  （⇒ 判据从「关于一个常量的陈述」变成「关于同一份数据的陈述」）

**`tests/integration/sql/four_way_compare_test.rs`**

* `SKIP_FOUR_WAY=1` 与「夹具缺失」从**静默 return** 改为
  `tpch_fixture::report_not_run(...)` **三态披露**
* 外部引擎不可得 ⇒ 该引擎单独披露 NOT-RUN，其余引擎照常参与
* 报告改写到**构建目录**（`TPCH_FOUR_WAY_OUT` 可覆盖），
  不再覆写仓库里被跟踪的 `docs/releases/v3.9.0/perf/FOUR_WAY_TPCH_REPORT.md`
  （与批次 8 的 `four_way_cell_diff_test` 同族）

**`tests/integration/sql/four_way_cell_diff_test.rs`**

* `run_pg_queries()` / `run_mariadb_queries()` 返回 `Result`：
  **失败 ⇒ 错误**，**客户端不可得 ⇒ NOT-RUN**，**两者都不再是空结果集**
* `FOUR_WAY_TRUTH=pg` 且 PG 持同一份夹具、但某条语句失败 ⇒
  **判红**（不静默换一个真值源：那会比较到调用方没要求的东西）

**另外 3 个文件（本批次新抓出的真实违规）**

* `tpch_sf01_22_vs_3engines.rs` / `..._test.rs`：删掉 `-U liying`
  （`psql` 自己按 `PGUSER` 解析）
* `tpch_full_test.rs`：`PGUSER` 未设时不再回落到写死的 `"liying"`

### 3.2 #5499 —— 文档指针

* **新建 `docs/releases/v3.12.0/perf/JSON_VS_BINT_MEASURED.md`**
  —— `BIN_LOAD_PERF.md` §"Speedup vs JSON — revised" 那张实测表的独立成文版：
  硬件、commit、feature flag、schema、测量方法、**逐档 rows/sec**、复现命令，
  以及「本文件**不是**二次测量」与「哪些数据**没有**」的显式声明。
* 修 `TEST_DISCRIMINATING_POWER_BATCH8_2026-10-21.md` 的 `v3.13.0` 断链（C）
  与其中的 U+FFFD 损坏（D）
* **新建门禁 `scripts/gate/check_docs_pointer_paths.sh`**
  —— 作用域 `docs/releases/v4.1.0`，基线
  `tests/baseline/doc_pointer_paths_releases_v4.1.0.json`，
  `--update` 显式重建，**非 update 模式必须不改动基线**（批次 8 的 M4 教训），
  并且**校验基线的 scope 与请求的 scope 一致**（防止拿另一个 scope 的基线来比）

### 3.3 两条新门禁都接进 CI

`.gitea/workflows/ci.yml` 新增两个步骤，**先跑 `--selftest` 再跑判定**：

```yaml
- name: External-tool path check (#5500, C7)
  run: |
    bash scripts/gate/check_test_external_tool_paths.sh --selftest
    bash scripts/gate/check_test_external_tool_paths.sh
- name: Documentation pointer-path check (#5499)
  run: |
    bash scripts/gate/check_docs_pointer_paths.sh --selftest
    bash scripts/gate/check_docs_pointer_paths.sh --scope docs/releases/v4.1.0
```

---

## 4. 判别力自证（变异测试）

门禁若分不清「违规」与「干净样本」，它就是假绿。
每个变异都**注入一处真实违规**，然后要求对应门禁**变红**。

| 变异 | 注入内容 | 要求 | 结果 |
|---|---|---|---|
| **M5** | 测试文件里注入一个硬编码的 homebrew 绝对路径 | C7 判红并点名该文件 | ✅ |
| **M6** | 测试文件里注入一个字面量客户端凭据（`-U`, `someuser`） | C7 判红并点名该文件 | ✅ |
| **M7** | 文档里注入一个失效指针 | 文档指针门禁判红并点名该目标 | ✅ |
| **M8** | 把 `psql` / `mysql` 从 `PATH` 里拿掉 | 相关目标走**可见的 NOT-RUN**，而不是静默判绿 | ✅ |

M8 的形状值得单说：外部工具不可得时，目标**仍然返回 ok**，
但它**必须**打印 `TPCH-NOT-RUN: …`。
「ok + NOT-RUN」是正确形状；「ok + 没有 NOT-RUN」才是假绿。

---

## 5. 回归与门禁

### 5.1 受影响目标

| 目标 | 结果 |
|---|---|
| `four_way_compare_test` | 17 passed / 0 failed（151.5s） |
| `four_way_cell_diff_test` | 17 passed / 0 failed（73.7s） |
| `tpch_sf01_22_vs_3engines_test` | 48 passed / 0 failed（12.8s） |
| `server_thread_model_guard_test` | 4 passed / 0 failed |
| `tpch_cli_harness` | 18 passed / 0 failed |

（`tpch_sf01_22_vs_3engines_test` 里 `tpch_sf01_22_vs_postgresql_pgdate_cell`
在**删掉 `-U liying` 之后仍然通过** ⇒ 证明新路径在真机上确实连得上，
不是「删掉一个参数就让它跳过了」。）

### 5.2 门禁

见 §6「门禁总表」。

### 5.3 一次**偶发**的 lib 失败（诚实记录）

`cargo test --lib` 的**第一次**运行是 `163 passed / 1 failed`，
第二次是 `164 passed / 0 failed`（与批次 8 基线一致）。

判定「与本 PR 无关」的两条硬依据：

1. **`git diff -- src/` 为空** —— 本 PR **一个字都没动 `src/`**，
   lib 的代码与基线树逐字相同；
2. `cargo test --lib` **根本不编译 `tests/`** —— 被编译进 lib 测试二进制的
   只有 `src/` 里的 `#[cfg(test)]` 模块，本 PR 改的文件一个都不在其中。

批次 8 的证据文档已记录过同族的偶发项
（「lib 偶发 2/6 vs 基线 3/6 ⇒ 既有跨测试 env 污染」）。

⇒ 记为**既有偶发**，不记为本 PR 引入；**不**把它算作本批次的绿灯。

---

## 6. 门禁总表

| 门禁 | 结果 |
|---|---|
| `check_test_manifest_honesty` | ✅ PASS —— 472 declared targets, 0 orphan test files |
| `check_test_count_monotonic` | ✅ PASS —— active 14143 → **14149**（+6，本批新增的 resolver 判别力测试） |
| `check_anti_ignore_gate` | ✅ PASS —— 116 real `#[ignore]`（上限 150）/ 69 active（上限 80） |
| `check_drift_not_pass` | ✅ PASS —— 0 anti-patterns |
| `check_path_casefold_conflict` | ✅ PASS —— 6632 tracked paths |
| `check_off_production_path_registry` | ✅ PASS —— 10 项豁免全部有据 |
| `check_test_target_lint_budget` | ✅ PASS —— 0 warnings，基线未改动 |
| `check_no_log_tbl_json` | ✅ PASS —— 白名单与 `.gitignore` 对齐后（见 §7） |
| **`check_test_external_tool_paths`（C7，本批新增）** | ✅ PASS —— 0 处机器专属定位；`--selftest` 亦 PASS |
| **`check_docs_pointer_paths`（本批新增）** | ✅ PASS —— 27 条已登记债务，0 条新增；`--selftest` 亦 PASS |

工具链：

| 项 | 结果 |
|---|---|
| `cargo test --lib --no-fail-fast` | 164 passed / 0 failed（第一次 163/1 为既有偶发，见 §5.3） |
| `cargo clippy --all-features --all-targets -- -D warnings` | **exit 0** |
| `cargo fmt --all && cargo fmt --check --all` | 干净 |

受影响目标：见 §5.1（5 个目标全绿）。

---

## 7. 诚实性声明

* 本批次**没有**声称覆盖率已测量（本机无覆盖率工具 ⇒ 记 NOT-RUN）。
* 本批次**没有**动 `develop/v4.1.0` 的历史发布文档里的失效指针 ——
  它们是**过去的记录**，改写它们会让证据描述一个从未存在过的状态。
  27 条已登记在 `tests/baseline/doc_pointer_paths_releases_v4.1.0.json`，
  逐条带分类，可审阅、可清账。
* docs/governance/AGENT_PROMPT_TEMPLATE.md **仍不存在**（只存在于他人分支；按上文写作约定用散文写出）。
  本批次**没有**把它「修好」—— 那需要从另一条分支取文件进来，
  超出本批次范围，且属于**已记录**的既有状态。
* 本批次**顺带修好了一个既有红灯**：`check_no_log_tbl_json.sh` 的白名单
  与 `.gitignore` 不同步（`.gitignore` 已有 `!tests/baseline/*.json`），
  导致它对**已提交**的 `tests/baseline/test_count.json` 与
  `tests/baseline/test_target_lint_budget.json` 判红。
  已把白名单与 `.gitignore` 对齐 —— 一个对已提交树判红的门禁，
  是没人会跑的门禁。

---

## 8. 关联

* #5477：C6 判据（「不得以家目录为根」）—— 本批次是它的**边界**，不是它的补丁
* #5481 / #5476 / #5410 / #5465 / #5404：批次 8（判别力家族）
* #5404：批次 8 做的同源检查，本批次的 `setup_external_db` 沿用同一原则
* #5498：q4 哨兵未接线（**不在本批次范围**，见 issue 收口说明）
* 迭代 5 批次 8 的证据文档：
  `docs/releases/v4.1.0/evidence/TEST_DISCRIMINATING_POWER_BATCH8_2026-10-21.md`
