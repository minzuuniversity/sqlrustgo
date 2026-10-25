# 交接 —— 批次 46（#5525 逻辑备份/恢复）+ 批次 42–45 的成果固化

| 字段 | 值 |
|---|---|
| 日期 | 2026-10-25 |
| Agent | openclaw-mcode（MiniMax Code） |
| 仓库 | `/Volumes/workspace/dev/sqlrustgo` |
| 主工作区 HEAD（**属他人，不可动**） | `9fbb63bfbc` |
| 本 Agent 工作树 | `/Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b40` |
| 成果分支 | `fix/logical-backup-restore-5525`（独立命名，基于 `develop/v4.1.0` = `5d4817a06b`） |
| 成果提交 | 3 个 commit（见 §8）：`135b944e96` 代码 / `2c848c7d0b` 测试+文档 / 第 3 个为本文档的数字订正 |
| Gitea | `http://192.168.0.252:3000/openclaw/sqlrustgo` |
| 关联 issue | #5525（本次关闭对象）、#5516、#5377、#5531 |
| 关联 PR | #5578（批次 37–45，**本批未改动**）、#5575（批次 36）、#5585（他人） |

> 本文档面向**没有读过本次对话**的下一个 Agent。

---

## 1. 已完成事项

### 1.1 批次 42–45（已推送，合并在 PR #5578 里）

| 批次 | issue | 内容 | 提交 |
|---|---|---|---|
| 42 | #5556 / #5376 | 崩溃测试必须崩在它声称的边界上（故障点 `occurrence` + 可配 hold） | `25f23dc55a` + `0acb051719` |
| 43 | #5577 / #5389 | 测试不得覆写已归档证据（`tests/common/evidence_output.rs` + 两层门禁） | `f330c52f76` |
| 44 | #5466 | 复核：A04 已入库证据实测 80/80 逐字节一致；`matrix.json` 从未入库 | `5b66627032` |
| 45 | #5453 | 提交信息规范收敛到一份 + 机器裁决门禁 | `b52a76d8d5` |

这四批的证据文档在
`docs/releases/v4.1.0/evidence/BATCH4{2,3,4,5}_*.md`，
issue 评论 id 分别是 196919/196920、196945、196960、197080。

### 1.2 批次 46（#5525）—— 本次交接的主体

**问题**：`sqlrustgo backup` / `restore` 看起来成功、实际上什么都没做。

| 方向 | 修前 | 修后 |
|---|---|---|
| 枚举表 | 空 `HashMap` ⇒ dump 里 0 张表，却打印成功 | 从 `StorageEngine` 真实枚举；空库 ⇒ **错误** |
| DDL | 从第一行列名**猜**（无类型/约束） | **带类型**：类型 / `NOT NULL` / `PRIMARY KEY` / `UNIQUE` / `FOREIGN KEY` / 宽度 |
| 空表 | **空表彻底消失** | 空表照写 `CREATE TABLE` |
| 转义 | 一律 `'{}'` | `'`→`''`、反斜杠转义、`NULL`/数字不加引号、BLOB `X'..'` |
| 恢复 | 只数 `INSERT` 行数，**从不落库** | 真写回 + **逐表校验落库行数** |
| 开关 | 全部**静默忽略** | `schema_only` 真生效；`compress` / 非 `full` 的 `backup_type` **明确报错**；`drop_first` 真生效 |
| 数据源 | 备份**没有数据源** | CLI **必须** `--data-dir`；拿不到 ⇒ 明确失败 |

**顺带修掉的存储层缺陷**（被本路径踩出来的）：
`FileStorage::drop_table_in()` 不删 `insert_buffer` ⇒ 被 DROP 的表的缓冲行
在同名表重建后**复活**（逻辑恢复的 `--drop-first` 因此落了两倍的行）。

**产物形态**（同一份 `BackupContent` 渲染，不会漂移）：
`<id>.sql`（可移植 dump）/ `<id>.data.json`（payload，自带 SHA-256，
restore 消费的就是它）/ `<id>.meta.json`（元数据）。

**证据文档**：`docs/releases/v4.1.0/evidence/BATCH46_LOGICAL_BACKUP_5525_2026-10-25.md`

