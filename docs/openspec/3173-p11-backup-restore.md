<!-- env:blocked:no-ci -->

# openspec/3173 - P1-1 Backup/Restore/Verify CLI + PITR

> **Issue**: [#3173](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3173)
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 3 (W5-6)
> **工作量**: 40h (~5 天)
> **优先级**: P1 (35%, 可靠性)
> **Milestone**: v3.9.0 (id=32, due 2026-09-23)
> **Label**: v3.9.0, P1-reliability, phase-3

## 一、问题分析

### 1.1 背景

v3.8.0 已具备 WAL + MVCC 事务, 但**没有 backup/restore 工具**. 一旦数据损坏或误操作, 数据无法恢复, 不满足"生产就绪"标准.

Per V390_VERSION_PLAN §1.2 核心反转:
> 不是"支持多少 SQL", 而是"数据库死了以后还能不能回来".

P1-1 是 v3.9.0 Production Readiness Release 的核心可靠性能力.

### 1.2 现有基础设施 (2026-06-05 审计)

| 组件 | 位置 | 状态 |
|------|------|------|
| `WalEntry` (含 `timestamp: u64` + `lsn: u64`) | `crates/storage/src/wal_legacy.rs` | ✅ 完整 |
| `WalManager` trait (append/flush) | `crates/storage/src/wal/mod.rs` | ✅ 完整 |
| `FileBackedWalManager` (物理 WAL) | `crates/storage/src/wal/file_backed_wal_manager.rs` | ✅ 完整 |
| `RecoveryEngine` (WAL replay) | `crates/storage/src/recovery_engine.rs` | ✅ 完整 |
| `WalStorage` (data + WAL 组合) | `crates/storage/src/wal_storage.rs` | ✅ 完整 |
| `BackupExporter` (CSV/JSON/SQL export, **逻辑**) | `crates/storage/src/backup.rs` | ✅ 现有 |
| `BackupStorage` (本地 + S3 抽象) | `crates/storage/src/backup_storage.rs` | ✅ 现有 |
| `backup_restore` (mysqldump 风格元数据) | `crates/tools/src/backup_restore.rs` | ✅ 现有 (但仅元数据) |
| **物理 backup CLI** | (无) | ❌ 缺失 |
| **物理 restore CLI** | (无) | ❌ 缺失 |
| **verify CLI** | (无) | ❌ 缺失 |
| **PITR** (Point-In-Time Recovery) | (无) | ❌ 缺失 |

### 1.3 目标

新增 4 个 CLI 工具:

```bash
sqlrustgo-admin backup     --data-dir <path> --output <path> [--format tar|dir]
sqlrustgo-admin restore    --input <path> --target-dir <path>
sqlrustgo-admin verify     --input <path>
sqlrustgo-admin pitr       --data-dir <path> --wal-dir <path> --target-time <TIMESTAMP>
```

## 二、变更设计

### 2.1 新 crate `sqlrustgo-admin`

**位置**: `crates/admin/` (新建)

**结构**:
```
crates/admin/
├── Cargo.toml
└── src/
    ├── main.rs       # CLI entry, clap-based subcommand dispatch
    ├── backup.rs     # 物理 backup 实现
    ├── restore.rs    # 物理 restore 实现
    ├── verify.rs     # 备份校验
    ├── pitr.rs       # PITR (WAL replay to time)
    ├── manifest.rs   # 备份元数据 (manifest.json with checksums)
    └── tests.rs      # 单元测试
```

**依赖**:
- `sqlrustgo-storage` (WalStorage, FileBackedWalManager, WalEntry)
- `sqlrustgo-types` (SqlResult, SqlError)
- `clap` 4 (CLI parsing, 已有 workspace dep)
- `serde` + `serde_json` (manifest serialization)
- `sha2` (SHA-256 checksums, 需要加 workspace dep)
- `flate2` (gzip 压缩, 已有)

**Workspace 集成**: 在根 `Cargo.toml` `[workspace]` `members` 加 `"crates/admin"`.

### 2.2 备份格式 (tar.gz)

**manifest.json** (per backup):
```json
{
  "version": 1,
  "created_at": "2026-06-05T07:00:00Z",
  "sqlrustgo_version": "3.9.0-alpha1",
  "data_dir": "/var/lib/sqlrustgo/data",
  "wal_file": "sqlrustgo.wal",
  "wal_size_bytes": 1048576,
  "data_files": [
    { "path": "table_1.json", "size": 4096, "sha256": "abc..." },
    { "path": "table_2.json", "size": 8192, "sha256": "def..." }
  ],
  "wal_checksum": "xyz...",
  "total_size_bytes": 16384
}
```

**物理布局**:
```
backup.tar.gz
├── manifest.json
├── data/
│   ├── table_1.json
│   ├── table_2.json
│   └── ...
└── wal/
    └── sqlrustgo.wal
```

### 2.3 backup 命令

**算法**:
1. 验证 data_dir 存在, 可读
2. 遍历 data_dir 所有文件, 计算 SHA-256
3. 读取 WAL 文件 (若存在), 计算 SHA-256
4. 构造 manifest.json
5. 打包: data/* + wal/sqlrustgo.wal + manifest.json → tar.gz

**伪代码**:
```rust
pub fn physical_backup(data_dir: &Path, output: &Path) -> SqlResult<BackupResult> {
    let manifest = Manifest::scan(data_dir)?;
    let tar = tar::Builder::new(flate2::write::GzEncoder::new(File::create(output)?, 6));
    tar.append_path_with_name(manifest.to_json()?, "manifest.json")?;
    for entry in manifest.data_files {
        tar.append_path_with_name(data_dir.join(&entry.path), format!("data/{}", entry.path))?;
    }
    if let Some(wal) = &manifest.wal_path {
        tar.append_path_with_name(wal, "wal/sqlrustgo.wal")?;
    }
    tar.finish()?;
    Ok(BackupResult { manifest, output_size: fs::metadata(output)?.len() })
}
```

### 2.4 restore 命令

**算法**:
1. 验证 input 存在 (tar.gz 或 dir)
2. 解压到临时目录
3. 读取 manifest.json
4. **校验**: 重新计算 SHA-256, 与 manifest 比对
5. 校验失败 → 报错 (verify failure)
6. 校验成功 → 复制 data/* 到 target_dir
7. 复制 wal/sqlrustgo.wal 到 target_dir

**校验** (与 verify 命令共用):
```rust
pub fn verify_backup(input: &Path) -> SqlResult<VerifyResult> {
    let manifest = Manifest::extract(input)?;
    let mut errors = Vec::new();
    for entry in &manifest.data_files {
        let actual = sha256(extract_file(input, &format!("data/{}", entry.path))?);
        if actual != entry.sha256 {
            errors.push(VerifyError::ChecksumMismatch { path: entry.path.clone() });
        }
    }
    if let Some(wal_sha) = &manifest.wal_checksum {
        let actual = sha256(extract_file(input, "wal/sqlrustgo.wal")?);
        if &actual != wal_sha {
            errors.push(VerifyError::WalChecksumMismatch);
        }
    }
    Ok(VerifyResult { manifest, errors })
}
```

### 2.5 PITR 命令

**算法**:
1. 读取 data_dir 当前状态 (full backup)
2. 读取 wal/sqlrustgo.wal 所有 entries
3. 过滤: 保留 `entry.timestamp <= target_time` 的 entries
4. 应用 entries 到 data_dir (类似 RecoveryEngine.recover)
5. 删除未完成事务 (no Commit)

**伪代码**:
```rust
pub fn pitr(data_dir: &Path, wal_path: &Path, target_time: u64) -> SqlResult<PitrResult> {
    let entries = WalReader::read_all(wal_path)?;
    let target_entries: Vec<_> = entries.iter()
        .filter(|e| e.timestamp <= target_time)
        .collect();
    let mut committed_txns = HashSet::new();
    let mut active_txns = HashSet::new();
    let mut applied = 0;
    let mut skipped = 0;
    for entry in &target_entries {
        match entry.entry_type {
            WalEntryType::Begin => { active_txns.insert(entry.tx_id); }
            WalEntryType::Commit => { committed_txns.insert(entry.tx_id); active_txns.remove(&entry.tx_id); }
            WalEntryType::Rollback => { active_txns.remove(&entry.tx_id); }
            _ => {
                if committed_txns.contains(&entry.tx_id) {
                    apply_entry_to_data_dir(data_dir, entry)?;
                    applied += 1;
                } else {
                    skipped += 1;
                }
            }
        }
    }
    // Uncommitted txns at target_time are aborted
    Ok(PitrResult { applied, skipped, aborted: active_txns.len() })
}
```

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| tar.gz 损坏 (传输错误) | 高 | SHA-256 校验每个文件 + manifest |
| 大数据库 (10GB+) 备份 OOM | 中 | 流式 tar, 不加载整个文件到内存 |
| PITR 时间精度 | 中 | u64 unix timestamp, 精度 1s (per G6 门禁) |
| 并发备份 (data dir 写入中) | 高 | 文档说明: 备份前应停止服务 或用 `FLUSH TABLES WITH READ LOCK` (v3.10+ 加) |
| 备份恢复后数据不一致 | 中 | verify 强制在校验失败时拒绝恢复 |
| tar crate 体积 | 低 | 已有 serde 依赖, tar 增量小 |
| sha2 编译时间 | 低 | 已有加密依赖 (rcgen) |
| 50+ tests 跑时间 | 中 | 大部分用 tempfile + 小数据集, < 1s/test |

## 四、测试策略

### 4.1 单元测试 (`crates/admin/src/tests.rs`)

| 类别 | tests |
|------|-------|
| Manifest | 5 (scan, serialize, deserialize, empty, large) |
| Checksum | 3 (file, dir, mismatch detection) |
| Tar pack/unpack | 4 (round-trip, gzip, error) |
| WAL filter | 4 (by time, by tx, by entry_type) |
| PITR | 4 (commit replay, rollback skip, partial) |
| **小计** | **20** |

### 4.2 e2e tests (`tests/backup_restore_test.rs`)

| 类别 | tests |
|------|-------|
| **基础功能** | 20 |
| - test_backup_full |  |
| - test_backup_empty_db |  |
| - test_backup_with_lots_of_tables (10 tables) |  |
| - test_backup_with_wal |  |
| - test_restore_full |  |
| - test_restore_overwrites_target |  |
| - test_restore_interrupted (中途 kill) |  |
| - test_verify_ok |  |
| - test_verify_corrupted_data |  |
| - test_verify_corrupted_wal |  |
| - test_verify_missing_file |  |
| - test_round_trip_preserves_data |  |
| - test_backup_then_restore_equals_original |  |
| - test_backup_incremental (基于 WAL LSN) |  |
| - test_restore_incremental |  |
| - test_backup_with_active_tx (未提交) |  |
| - test_backup_disk_full_simulation |  |
| - test_backup_with_large_wal (10MB+) |  |
| - test_backup_idempotent (same dir → same checksum) |  |
| - test_backup_creates_unique_output |  |
| **PITR** | 15 |
| - test_pitr_to_specific_timestamp |  |
| - test_pitr_with_active_tx_at_target |  |
| - test_pitr_with_dropped_table |  |
| - test_pitr_at_beginning_of_wal |  |
| - test_pitr_at_end_of_wal |  |
| - test_pitr_middle_of_transaction |  |
| - test_pitr_preserves_committed_data |  |
| - test_pitr_skips_uncommitted_data |  |
| - test_pitr_time_precision (1s) |  |
| - test_pitr_with_empty_wal |  |
| - test_pitr_with_corrupted_wal |  |
| - test_pitr_replays_checkpoints |  |
| - test_pitr_idempotent |  |
| - test_pitr_at_exact_entry_timestamp |  |
| - test_pitr_after_recovery |  |
| **错误恢复** | 15 |
| - test_restore_to_nonexistent_dir |  |
| - test_restore_corrupted_tar |  |
| - test_verify_invalid_manifest |  |
| - test_backup_with_readonly_data |  |
| - test_restore_to_readonly_target |  |
| - test_backup_concurrent (多进程) |  |
| - test_restore_with_disk_full |  |
| - test_verify_with_truncated_file |  |
| - test_backup_with_symlinks |  |
| - test_restore_preserves_permissions |  |
| - test_backup_metadata_persistence |  |
| - test_restore_with_older_version_warning |  |
| - test_backup_then_modify_then_backup |  |
| - test_verify_after_data_modification |  |
| - test_backup_then_delete_source |  |
| **小计** | **50** |

**总 e2e tests**: 50 (per V390_TEST_PLAN §G6 要求)

## 五、门禁 (G6)

**位置**: `scripts/gate/check_backup_restore.sh` (新建, ~100 行)

**检查项**:
1. `crates/admin/` 存在 + `Cargo.toml` 有 `[[bin]] name = "sqlrustgo-admin"`
2. 4 个子命令 (backup/restore/verify/pitr) 在 `main.rs` 用 clap 定义
3. `tests/backup_restore_test.rs` 至少 50 tests
4. `cargo test -p sqlrustgo-admin` 全部 PASS
5. 端到端 GMP 场景 (CLI 跑一遍)

## 六、实施步骤 (按 V390_DEVELOPMENT_PLAN §P1-1)

| # | 步骤 | 文件 | 工作量 |
|---|------|------|--------|
| 1 | SPEC 编写 (本文档) | `docs/openspec/3173-...` | 2h |
| 2 | 创建 `sqlrustgo-admin` crate + main.rs (clap) | `crates/admin/` | 4h |
| 3 | manifest.rs + sha256 checksum | `crates/admin/src/manifest.rs` | 3h |
| 4 | backup.rs (物理 tar.gz) | `crates/admin/src/backup.rs` | 6h |
| 5 | restore.rs + verify.rs (共用校验) | `crates/admin/src/{restore,verify}.rs` | 6h |
| 6 | pitr.rs (WAL replay) | `crates/admin/src/pitr.rs` | 6h |
| 7 | 50 e2e tests | `tests/backup_restore_test.rs` | 8h |
| 8 | G6 gate script | `scripts/gate/check_backup_restore.sh` | 2h |
| 9 | clippy + fmt + 全量 test | — | 2h |
| 10 | 报告 + PR | `docs/releases/v3.9.0/reports/BACKUP_RESTORE_REPORT.md` | 1h |
| **合计** | | | **40h** |

## 七、交付物清单

| 类别 | 文件 | 大小预估 |
|------|------|----------|
| Spec | `docs/openspec/3173-p11-backup-restore.md` (本文件) | 350 行 |
| Report | `docs/releases/v3.9.0/reports/BACKUP_RESTORE_REPORT.md` (后续) | 200 行 |
| Crate | `crates/admin/Cargo.toml` (新) | 30 |
| Code | `crates/admin/src/main.rs` (clap) | 80 |
| Code | `crates/admin/src/manifest.rs` | 120 |
| Code | `crates/admin/src/backup.rs` | 180 |
| Code | `crates/admin/src/restore.rs` | 100 |
| Code | `crates/admin/src/verify.rs` | 100 |
| Code | `crates/admin/src/pitr.rs` | 180 |
| Code | `crates/admin/src/tests.rs` (20 unit tests) | 250 |
| Test | `tests/backup_restore_test.rs` (50 e2e tests) | 600 |
| Gate | `scripts/gate/check_backup_restore.sh` | 100 |
| Cargo | 根 `Cargo.toml` (workspace members + sha2) | +3 |
| **合计** | | **~2293 行** |

## 八、依赖关系

### 8.1 编译依赖 (无)

- WalStorage, FileBackedWalManager, WalEntry, RecoveryEngine 已存在
- 复用 `crates/storage` 的 WAL API
- 不依赖 P0-4 (Savepoint) — PITR 用 RecoveryEngine 不需 Savepoint

### 8.2 流程依赖

- v3.9.0-alpha1 (含 P0-1, P0-2, P0-3)
- Workspace 已含 `clap`, `serde`, `serde_json`, `flate2`
- 需添加 `sha2 = "0.10"` 和 `tar = "0.4"` 到 `[workspace.dependencies]`

### 8.3 测试依赖

- 不需要外部服务
- 用 `tempfile` (已有) 创建临时 data_dir

## 九、Issue 关闭条件 (per §3.1)

满足 4 项:
1. ✅ PR 关联 (`fix/issue-3173-p11-backup-restore` → `develop/v3.9.0`)
2. ✅ 50+ e2e tests PASS
3. ✅ 4 CLI 工具 (`backup`, `restore`, `verify`, `pitr`) 端到端可用
4. ✅ G6 门禁 PASS (`check_backup_restore.sh` exit 0)

## 十、参考

- Issue #3173: <http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3173>
- 计划: `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md` §P1-1
- 测试: `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` §G6
- 现有 WAL: `crates/storage/src/wal_legacy.rs` (WalEntry, timestamp)
- 现有 Recovery: `crates/storage/src/recovery_engine.rs`
- 现有工具: `crates/tools/src/backup_restore.rs` (mysqldump 风格)
- 治理: `docs/governance/ISSUE_CLOSING_VERIFICATION.md` §3.1
