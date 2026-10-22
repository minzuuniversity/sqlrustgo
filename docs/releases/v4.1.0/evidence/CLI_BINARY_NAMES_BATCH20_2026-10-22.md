# 迭代 5 · 批次 20 —— `scripts/` 侧 6 处把**包名**当**二进制名**：目标二进制根本不存在（#5516）

- **Issue**: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5516
- **Source Issue**: #5516
- **Agent**: MCode（MiniMax Code，单 Agent 主控 Loop；子代理只做只读盲审）
- **分支**: `fix/cli-binary-names-5516-batch20` → `develop/v4.1.0`
- **基线**: `775dc5da72`（develop/v4.1.0）
- **执行**: 本机 worktree `/Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-batch20`，未触碰主工作区
- **关联新 issue**: #5525（逻辑备份/恢复是骨架）、#5526（runner 镜像不固定 clippy 版本）

---

## 0. 一句话

`scripts/` 下 6 处脚本指向的二进制**要么从不存在，要么以错误的参数面被调用**；
其中 `run_sf10.sh` 是「找不到 ⇒ 记 stub SKIPPED ⇒ exit 0」的**恒假绿**。
本批把二进制定位收敛到**仓库唯一的实现**，把 6 处断口全部指向真实二进制，
并把沿路挖出来的 4 个真缺陷（转发位置参数、抓到别的工作区的旧二进制、
恒假绿、单查询模式不可达）一并修掉。

---

## 1. 现象：6 处断口，逐处核对

| 脚本 | 原引用 | 实际情况 |
|---|---|---|
| `scripts/backup.sh` | `./target/debug/sqlrustgo-tools backup …` | `sqlrustgo-tools` 是**包名**；该包的 bin 是 `tbl2bin`/`bench_client`/`physical-backup`/`repro_4564`/`ha`/`log-rotation` —— **没有** `backup` 子命令 |
| `scripts/restore.sh` | `./target/debug/sqlrustgo-tools restore …` | 同上 |
| `scripts/gate/check_bustubx_edu_cli_smoke_v312.sh` | `$REPO_ROOT/target/debug/sqlrustgo-cli` | `sqlrustgo-cli` 是**包名**；其 bin 叫 `sqlrustgo` |
| `scripts/stability/soak_1h.sh` | `CLI_BIN=${CLI_BIN:-./target/release/sqlrustgo-cli}` | 同上 |
| `scripts/stability/run_24h_soak_v2.sh` | `./target/release/sqlrustgo-cli soak --duration --rate --query-file` | 同上，且**参数面也不对**（见 §2.2） |
| `scripts/tpch/run_sf10.sh` | `$REPO_ROOT/target/release/tpch_runner` | **`tpch_runner` 在整个 git 历史里从未存在过**（无 `.rs`、无 Cargo target；引入它的提交 `7248a11d93 "GA-P1 #3607"` 一开始就引用了它） |

### 1.1 `run_sf10.sh` 是恒假绿

```bash
if [ -x "$REPO_ROOT/target/release/tpch_runner" ]; then … fi
if [ -z "$runner" ]; then
    log_warn "tpch_runner not found. Recording stub result."
    echo "SKIPPED: tpch_runner not built" > "$log_file"
    return 0          # ← 永远走这里，且永远成功
fi
```

二进制永远找不到 ⇒ 每次写一份 `SKIPPED` 的 stub 报告并 **exit 0**。
读日志的人看到的是「SF=10 的查询跑过了」，实际上**一次都没跑**。

**真正的 runner** 是 cargo example `crates/bench/examples/tpch_run_query.rs`
（`--query <name>` 读 `queries/<name>.sql`、`--data-dir`、`--timeout`，
输出一个 JSON 对象）。

### 1.2 runner 的退出码会骗人

`tpch_run_query` **查询失败时也 exit 0** —— 错误在 JSON 的 `error` 字段里：

```rust
Err(e) => RunResult { …, error: Some(e.to_string()), timed_out }
```

只看退出码 ⇒ **22 个失败的查询会变成「22/22 PASSED」**。
判定必须读 JSON 的 `"error"` 字段。

---

## 2. 接口决策与逐处修法

### 2.1 `backup.sh` / `restore.sh`：规范入口是 `sqlrustgo backup/restore`