### 1.3 其它已完成的登记

* `#5377`：`CREATE INDEX` 用 `storage.get_table_info(table_name)` 不带库名 ⇒
  `USE d1` 后建索引报 `Table not found`；默认库里「成功」但 `SHOW INDEX`
  只有 PRIMARY ⇒ **索引根本没建上**。存储层缺口 = `create_index` 没有
  `_in_db` 变体。评论 **196903**。
* `#5531` 协作台账：CI 槽位互杀（仓库级并发组 `sqlrustgo-ci-pipeline-v2`
  + `cancel-in-progress: true`）。评论 **197034**。

---

## 2. 实际运行过的测试及结果

全部在 `wt-v410-b40` 上、**串行**执行（并行会互相覆盖 `target/`，见 §5.3）。

| 命令 | 结果 |
|---|---|
| `cargo test -p sqlrustgo-tools` | **505 passed / 0 failed** |
| `cargo test -p sqlrustgo-storage --features fault-injection --no-fail-fast` | **1562 passed / 1 failed** —— 唯一失败是**既有红** `index_update_crash_does_not_serve_dangling_rows`，信息逐字未变：`ERROR 1146 (42000) at line 1: Table not found: t`（根因 #5377，本批只引用不修）。两个崩溃测试二进制：`crash_commit_fsync_b1` **2 passed**、`crash_wal_index_b1` 的 `wal_append_*` **ok** |
| `cargo test -p sqlrustgo-mysql-server` | **675 passed / 0 failed** |
| `cargo test -p sqlrustgo-cli` | **87 passed / 0 failed** |
| A/B：`crates/storage/tests/drop_table_discards_buffered_rows_5525.rs` | BASE（撤掉修复）**2 red** / FIXED **3 green** |

**没有跑** `cargo test --workspace`（全量 ~30 min，且会覆写
`docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt`）。

> **统计口径**：`cargo test` 默认在第一个失败的测试目标后**停止**，后面的二进制
> 根本没跑 ⇒ 汇总数字会被低估。**必须加 `--no-fail-fast`** 才能拿到真实总数
> （storage：加之前统计到 993，加之后 1562）。

---

## 3. 未验证事项

1. **本批成果分支尚未在 CI 上跑过**（CI 槽位被仓库级并发组互杀，见 §5.1）。
2. **`cargo test --workspace` 未跑**；只跑了四个受影响 crate 的全量套件。
3. **恢复只吃本实现写出的 payload**，不解析任意外部 mysqldump 文件
   （有意取舍：SQL 字面量无法无损表达 JSON / `NaN` / `-0.0`）。
   ⇒ `mysql < dump` 与 `sqlrustgo restore` 是两条路径。
4. **`Value::Json` / `Value::Point` 在 `.sql` 里按文本渲染**（payload 里无损，
   dump 头部有显式说明）。
5. **`CHECK` 约束没有进备份**（`CheckConstraint` 持 AST `Expression`）。
   `UNIQUE` / `FOREIGN KEY` 已进。
6. **PR #5578 的 CI 尚未绿**：run 9918（in_progress）/ 9919（pending），
   之前多轮被仓库级并发组取消。

---

## 4. 当前阻塞

