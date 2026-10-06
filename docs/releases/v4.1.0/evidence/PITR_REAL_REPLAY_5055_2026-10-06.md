# #5055 — PITR 真实回放（证据）

- **Issue**: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5055
- **PR**: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/5061
- **分支**: `fix/5055-pitr-writes-data` → `develop/v4.1.0`
- **commit**: `654488601f`（实现）、`3ad90cdc61`（可测退出码 + 变异）
- **基线**: `fc9e675dfc74`（5 远端一致）
- **执行**: 本机 worktree `~/workspace/dev/sqlrustgo-worktrees/wt-5055`，未触碰主工作区

## 1. 缺陷：CLI 报假成功

```rust
// crates/admin/src/main.rs:157-176（修复前）
Commands::Pitr { data_dir, wal, target_time } => {
    let wal_path = std::path::PathBuf::from(&wal);
    let _ = data_dir;                                  // data_dir 被丢弃
    let target_ts = pitr::parse_target_time(&target_time)?;
    let r = pitr::pitr_replay(&wal_path, target_ts)?;   // 只计数
    println!("pitr ok: scanned={} applied={} ...", ...);
    Ok(0)
}
```

`pitr_replay_entries`（`crates/admin/src/pitr.rs:31`，修复前）只做三件事：按时间戳过滤、统计 committed/aborted、把 `applied += 1`。不打开数据目录，不写任何一行。

## 2. 根因：FileStorage 根本没有 WAL

```rust
// crates/storage/src/file_storage.rs:264（修复前）
pub fn new_with_wal(data_dir: PathBuf) -> std::io::Result<Self> {
    let _wal_path = data_dir.join("sqlrustgo.wal");   // 算完就丢
    let storage = Self { /* 无 wal 字段 */ };
```

结构体无 WAL 字段、无 append 路径、未覆写 `is_wal_enabled()`（继承 trait 默认 `false`）。

## 3. 追查中发现的四个独立缺陷

| # | 缺陷 | 证据 |
|---|---|---|
| 3.1 | 编解码器漂移：编码端 8 变体 / 解码端 6 变体 | `WalStorage::record_to_bytes` 有 `P:`/`J:`；`RecoveryEngine::bytes_to_record` 走 #4682 容忍分支，`Value::Null` + 只前进 2 字节 |
| 3.2 | V1 格式含 NUL 的 Text/Blob 无法往返 | `\0` 终止符：`decode("a\0b")` → `"a"`，剩余字节再扫成 Null |
| 3.3 | LSN 从不落盘 | `WalWriter::append` 递增 `self.lsn` 但序列化调用方的 entry；磁盘每条 `lsn == 0` |
| 3.4 | 事务 id 复用 | `next_tx_id` = `now_nanos % 1_000_000 + undo 长度`；100 次连续 BEGIN/COMMIT → 98 个不同 id |

3.4 的实测（本次修复前的探针输出）：

```text
PROBE 100 back-to-back BEGIN/COMMIT pairs
PROBE distinct tx ids = 98   (expected 100)
```

危害：回放按 `tx_id` 判定"已提交"。回滚事务 + 同 id 的后续事务 = 明确丢弃的行被恢复。

## 4. 修复要点

- `crates/storage/src/wal_record_codec.rs`（新）：编解码唯一来源；V2 = 1 字节类型码 + `u32` 长度；`V2` 魔数自识别，V1 兼容解码
- `FileStorage`：`wal: Mutex<Option<Box<dyn WalManager>>>`；5 条 DML 写路径 + 事务边界；`is_wal_enabled()` 覆写；打不开 WAL 时**报错**而非静默降级
- 边界条目在 `current_tx_id` 清零**之前**捕获 tx_id
- append 在 `write_state` 守卫释放之后调用（WAL 追加是文件 I/O）
- `FileBackedWalManager` 拥有 LSN 计数器并盖进条目；`next_lsn_on_disk` 从磁盘高水位续号
- `crates/storage/src/pitr.rs`（新）：`replay_entries_until` + `PitrReport`
- `apply_wal_entry` 从 `RecoveryEngineImpl::apply_entry` 提取，共用（两份实现正是编解码漂移的成因）
- 旧 WAL 缺表名 → 报错，不猜；`"Aa"`/`"BB"` 哈希碰撞 → 显式拒绝
- autocommit（`tx_id == 0`，无边界）按既有约定视为已提交；判据是"整个日志无边界"
- `next_tx_id` 改 `AtomicU64` 单调计数器，从 1 开始
- `pitr::exit_code_for` / `incompleteness_warnings`：exit 3 = 有条目失败，exit 4 = 有已提交事务却 0 条应用

