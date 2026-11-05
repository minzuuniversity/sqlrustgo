# P1-2：能力声明核查 —— 「声称的能力」到底是不是真的（2026-11-05）

## 0. 结论先行

`audit/v4.1.0/capability_claims.py` 把五个被声称的能力**用真实产物真跑一遍**：

| 探针 | 声称 | 2026-11-05 实测 |
|---|---|---|
| `cli-real-query` | 真 CLI 能跑真查询 | **VERIFIED** —— 5 行逐值一致 |
| `mysql-wire-real` | 真 MySQL 客户端走 wire 协议 | **VERIFIED** —— 5 行逐值一致 |
| `durable-on-disk` | 真落库，进程退出后还在 | **VERIFIED** —— 正常停服后重启读回一致 |
| `crash-restart` | 崩溃后重启读回 | **VERIFIED** —— SIGKILL 后重启读回一致 |
| `backup-restore` | 备份→恢复→逐值比对 | **VERIFIED** —— 往返后与备份前逐值一致 |

反向变异 **10 条对照全部按期望发生**（该判红的判红、该判绿的判绿），
其中「删掉落盘数据文件 ⇒ 持久化探针判红」正是验收口径里
「把生产存储换成 MemoryStorage ⇒ 必须红」的运行时等价形态。

**但有一条重要限定：备份探针走的是*物理*备份，不是逻辑备份。**
逻辑备份（`crates/tools/src/backup_restore.rs`）按 #5525 是骨架，
枚举 0 张表却报成功 —— 那一侧本轮**没有**能力可言，详见 §5。

## 1. 这条要解决什么

整改方案 §P1-2 的原话：

> `capability_claims.py` = **使用面冒烟**：真 CLI + 真 MySQL 客户端 + 真持久化落库
> + 崩溃后重启读回 + 备份→恢复→逐值比对（复用 #5525 三条假成功反例）

验收口径：

> 把生产存储默认换成 `MemoryStorage` ⇒ 必须红；
> 只 Mock 的测试不得支撑任何「已验证」声明。

也就是说：文档声称的能力，必须有一条**真的把它跑起来**的证据，
而不是「有个测试文件写着它测这个」。

## 2. 探针怎么做的（关键设计）

### 2.1 全是真实产物，一个 Mock 都没有

| 探针 | 真实二进制 | 真实客户端 | 真实落盘 |
|---|---|---|---|
| cli-real-query | `target/debug/sqlrustgo` | — | 是 |
| mysql-wire-real | `target/debug/sqlrustgo-mysql-server` | `/opt/homebrew/bin/mysql`（MariaDB 12.3.2 客户端） | 是 |
| durable-on-disk | 同上 | 同上 | 是 |
| crash-restart | 同上 | 同上 | 是 |
| backup-restore | 同上 + `target/debug/physical-backup` | 同上 | 是 |

「只 Mock 的测试不得支撑已验证声明」这条，在本工具里是**结构上**成立的：
工具不读任何测试输出，它自己起进程、自己发协议、自己读盘。

### 2.2 二进制解析**刻意不走 PATH 回退**（IL-10 的教训）

解析顺序：`SQLRUSTGO_BIN` → `CARGO_TARGET_DIR` → 包根 `target/{debug,release}`。
**没有 PATH 这一档。**

为什么：本轮做 IL-3 调查时踩到过 —— `tests/common/bin_resolver.rs` 的 PATH 回退
在本机取到 `/Users/liying/.cargo/bin/sqlrustgo`（**2026-02-21** 的全局安装），
它不支持 `sqlite --batch --mode csv --headers true --cmd`，
只打印 REPL 横幅就退出。证据生成器于是拿横幅去跟归档证据比，
报出「归档证据已与引擎漂移」的**假红**，并直接催生了 #5466 这条 P0。

能力探针如果也走 PATH 回退，就会拿一个干不了活的二进制去「验证」能力 ——
那正是本条要防的事。找不到产物就是 `NOT_VERIFIED`，并指名该设哪个环境变量。