| # | 阻塞 | 性质 | 谁能解 |
|---|---|---|---|
| 1 | **CI 槽位互杀**：仓库级并发组 `sqlrustgo-ci-pipeline-v2` + `cancel-in-progress: true` ⇒ 多个 PR 互相取消 CI（已实测 9893/9899/9901 等多轮被取消） | 基础设施 / 协作 | 属主裁决：改按 ref 分组，或按序排队（已登记 #5531） |
| 2 | **CI 基础设施红**（#5573）：runner 每 6–9 分钟被要求优雅关机 ⇒ 容器 pause，长任务被红判 | 基础设施 | 仓库侧无法修 |
| 3 | **`CREATE INDEX` 库解析**（#5377）：执行器层 + 存储层两层都要修；修法 A/B/C 待裁决 | 待裁决 | 属主 |
| 4 | **#5584 三处语义**（显式 COMMIT 是否触发故障点 / 文件存储是否补 fsync / 半建成索引恢复） | 待裁决 | 属主 |
| 5 | **A04 的 `matrix.json` 未入库** ⇒ #5226 关闭条件结构上不成立；重新生成需要**有 PostgreSQL 参考容器**的环境（CI runner 上有 `a04-pg-ref`），本机不可用 | 环境 | 需 CI 环境 |
| 6 | **GA 三次阶段跃迁**（ALPHA→BETA→RC→GA）需人类审批；`release/v4.1.0` 尚不存在 | 流程 | 属主 |
| 7 | **168h SOAK 未完成**；TPC-H SF=1 22/22 未记录（#5387）；#4540 Q20 oracle 缺失 | 数据 | 需要跑时长 |
| 8 | **#5527**（LOAD DATA 重复键）：执行器 `execute_insert` 已有主键检查，但 LOAD DATA 走 `bulk_insert_records` 绕过；修它触及 TPC-H 导入热路径（复合主键 lineitem ⇒ 可能 O(N²)）⇒ **需先测性能** | 待测量 | 下一个 Agent |

---

## 5. 风险

### 5.1 协作风险（CI 槽位）

本批成果**放在独立分支**，不与 #5578 / #5575 / #5585 抢同一个 PR，
也不改任何人的分支。但仓库级并发组意味着：**谁最后推，谁的 CI 活下来**。
⇒ 建议：合并顺序 **#5578（9 批 / 13+ issue）> 本批 > #5575**；
推之前先看 `actions/runs` 里有没有别人的 run 在跑。

### 5.2 合并风险（已用三方合并预演实测）

本批分支基于 `develop/v4.1.0`（`5d4817a06b`），**与 PR #5578 的文件级交集
只有一个文件**：

```
git diff --name-only gitea252/develop/v4.1.0...b52a76d8d5 | sort > A
git diff --name-only b52a76d8d5 95b136a0a5              | sort > B
comm -12 A B
  → crates/storage/src/file_storage.rs
```

| 本批改的文件 | 在 PR #5578 的 65 个文件里吗 |
|---|---|
| `crates/storage/src/file_storage.rs` | **是**（#5578 在批次 42 改了同文件的 `save_index_in` 原子写） |
| `crates/tools/src/backup_restore.rs` / `lib.rs` / `Cargo.toml` | 否 |
| `crates/tools/tests/*`（6 个） | 否 |
| `crates/storage/tests/*`（新增 1 个） | 否 |
| `crates/mysql-server/src/main.rs` | **否**（#5578 没碰这个文件） |
| `crates/sqlrustgo-cli/src/lib.rs` | 否 |
| `scripts/backup.sh` | 否 |
| `docs/releases/v4.1.0/evidence/BATCH46_*.md`、`docs/knowledge-loop/handoffs/*` | 否 |

**三方合并预演（不改工作树）**：

```bash
git merge-tree --write-tree --name-only HEAD b52a76d8d5
  → 3fd267785b9f0bd601037d6ce244e86b4fb96fdc   (rc=0，无冲突)
```

合并树已核对，同时含两边关键内容：批次 42 的 `fault_injection.rs`
`occurrence`（10 处）、批次 43 的 `tests/common/evidence_output.rs`（102 行）、
本批的 `drop_table_in` 缓冲行修复（1 处）与 `collect_all_tables`（8 处）。

⇒ **合并顺序建议：先合 #5578，本批 rebase 后再合**（或反之，两边都已验证可合）。

### 5.3 环境陷阱（最容易踩）

**同一个 `target/` 目录里并行跑两套 cargo 任务会互相破坏**：
`cargo test -p sqlrustgo-mysql-server` 会**不带 `fault-injection` feature**
重建 `target/debug/sqlrustgo-mysql-server`，把崩溃测试需要的那个二进制
覆盖掉 ⇒ 崩溃测试报 `server never reached fault point ... within 60s`，
看起来像代码回归，其实不是。

**硬规则**：

```bash
# 跑崩溃测试之前，必须先这样重建
cargo build -p sqlrustgo-mysql-server --features sqlrustgo-mysql-server/fault-injection
# 且不要与其它 cargo 任务并行
cargo test -p sqlrustgo-storage --features fault-injection --test crash_wal_index_b1 -- --test-threads=1
```