issue 原文说这两个脚本「需要一个**接口决策**，不是改名」。本批的决策与依据：

| 候选 | 结论 |
|---|---|
| `sqlrustgo backup` / `sqlrustgo restore`（走 `crates/tools/src/backup_restore.rs`） | **选它** —— 规范 CLI 的入口面，与脚本的 `--database` 语义一致 |
| server 的 `backup` 子命令 | 是上面那条路径的**下游**（CLI 会 spawn 它），不是独立入口 |
| `physical-backup` | **物理**备份（`--dir`/`--data-dir`/`--wal-dir`），与脚本的「按数据库逻辑备份」语义不是一回事，不选 |

选定的命令形态（实测可用）：

```bash
sqlrustgo backup  <output_dir> --database <name> --backup-type <type>
sqlrustgo restore <backup_id>  --database <name> --backup-dir <dir>
```

**顺带修掉的转发缺陷**：`SubCmd::Backup` 原来转发 `--output <dir>`，
而服务端的 `Backup { output: String }` 是**位置参数** ⇒ clap 直接报错，
`sqlrustgo backup <dir>` 从来没通过过；`Restore` 同理（`--input`/`--database`
vs 位置参数 + 服务端硬编码 `database="default"`/`backup_dir="."`）。

⇒ 服务端 `Backup` 增 `--database`/`--backup-type`，`Restore` 增
`--database`/`--backup-dir`，并透传；CLI 侧改为**位置参数 + 正确选项名**转发。

### 2.2 `soak_1h.sh` → `sqlrustgo`；`run_24h_soak_v2.sh` → `sqlrustgo-soak`

仓库里有两个名字相近、职责不同的二进制：

| bin | 产出包 | `soak` 子命令是什么 |
|---|---|---|
| `sqlrustgo` | `sqlrustgo-cli`（`crates/sqlrustgo-cli`） | **REPL**：读 stdin，输出 `OK\t`/`ROWS\t`/`COL\t`/`DATA\t`/`ROW\t`/`ERR\t` |
| `sqlrustgo-soak` | `sqlrustgo-soak`（`crates/cli`） | **压测跑子**：`--duration`/`--rate`/`--report-interval`/`--query-file` |

* `run_24h_soak_v2.sh` 传的是 `--host --port --user --password --duration
  --rate --report-interval --query-file` ⇒ **跑子**的参数面 ⇒ 指向 `sqlrustgo-soak`。
* `soak_1h.sh` 用的是 `serve --port --data-dir`、`cli "<SQL>" --port`、
  `soak --host --port`（管道喂 stdin 并检查 `^ROW`）⇒ **规范 CLI** 的参数面 ⇒ 指向 `sqlrustgo`。

`soak_1h.sh` 里还埋着另一个真缺陷：它调的是
`sqlrustgo exec "<SQL>" --port <port>`，而 `SubCmd::Exec` **没有** `--port`
且是本地模式 ⇒ clap exit 2 + `set -e` ⇒ 脚本在第一条 DDL 就死，
而 `2>/dev/null` 又把错误吞掉 ⇒ **压测从未真正跑过**。
⇒ 改成走服务端路径的 `cli "<SQL>" --port <port>`，并且**不再吞错误**。

---

## 3. 一个规则只能有一份实现：二进制定位收敛到一处

仓库里 shell 侧定位 cargo 产物的实现原本是 `tests/common/resolve_bin.sh` 的
`resolve_sqlrustgo_bin()`，但它**只认 `sqlrustgo`**，而且 scripts/ 下另有 6 处
各写各的候选路径。

本批：

* `tests/common/resolve_bin.sh` 新增 **`resolve_cargo_bin <name> [env_var] [label]`**
  ——**唯一**的查找实现，顺序为：
  显式环境变量 → `$CARGO_TARGET_DIR/{debug,release}/<name>` →
  `…/examples/<name>`（cargo 的 example/bench 产物）→ `<repo>/target/…` → `$PATH`；
  找不到 ⇒ 打印 `TEST-NOT-RUN:` 披露并返回 **77**（automake 的 "skipped"），
  **绝不**返回 0。
* `resolve_sqlrustgo_bin()` 改为调用它，候选名**只保留 `sqlrustgo`** ——
  `sqlrustgo-cli` 是包名，把包名列为候选名等于让「这个二进制叫什么」有第二个来源。