### 2.3 三态不可混写

| 状态 | 含义 | 整体判红？ |
|---|---|---|
| `VERIFIED` | 真跑过，期望值逐项吻合 | 否 |
| `FAILED` | 真跑过，对不上 | **是** |
| `NOT_VERIFIED` | 跑不了（缺产物 / 缺客户端 / 环境不具备） | **是** |

`NOT_VERIFIED` **绝不等于通过**。被声称的能力只要落在这一档，整体就红。
（交接提示词 §4 的三态纪律：SKIP / INCOMPLETE / 超时一律写 INCOMPLETE，不许写成通过。）

### 2.4 逐值比对的粒度是**字段**，不是分隔符

CLI 的 `--mode csv` 是逗号分隔，mysql 客户端的 `--batch` 是制表符分隔。
比对按 `(行号, 字段)` 做，两个方向的差异（期望多 / 实得多 / 字段值不同）都算不符。

## 3. 实测输出（2026-11-05）

```
$ python3.14 audit/v4.1.0/capability_claims.py run
{"kind": "capability_claims", "probes": 5, "verified": 5, "failed": 0,
 "not_verified": 0,
 "claimed": ["cli-real-query", "mysql-wire-real", "durable-on-disk",
             "crash-restart", "backup-restore"],
 "ledger": "docs/releases/v4.1.0/CAPABILITY_LEDGER.jsonl",
 "verdict": "VERIFIED", "fails": []}
```

台账 `docs/releases/v4.1.0/CAPABILITY_LEDGER.jsonl` 逐条记录每个能力的
状态、细节与证据（命令、读回的行、使用的客户端与端口）。

### 3.1 逐值比对用的数据（5 行，含非 ASCII）

```
id | s
1  | alpha
2  | beta
3  | gamma-三
4  | delta
5  | epsilon
```

`gamma-三` 是刻意放的：它能抓住「按字节截断」「按 UTF-8 边界切错」这类问题 ——
纯 ASCII 的数据比不出来。

## 4. 反向变异：证明判据可被证伪

```
$ python3.14 audit/v4.1.0/capability_claims.py selftest
{"kind": "capability_selftest", "checks": 10, "ok": 10, "not_ok": 0,
 "verdict": "VERIFIED", "detail": []}
```

| # | 对照 | 期望 | 实测 |
|---|---|---|---|
| 1 | 把期望值改成错的 ⇒ `cli-real-query` | 判红 | FAILED 是 |
| 2 | 把期望值改回对的 ⇒ `cli-real-query` | 判绿 | VERIFIED 是 |
| 3 | 查一张不存在的表 ⇒ 客户端 | 非零退出 | rc≠0 是 |
| 4 | 两次运行之间**删掉落盘数据文件** ⇒ `durable-on-disk` | 判红 | FAILED 是 |
| 5 | 同上 ⇒ `crash-restart` | 判红 | FAILED 是 |
| 6 | 不删数据 ⇒ `durable-on-disk` | 判绿 | VERIFIED 是 |
| 7 | 不删数据 ⇒ `crash-restart` | 判绿 | VERIFIED 是 |
| 8 | 把期望值改成错的 ⇒ `backup-restore` | 判红 | FAILED 是 |
| 9 | 正向往返 ⇒ `backup-restore` | 判绿 | VERIFIED 是 |
| 10 | **改掉恢复目录里的一个值**再读回 ⇒ 逐值比对 | 检出差异 | 检出 是 |

第 4、5 条就是验收口径的运行时等价形态：
「把生产存储换成 MemoryStorage」的运行时表现正是**数据不落盘**，
删掉落盘文件与之等价 —— 探针必须判红，实测判红了。
第 6、7 条保证第 4、5 条不是因为探针恒红。
第 10 条保证「逐值比对」不是空话：篡改一个值后比对确实检出差异。

## 5. 重要限定：备份走的是**物理**备份，逻辑备份仍是骨架