其它已踩过的坑（同一批交接范围内的）：

* `git reset --hard` 会吞掉未提交的改动 ⇒ 回滚用 `mavis-trash` 或
  `git reset --soft/--mixed`。
* `cargo fmt --all` 会连带重排 `crates/storage/*` ⇒ 用 `cargo fmt -p <crate>`。
* `cargo test --workspace` 会覆写 `docs/.../a04-5226/wire/*.txt` ⇒ 跑完
  `git checkout -- docs/`（**别用 `git reset --hard`**）。
* BSD sed 在含非 ASCII 的管道里报 `RE error: illegal byte sequence`
  ⇒ 门禁脚本的 python 输出必须**纯 ASCII**，并给 `sed`/`grep` 加 `LC_ALL=C`。
* macOS bash 3.2：`source <(...)` 读不到；不支持 `grep -P`；
  空数组在 `set -u` 下用 `${arr[@]+"${arr[@]}"}`。
* `git log --format` 在 `%x00` 后会插换行 ⇒ 解析字段要 `strip()`。

---

## 6. 下一步最小三个任务

> 按「覆盖 issue 数 / CI 轮次成本」排序。**每一步都能独立闭环。**

### 任务 1 —— 合并本批 + 关单 #5525（最小、收益最直接）

1. `git fetch gitea252 && git checkout fix/logical-backup-restore-5525`
2. 若 #5578 已合并：`git rebase gitea252/develop/v4.1.0`（唯一可能冲突点
   `crates/mysql-server/src/main.rs`）
3. 推送 → 等 CI 绿 → 合并 → 在 **develop 树上复跑**
   `cargo test -p sqlrustgo-tools -p sqlrustgo-storage --features fault-injection -p sqlrustgo-mysql-server -p sqlrustgo-cli`
4. 贴 40-hex 证据评论 → 关单 **#5525**

### 任务 2 —— 合并 PR #5578 并关掉那 13 条 issue

1. 盯 `actions/runs`，确认没有别人的 run 在跑（否则会被取消）
2. CI 绿 → 合并 → develop 树复跑 → 逐条贴证据 → 关单：
   #5462 / #5463 / #5549 / #5548 / #5579 / #5491 / #5580 / #5509 /
   #5514 / #5581 / #5556 / #5376 / #5577 / #5466 / #5453
   （#5389 与 #5454 只在 `Refs` 里，不关）
3. 合并后 `bash scripts/sync/5remotes_sync.sh develop/v4.1.0` +
   `bash scripts/sync/5remotes_drift_check.sh`

### 任务 3 —— 裁决并推进 #5527 / #5377 / #5584

1. **#5527**：先**测性能**（LOAD DATA 复合主键 lineitem 的 O(N²) 风险），
   把测量数据贴进 issue，再决定改法
2. **#5377**：`CREATE INDEX` 的库解析 —— 执行器层（`execute_create_index`
   用 `get_table_info(table_name)` 不带库名）+ 存储层（`create_index`
   缺 `_in_db` 变体）**两层都要修**；修法 A/B/C 待属主裁决
3. **#5584**：三处语义待裁决（显式 COMMIT 是否触发故障点 / 文件存储是否补
   fsync / 半建成索引恢复）

---

## 7. 任务分类建议

### 交给**本地模型**（小、确定、可脚本化、不需要长上下文）

* 跑门禁脚本并把输出贴进 issue：`scripts/gate/check_*.sh`
* 复跑已有回归套件并汇总通过/失败数（注意 §5.3 的串行要求）
* 为新门禁写 `--selftest` 变异样本
* 批量补 `Refs #NNNN` trailer（用 `check_commit_message.sh --range` 报告）
* 把证据文档里的表格数字按实测刷新

### 交给 **MiniMax**（跨文件、需要推理、写代码）

* **#5527**：先测性能再改 LOAD DATA 的主键重复检查（触及热路径，必须有测量）
* **#5377**：`CREATE INDEX` 的库解析修法 A/B/C 的**实现**（执行器层 + 存储层
  两层，参考 #5266 S1 / #4951 / #5250 的既有修法）