## 5. 测试

| 文件 | 数量 | 状态 |
|---|---|---|
| `crates/storage/tests/file_storage_wal_5055.rs` | 21 | ok |
| `crates/storage/src/wal_record_codec.rs`（单测） | 19 | ok |
| `crates/admin/src/pitr.rs`（单测） | 10 | ok |
| `crates/storage/src/recovery_engine.rs`（resolve 相关） | 4 | ok |

端到端形态：**写 WAL → 恢复到另一个目录 → 重开磁盘核对行**。重开是关键 —— 还在 insert buffer 里的行不会出现在那里；一个把行留在 buffer 就返回的恢复等于什么都没恢复。

## 6. 变异测试：11 个，全部被抓住

| ID | 变异 | 失败测试数 |
|---|---|---|
| M1 | insert 不写 WAL | 6 |
| M2 | LSN 不落盘 | 1 |
| M3 | V2 解码器不认 Point | 2 |
| M4 | V2 Text 去掉长度前缀 | 5 |
| M5 | 回放不检查提交状态 | 3 |
| M6 | 回放去掉 Insert 去重 | 1 |
| M7 | 缺表名时按哈希猜表 | 1 |
| M8 | CLI 退出码退回恒 0 | 3 |
| M9 | tx_id 退回时钟推导 | 1 |
| M10 | 回放忽略目标时间窗 | 1 |
| M11 | COMMIT 记录成 tx 0 | 4 |

**无效变异（明确标注，不计入）**：

- M3 第一版：锚点写成 `b"P:"`，与 rustfmt 后的 `T_POINT` 不匹配 → PATCH-FAIL
- M6 第一版：替换块里引用了作用域外的 `existing` → 编译错误。那是改坏代码不是变异
- M8 第一版：退出码判定内联在 `main.rs`，无法测试。变异跑出 "CAUGHT"，但触发的是**本来就失败**的无关用例 `test_wire_admin_status`（既存缺陷）→ 等于没被抓住。已抽出 `pitr::exit_code_for` 后重做，M8 有效

## 7. 改动的已合并测试 —— 理由

`tests/integration/sql/backup_restore_test.rs` 15 个 `test_pitr_*` + `crates/admin/tests/admin_coverage_tests.rs` 3 个，断言 `pitr_replay_entries` 的只计数契约，函数随修复移除。窗口算术覆盖（目标时间、未决事务、checkpoint、幂等）全部保留，每个用例新增"恢复目标磁盘上实际有什么"的断言。文件内留有说明。

## 8. 基线核对（checkout 到 `fc9e675dfc74` 实跑）

以下失败在基线上同样存在，非本 PR 引入：

- `tests/integration/ddl/alter_table_test.rs` 5 项
- `tests/integration/autoinc_test.rs` `test_autoinc_insert` / `test_autoinc_with_explicit_id` 挂死（`MemoryStorage` RwLock 争用；`sample` 抓栈确认阻塞在 `parking_lot::RwLock::read`）
- `crates/admin/tests/wire_client_integration_tests.rs` `test_wire_admin_status`（`SHOW GLOBAL STATUS` 协议层 null bitmap）
- `crates/storage/tests/phase_c_1_race.rs` `c1_concurrent_begin_commit_rollback`（= #5059）
- clippy `approx_constant`：`crates/storage/tests/vtu_ir_test.rs`、`crates/tools/src/bin/tbl2bin.rs`
- `--all-features` 下 `tests/integration/tpch/v313_3_profile_experiments.rs` 缺编译期常量 `TMPFS_ROOT`

本次改动文件内 clippy 0 error / 0 warning。

## 9. 遗留

- **#5060** `FileStorage` insert_buffer 对 `update`/`delete`/`delete_if`/`update_if` 不可见（SELECT 看得见，UPDATE/DELETE 影响 0 行）。回放用 `force_insert` 绕开，但缺陷本身独立存在。
- **#5059** 并发事务下提交 200 行而非 100。