* 新增 `scripts/common/resolve_bin.sh` 作为 scripts/ 侧的入口，
  **只 source、不复制逻辑**。
* 权威的「包名 → 二进制名」映射仍由 `scripts/gate/check_cli_binary_names.py` 持有。

### 3.1 顺带修掉的：`mysql_server_bin()` 会抓到别的工作区的旧二进制

改 `crates/sqlrustgo-cli/src/lib.rs` 时实跑撞到：

```
$ sqlrustgo backup /tmp/b20_bk2 --database default --backup-type full
error: unexpected argument '--database' found
Usage: sqlrustgo-mysql-server backup <OUTPUT>
```

原因：CLI 用 `mysql_server_bin()` 定位服务端二进制，而它的候选表是
**裸名字（走 PATH）+ `./target/debug/…` + `./target/release/…`**（相对**当前工作目录**），
**完全不认 `CARGO_TARGET_DIR`**。于是在别的工作区里跑时，它抓到了
**另一个工作区 `./target/debug` 下的旧服务端二进制** —— 一个 CLI 面已经变了的旧产物。

⇒ `mysql_server_bin()` 改为：`SQLRUSTGO_SERVER_BIN`（仓库既有约定，
`crates/storage/tests/fault_coordination.rs` 与 `scripts/tpch/*`、`scripts/sysbench/*` 都在用）
→ `$CARGO_TARGET_DIR/{debug,release}/` → **当前二进制所在目录**（最可靠：
它就是 CLI 自己的构建）→ `./target/{debug,release}/` → `$PATH`。
找不到时保留裸名字，让 spawn **大声失败**。

---

## 4. 验证（实测输出，不是推断）

### 4.1 `scripts/backup.sh` —— 两个二进制都定位对了，转发生效

```console
$ bash scripts/backup.sh -o /tmp/b20_bk2 -d default
Binary:      /Volumes/…/wt-v410-b20-target/debug/sqlrustgo
Server Bin:  /Volumes/…/wt-v410-b20-target/debug/sqlrustgo-mysql-server
Database:    default
Output Dir:  /tmp/b20_bk2
Starting backup...
Starting backup for database: default
Backup completed successfully!
Backup ID: backup_1792636196
Size: 62 bytes
ERROR: the backup artifact /tmp/b20_bk2/backup_1792636196.sql contains no table.
       The backup implementation enumerates zero tables today, so the
       run 'succeeded' without backing anything up. This is reported as
       a failure rather than as a successful backup.
SCRIPT_EXIT=1
```

⇒ 二进制定位 ✓、参数面 ✓、**产物被校验** ✓。
最后那行红灯是**真缺陷**，不是脚本坏了：逻辑备份的实现本身是骨架
（`run_backup` 拿到的表清单是空的 `HashMap`；`restore` 只数 `INSERT` 行数），
已开 **#5525**。脚本的选择是：**把空备份报成失败**，而不是替实现方圆场。

### 4.2 `scripts/tpch/run_sf10.sh` —— 查询真的跑了

单查询模式（`Q#` 模式原先**根本进不去**，见 §5 的死代码项）：

```console
$ bash scripts/tpch/run_sf10.sh /Volumes/…/tpch-fixture 900 q1
[INFO] Data directory: /Volumes/…/tpch-fixture
[INFO] Runner: /Volumes/…/wt-v410-b20-target/debug/examples/tpch_run_query
[Q: Starting...] : Starting...
[ERROR] Q: TIMEOUT after 900s

Duration: 900s
Exit code: 124
TIMEOUT
SCRIPT_EXIT=1
```

⇒ runner 由解析器定位 ✓；查询**真的执行了 900 秒**（不是 stub）✓；
超时由**退出码 124** 判定 ✓；判定写进日志 ✓；**脚本退出码=1** ✓。

（对照：改动前这个脚本每次都写 `SKIPPED: tpch_runner not built` 并 exit 0。）

22 条查询的整轮实跑（同一份 fixture，逐条 60s 上限）：

```
Q1   TIMEOUT    60s
Q2   TIMEOUT    60s
…
Q22  TIMEOUT    61s
```

⇒ 22 个查询**全部真实执行**（旧的 `tpch_runner` 路径一次都跑不了），
且**失败被如实计入**。