* **#5584** 三处语义的具体实现
* 给 `sqlrustgo restore` 增加「解析外部 mysqldump 文件」的能力
  （需要一个新的、严格的解析器；不要用 `crates/tools/src/mysqldump.rs` 的
  `DumpImporter` —— 它会丢 NULL/空串语义，且按 `,` 切列定义会破
  `DECIMAL(10,2)`）

### 交给 **Codex / Claude**（长上下文、跨仓库、需要审阅）

* **GA 三次阶段跃迁**的判定材料整理（RC checklist → 人类审批包）
* **#5527 的性能测量方案设计**（怎么量 O(N²) 才算可信）
* **凭据处置**：①轮换 gitea250 scope=all + 4 枚活外部凭据、
  ③5.0.43 rev + 1698 refs/pull 重写 —— 待属主拍板，**不要自己动**
* **CI 槽位方案**：把仓库级并发组改成按 ref 分组的**完整改法**与回归验证
* 审阅 PR #5578 与本批的**合入顺序与冲突预案**

### **不要交给任何模型自行决定**（需人类审批）

* GA 三次阶段跃迁（ALPHA→BETA→RC→GA）
* `#5454` release-please 接上 vs 放弃自动化改显式版本门禁
* `#5466` 的结论更正（我只贴了证据，**未改** issue 标题/标签）
* `#5582` 回收器语义、`#5583` 的 A/B/C
* `Refs` 是否升级为强制
* `#5570` 是否允许创建 `release/v4.1.0`
* 凭据处置 ①③
* `bustubx-unpause-ci.sh` 是否放宽到 sqlrustgo job 容器

---

## 8. 分支与提交（恢复现场用）

```
仓库          /Volumes/workspace/dev/sqlrustgo            （主工作区，属他人，HEAD 9fbb63bfbc，勿动）
本 Agent 树   /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b40
BASE 对照树   /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b42base   (detach 572f2b0ad2)
成果分支      fix/logical-backup-restore-5525
```

| 分支 | 内容 |
|---|---|
| `fix/logical-backup-restore-5525` | **本批（批次 46）**，基于 `develop/v4.1.0` |
| `fix/group-by-binder-5462-5463` | PR #5578（批次 37–45），**本批未改动** |
| `fix/gate-executability-batch36` | PR #5575（批次 36） |
| `openclaw-mcode/doc-stage-drift-20261025` | PR #5585（他人） |

**恢复命令**（万一工作树被弄乱）：

```bash
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b40
git fetch gitea252
git checkout fix/logical-backup-restore-5525
git reset --hard gitea252/fix/logical-backup-restore-5525   # ← 只在确认无未提交改动时用
```

**推送方式**（凭据不落盘，token 在 `/tmp/.gitea_tok`）：

```bash
TOK=$(cat /tmp/.gitea_tok)
git -c credential.helper= -c http.extraHeader="Authorization: token $TOK" \
    push gitea252 <40-hex>:refs/heads/fix/logical-backup-restore-5525
```

**提交信息规范**（批次 45 起生效）：`<type>(<scope>): <subject>`，
type ∈ `feat fix refactor docs test chore perf ci style merge`；
`Refs #NNNN` 推荐（`Closes` 不参与机器裁决）；
git 自动生成的 `Merge ` 开头豁免。
门禁：`bash scripts/gate/check_commit_message.sh`。

---

## 9. 协作约定（必须遵守）

* **主工作区绝对不可动**（另一 Agent 在飞）。
* **一个规则只有一份实现**；门禁 fail-closed 且不改被检对象。
* **失败分三类**：代码红 / 环境阻塞（NOT RUN + 原因码）/ 基础设施红。
  NOT RUN ≠ PASS。
* **合并 ≠ 完成**：40-hex 证据 + develop 树复跑 + 证据评论后才关单。
* 多 Agent 协作：**声明先于动作**；只写 `Refs` 不写 `Closes`；台账在 #5531。
* 凭据绝不打印。
* 三次阶段跃迁（ALPHA→BETA→RC→GA）**需人类审批**，不自行审批。