`backup-restore` 探针用 `physical-backup`（`crates/tools/src/physical_backup.rs`，
bin 名 `physical-backup`，支持 `backup`/`list`/`verify`/`restore`/`prune`），
实测往返成功：

```
physical-backup backup   -d <bkdir> -D <datadir> -w <datadir/wal>   ⇒ rc=0
physical-backup restore -d <bkdir> -t <target>  -w <target/wal>    ⇒ rc=0
在 <target>/data 起服读回 ⇒ 5 行，与备份前逐值一致
```

**逻辑备份（`crates/tools/src/backup_restore.rs`）本轮没有测，也不该测** ——
按 #5525（priority/p0，仍 open），它的 `run_backup()` 拿到的表清单是空 `HashMap`，
写出的 `.sql` 里一张表都没有，却照样打印「Backup completed successfully!」；
`restore()` 只数 INSERT 行数，数据一行都不落回。
拿它当能力证据，就是 #5525 描述的那种假绿。

⇒ **能力台账里「备份恢复」这一条，指的是物理备份。**
逻辑备份的能力当前是 **0**，且这是已知 P0，不在本条整改范围内。

## 6. 口径变化（如实记录）

| 指标 | 改动前 | 改动后 | 说明 |
|---|---|---|---|
| `check_n` | 168 | 168 | 分母未动（本工具不进 STAGE_CONFIG，文件名不叫 `check_*`） |
| `docs410` | 82 | **84** | 本报告 + EX-6 报告各占 1 份；两份自身声明记号均为 0 |
| `claims_hits` | 1144 | 1144 | 未动 |

台账 `docs/releases/v4.1.0/CAPABILITY_LEDGER.jsonl` 落 `docs/` 而不是 `audit/v4.1.0/`：
后者的子目录会被 `gov_probe` 计进 `audit_runs` 指标，
run 目录命名正则还会要求 manifest.yaml。

## 7. 残余风险与未闭环部分

- **逻辑备份/恢复（#5525）未验证且已知是骨架**。本条不能被读成「备份恢复已验证」——
  验证的是物理备份。
- **探针覆盖的是「能力存在」，不是「能力正确」**。
  5 行数据比对能证明「真能落库、真能读回、真能往返」，
  不能证明语义完全正确 —— 语义正确性由各功能的测试负责。
- **备份探针读回的是恢复目录里由同一套存储代码读出的数据**。
  如果存储的序列化/反序列化两侧有同一个 bug，往返比对看不出来。
  这一层要靠「恢复目录用**独立**实现读出」才能覆盖，本轮没做到。
- **本条不构成门禁通过声明**：本仓未跑 `ci_gate.sh`。
- 归档 run 侧的第二血统复核仍被 IL-9（CI 执行面，runner-03 缺镜像）挡住。
- 验收口径里的**「把生产存储默认换成 MemoryStorage ⇒ 必须红」**，
  本轮是以运行时等价形态（删落盘文件）验证的，
  **没有**真的改生产存储装配链再重建验证。
  静态侧的那条门禁 `scripts/gate/check_correctness_tests_production_storage.sh`
  已存在且自带 `--selftest`（其头部注释自证该判据可用），
  但「扩到所有声称验证存储语义的测试」这半条本轮**未做**（见 §8）。

## 8. 未做的半条：生产存储门禁的扩面

整改方案 §P1-2 还有后半句：
「把 `check_correctness_tests_production_storage.sh` 的『按分支判定装配链』
扩到**所有**声称验证存储语义的测试」。

本轮**未做**这一半。原因是它要枚举「哪些测试声称验证存储语义」，
而仓里目前**没有**这份声明的权威清单 ——
这正是 P1-4「每道门禁写 effect」要补的东西（effect 声明 + honesty 核对）。
两份工作有依赖关系：先有 effect 声明，才谈得上「所有声称验证存储语义的测试」。

已登记为看板新行 **IL-11**（未开始），并注明它被 P1-4 挡。