> 这一轮跑的是「整轮扫描」路径，用来说明「查询真的跑了」；
> 判定行顺序、退出码传播、`timed_out` 判定这三项是**改完之后**用单查询模式
> （上面的 900s 那次）验证的 —— 整轮那次跑在改动中途，脚本文件被边跑边改
> 打断过（bash 按字节偏移读脚本），末尾的收尾输出不可信，
> 所以只引用它的「22 条查询都执行了」这一事实。

### 4.3 `scripts/backup.sh` 的选项拒绝是显式的

`--compress` / `--schema-only` / `--drop-first` / 非 `full` 的 `--type`
在实现里都是 `#[allow(dead_code)]`（接收后**静默忽略**）。
脚本**显式拒绝**并给出原因（exit 2），而不是假装支持。

### 4.4 lint：本批改动的两个文件贡献 **0** 条 lint

`cargo +1.95.0 clippy -p sqlrustgo-cli -p sqlrustgo-mysql-server --all-features
--all-targets -- -D warnings` 在这两个 crate 上仍然是红的，但**全部 8 处命中都在
`crates/sqlrustgo-cli/src/sqlite_mode.rs`** —— 一个本批**没有碰过**的文件
（`unused_mut` / `unused_variables` / `if_same_then_else` /
`field_reassign_with_default`，属于 workspace 既有 lint 债，由
`check_test_target_lint_budget.sh --scope workspace` 记账）。
本批改动的 `crates/sqlrustgo-cli/src/lib.rs` 与
`crates/mysql-server/src/main.rs` **一条 lint 都没有**。

---

## 5. 负对照：门禁/脚本必须还能分辨

| 负对照 | 命令 | 期望 | 实测 |
|---|---|---|---|
| runner 缺失 | `CARGO_TARGET_DIR=/tmp/empty-target-dir bash scripts/tpch/run_sf10.sh <fixture> 60 q1` | 硬失败，**不是** stub SKIPPED | `TEST-NOT-RUN: run_sf10 reason=binary_not_found …` + exit **1** ✓ |
| 未实现的选项 | `bash scripts/backup.sh --compress` | 显式拒绝 | exit 2 + 说明「会被接收后静默忽略」✓ |
| 空备份产物 | 见 §4.1 | 报失败 | exit 1 + 说明「枚举 0 张表」✓ |
| 旧二进制被抓 | `sqlrustgo backup … --database default`（在别的工作区） | 报「未知参数」 | 修复后转发到**本工作区**的服务端二进制 ✓ |

### 5.1 顺带修掉的死代码：`Q#` 模式原先进不去

`run_sf10.sh` 的顶层用 `$1`/`$2` 取 `DATA_DIR`/`TIMEOUT`，而 `main` **不带参数**被调用
（`main`），于是 `main` 里的 `case "${1:-all}"` 里的 `$1` **恒为空** ⇒ 永远走 `all` 分支，
文档里写的 `Q#` 单查询模式是**不可达的死代码**。
⇒ 模式改为独立的 `$3`（`MODE="${3:-all}"`），并同步更新 usage。

### 5.2 顺带修掉的第二个假绿：脚本退出码不反映判定

`main` 的 `all` 分支原来是 `run_all; show_summary` —— 分支的退出码取**最后一条命令**
（`show_summary`，即 0），于是**即使有查询失败，脚本也 exit 0**。
⇒ `run_all || overall=$?` 捕获判定，`show_summary` 照常打印，
最后 `return "$overall"`；单查询分支同理。

### 5.3 顺带修掉的第三个假绿：`timed_out` 没被判定

`tpch_run_query` 的 JSON 里还有 `"timed_out"` 字段：查询跑完了但**超过了自己
的时限**时，它是 `true` 而 `error` 仍是 `null`。只看 `error` 会把这种查询算成通过。
⇒ PASS 的判定改为**同时**要求 `"error": null` **且** `"timed_out": false`。

### 5.4 顺带修掉的第四个：`run_query` 的判定行被覆盖

日志原本先写 `$status` 再写 `Exit code: N`，而 `run_all` / `show_summary` 用
`tail -1` 读判定 ⇒ 失败时读到的是 `Exit code: 1` 而不是 `FAIL`/`TIMEOUT`。
⇒ **判定行改为最后写**。

---

## 6. 本批跑测试时暴露的两件事（都不是本批引入的）

本批第一次**真的运行** `crates/sqlrustgo-cli` 与 `crates/mysql-server`
的测试（`cargo test -p sqlrustgo-cli -p sqlrustgo-mysql-server --no-fail-fast`），
暴露了两个**既有**问题 —— 它们之所以是「既有」，正是因为 CI 从来不跑
`crates/*` 的测试目标（**#5515**）。

### 6.1 `cli_test` 10 个失败 —— 测试树里的同一个病

`crates/sqlrustgo-cli/tests/cli_test.rs` 的 `cli_bin()`：

```rust
let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
p.push(".."); p.push(".."); p.push("target"); p.push("debug");
p.push("sqlrustgo-cli");        // ← 包名，不是二进制名；且硬编码 target/debug
```

⇒ 拼出来的路径**永远不存在**，10 个测试全部 panic 在
`binary should run: Os { code: 2, kind: NotFound }`。

这是 #5516 的**测试树孪生体**。**批次 15（#5405）已经修好了它**
（改用 `env!("CARGO_BIN_EXE_sqlrustgo")` —— 仓库既有的写法，
`crates/mysql-server/tests/` 下多个文件都是这么写的），
本分支基于 develop、还没合入批次 15，所以这里仍然是红的。
⇒ **不在本批重复修**，等批次 15 合并即可。

### 6.2 `client_local_infile_test` 1 个失败 —— 断言的前提是错的

```rust
assert_eq!(count(&mut conn, "t"), 4,
    "re-loading the same ids overwrites (first column is the row key), …")
```

实测 `left: 6 / right: 4` ⇒ **重复导入是追加，不是覆盖**。

核对下来，这条断言的前提**不成立**：本引擎的 DML **没有**「首列即行键」的规则
（首列只是 MVCC 记账用的 tombstone key，见
`crates/storage/src/engine.rs` 的
「collect primary keys (column 0) of deleted rows」）；
普通 `INSERT` 走的也是追加语义。而 MySQL 在**没有声明键**的表上
同样是追加 —— 表 `CREATE TABLE t (id INT, name TEXT)` 没有主键，
所以 6 行才是**与 MySQL 一致**的结果。

⇒ 断言改成真实语义（6 行，并写明为什么），测试真正的目的
（**连接在 LOCAL INFILE 往返之后仍然可用**，协议序号不错位）保持不变。
该测试目标现在 **7/7 绿**。

**但由此暴露了一个真分叉**：`LOAD DATA` **完全不检查重复键** ——
表**声明了** `PRIMARY KEY` 时，MySQL 会报
`ERROR 1062 Duplicate entry '1' for key 'PRIMARY'`，本引擎仍然静默追加。
已开 **#5527**。

---

## 7. 遗留与登记

* **#5525**（新开）：逻辑备份/恢复是骨架 —— `backup` 枚举 0 张表、
  `restore` 只数 `INSERT` 行数、`schema_only`/`compress`/`drop_first`
  被静默忽略、备份格式从第一行猜 DDL 且**空表会从备份里消失**。
  这是本批**揭开**的一层，需要单独一批实现。
* **`scripts/` 纳入 `check_cli_binary_names.py` 的 C3 扫描范围 —— 本批已做**。
  该门禁脚本由批次 15 引入（develop 尚无），所以本批先把批次 15 合了进来
  （冲突只有 `tests/common/resolve_bin.sh` 一处，解法取两边并集），
  再在**同一份**实现上扩范围 —— 不复制第二份门禁。
  扩范围后实测：`scripts/` 侧**零断口**，门禁整体 EXIT=0。
  原本考虑把扩展放在批次 15，但那样批次 15 单独合并即 CI 红（C3 抓到
  `scripts/` 尚未修的 9 处断口）⇒ 两个 PR 就都不能独立合并。
  放在本批（修断口的这一批）则合并后立即全绿，**两个 PR 各自独立可合并**。
* `crates/tools/src/backup_restore.rs` 里 `BackupCommand`/`RestoreCommand`
  仍然没有任何 binary 暴露它们（只有 `sqlrustgo` 这条路径经服务端转发），
  属于 #5525 的范围。
