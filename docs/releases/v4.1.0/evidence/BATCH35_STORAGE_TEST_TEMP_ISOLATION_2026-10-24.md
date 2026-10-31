# 批次 35 证据 —— #5571：storage 测试的临时路径唯一化（并发必红 ⇒ 并发必绿）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b35-storage-test-temp-isolation-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 35（本机实测，Mac-mini-7.local arm64 macOS 27.0.1） |
| timestamp | 2026-10-24T06:55:00+08:00 |
| 基线 | `develop/v4.1.0` = `da8c26c3dc` |
| 工作树 | `wt-v410-b35`（分支 `fix/storage-test-temp-isolation-5571`） |
| 关联 issue | #5571（实施）、#5573（同期发现的**另一条**基础设施红） |

---

## 0. 一句话

`crates/storage` 的测试把工作目录放在**固定全局路径**上（`/tmp/bin_test_float` 这类），
而 CI 上 `test-crates` job 与 `postcheck` job 会**并发**跑**同一个**
`sqlrustgo_storage` lib 测试二进制 ⇒ 两份进程同时写同一个目录 ⇒
**结构性的随机红**（单独跑必绿、并发跑必红）。

修法：**一份实现** —— `crates/storage/src/test_support.rs::unique_test_path()`，
单元测试与集成测试共用。

---

## 1. 现象与根因

### 1.1 现象

AFP 门禁的 CHECK 1.5（V312-24 test infra artifacts）间歇性红，失败点全部落在
`sqlrustgo-storage` 的 **lib 测试**里：

```
---- binary_storage::tests::test_binary_storage_load_float stdout ----
panicked at crates/storage/src/binary_storage.rs:1495:45:
called `Result::unwrap()` on an `Err` value: Os { code: 2, kind: NotFound, … }

---- binary_storage::tests::test_binary_storage_load_text stdout ----
panicked at crates/storage/src/binary_storage.rs:1527:38:
called `Result::unwrap()` on an `Err` value: Os { code: 22, kind: InvalidInput, … }
```

同批还有 `replication::tests::test_binlog_writer_and_reader`、
`restore_filespace::tests::test_cleanup_orphaned_table_files` 等。

### 1.2 根因（可测量）

`crates/storage/src/binary_storage.rs`（修复前）：

```rust
fn test_binary_storage_load_float() {
    let tmp = std::env::temp_dir().join("bin_test_float");   // ← 固定路径，无 PID/随机数
    let storage = BinaryTableStorage::new(tmp.clone()).unwrap();
    storage.save("floats", &data).unwrap();
    let loaded = storage.load("floats").unwrap();             // ← 这里 NotFound
    …
    std::fs::remove_dir_all(tmp).ok();                        // ← 只在**结尾**清理
}
```

两个问题叠加：

1. **路径固定** —— `/tmp/bin_test_float` 是全机共享的；
2. **开头不清理** —— 只在**结尾** `remove_dir_all`，上次被中断（kill / OOM /
   机器重启）留下的残骸会被下一次直接复用。

### 1.3 为什么 CI 一定会撞（不是概率问题）

* **跨 job**：CI 的 `test-crates` job 跑
  `cargo test --workspace --exclude sqlrustgo`（含 storage lib 测试），
  `postcheck` job 跑 AFP 门禁，门禁用 `test-runner` 跑**同一个**
  `target/release/sqlrustgo_storage` 二进制 ⇒ 两个 job 在同一台 runner 上
  **并行**跑同一份代码、同一批固定路径。
* **门禁内部**：`test-runner` 的 `--max-parallel` 默认是 `num_cpus`，
  AFP 调用时**没有传** ⇒ manifest 的 5 个条目完全并行启动。

⇒ `postcheck` 会随机红，而 `postcheck` 是 GA 的必经门禁。

---

## 2. 修法（**一份实现**）

### 2.1 唯一实现

`crates/storage/src/test_support.rs`：

```rust
pub fn unique_test_path(prefix: &str) -> PathBuf {
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "sqlrustgo_{prefix}_{}_{}_{seq}",
        std::process::id(),
        *PROCESS_TOKEN
    ));
    remove_any(&path);   // 规则 2：开头清理
    path                 // 规则 3：返回时不存在
}
```

三条规则，缺一不可：

| 规则 | 实现 | 防的是什么 |
|---|---|---|
| **唯一** | `pid` + **进程内随机 token** + 单调计数器 | 并发的两个进程 / 同一进程内的不同测试互相踩 |
| **开头清理** | 返回前删掉同名**目录和文件** | 上次被中断留下的残骸污染本次测试 |
| **返回时不存在** | 返回路径交给测试自己创建 | 与修复前语义完全一致 ⇒ **不用改任何断言** |

**为什么 token 不能只用 pid**：CI 的不同 job 可能跑在**不同的 PID namespace**
（各自的 `getpid()` 是同一个小数字），单靠 pid 并不能区分它们。
`PROCESS_TOKEN` 用 `RandomState` 的 OS 熵种子生成，每个进程一个。

### 2.2 单元测试与集成测试共用同一份实现

集成测试是**独立的 crate**，链接库时**没有** `cfg(test)` ⇒ 与
`fault_injection` 同一手法：

```rust
// crates/storage/src/lib.rs
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
```

```toml
# crates/storage/Cargo.toml —— self dev-dependency 打开 feature
sqlrustgo-storage = { path = ".", features = ["fault-injection", "test-support"] }
```

⇒ 单元测试用 `crate::test_support::unique_test_path`，
集成测试用 `sqlrustgo_storage::test_support::unique_test_path`，
**同一份实现**；生产 `cargo build` 不带这个 feature ⇒ 不进发布产物。

### 2.3 覆盖范围

`crates/storage` 下 **45 个文件**、**约 180 处**调用点全部收敛到这一个函数：

| 文件 | 处数 | 备注 |
|---|---|---|
| `crates/storage/src/file_storage.rs` | 45 | 另有 3 个 `#[cfg(test)] mod` 需要各自 `use` |
| `crates/storage/src/binary_storage.rs` | 41 | 固定名 `bin_test*` / `box_*` |
| `crates/storage/src/append_only_storage.rs` | 13 | **原有自己的 pid+counter 实现** ⇒ 改为转调唯一实现 |
| `crates/storage/src/file_table.rs` | 12 | **原有自己的 pid+counter 实现** ⇒ 改为转调唯一实现 |
| `crates/storage/src/backup.rs` | 10 | 固定名 `test_backup.csv` 等（**文件**路径） |
| `crates/storage/src/columnar/parquet.rs` | 6 | 见 §2.4 的额外发现 |
| `crates/storage/src/binlog_server.rs` | 5 | 固定名 ⇒ 唯一名 |
| `crates/storage/src/integrated_storage.rs` | 4 | 固定名 `sqlrustgo_test_*_v311` |
| `crates/storage/src/restore_filespace.rs` / `pitr_recovery.rs` / `backup_storage.rs` | 3 / 3 / 3 | 固定名 |
| `crates/storage/src/replication.rs` / `recovery_engine.rs` / `mvcc_storage.rs` | 1 / 1 / 1 | 固定名 |
| `tests/**.rs` | 33 个文件 | 集成测试（固定名 / 各自的 pid+nanos 实现） |

### 2.4 顺手修掉的两个更糟的情况（**原有缺陷，与 #5571 同族**）

1. `crates/storage/src/columnar/parquet.rs` 的 6 个测试直接把
   `std::env::temp_dir()`（**就是 `/tmp` 本身**）当作 FileStorage 的数据目录
   ⇒ 测试把表文件直接写在 **`/tmp` 根目录**上。现在是一个唯一子目录。
2. `crates/storage/src/replication.rs` / `crates/storage/src/binlog_server.rs` 也用 `/tmp` 作 binlog 文件的
   基准目录 ⇒ 同样是全机共享。现在是唯一文件路径。

---

## 3. 判别力证据（**变异测试**）

改动的判别力必须能证明「机制真的在起作用」，而不是「碰巧绿」。

### 3.1 修复前 ⇒ 红的基线

同一个二进制、同一套命令，两个进程并发：

```
$ (./target/release/sqlrustgo_storage --test-threads=1 > /tmp/c1.log 2>&1 & \
   ./target/release/sqlrustgo_storage --test-threads=1 > /tmp/c2.log 2>&1 & wait)
$ grep "test result:" /tmp/c1.log /tmp/c2.log
/tmp/c1.log:test result: FAILED. 912 passed; 1 failed; 3 ignored; …
/tmp/c2.log:test result: FAILED. 911 passed; 2 failed; 3 ignored; …
$ grep "panicked at" /tmp/c1.log /tmp/c2.log
/c1.log: thread 'backup::tests::test_export_csv_with_quotes' panicked at crates/storage/src/backup.rs:847:9
/c2.log: thread 'binary_storage::tests::test_bin_rename_table' panicked at crates/storage/src/binary_storage.rs:1786:44
/c2.log: thread 'binary_storage::tests::test_bin_update' panicked at crates/storage/src/binary_storage.rs:1702:37
```

⇒ **单独跑必绿，并发跑必红**。

### 3.2 修复前 ⇒ 13/20 轮红（同一套命令，修复前的 release 二进制）

```
$ /tmp/conc5571_before.sh <pre-fix release binary> 20
round  1: FAIL | rc1=101 rc2=101 | FAILED. 911 passed; 2 failed; … | FAILED. 911 passed; 2 failed; …
   …（失败点每次都不同：test_bin_delete_all / test_binary_storage_load_float /
      test_binary_storage_load_text / test_binary_storage_multiple_rows /
      test_binary_storage_multiple_tables …）
round 20: OK   | test result: ok. 913 passed; 0 failed; …
=== FAIL ROUNDS: 13 / 20 ===
```

⇒ 修复前 **65% 的轮次是红的**（失败点每次随机 ⇒ 典型的「随机红」）。

### 3.3 修复后 ⇒ 20/20 轮全绿

同一套命令，**连续 20 轮**，每轮两个进程并发：

```
round  1: OK   | test result: ok. 913 passed; 0 failed; 3 ignored; … | 913 passed; 0 failed; …
…
round 20: OK   | test result: ok. 913 passed; 0 failed; 3 ignored; … | 913 passed; 0 failed; …
=== FAIL ROUNDS: 0 / 20 ===
```

（逐轮日志留存于 `/tmp/c5571_rounds/`，二进制
`target/debug/deps/sqlrustgo_storage-be7f4a078002e1cc`，工作树
`wt-v410-b35`，HEAD = 本分支。）

### 3.4 修复引入的回归也测到了（**诚实记录**）

改动初期 `file_storage::tests::test_new_with_wal` 红了：

```
assertion failed: storage.data_dir.ends_with("file_storage_wal")
```

原因：那条断言依赖「目录名以固定串结尾」，而唯一化后目录名带上了后缀。
**断言改成了它真正想说的那句话**：

```rust
// `data_dir` **就是**调用方传进来的那个目录
assert_eq!(storage.data_dir, temp_dir);
```

（改前 `ends_with` 只能证明「名字对」，改后 `==` 证明「就是它」。断言变强了。）

### 3.5 全量扫描确认没有漏网

```bash
$ grep -rn 'env::temp_dir()' crates/storage/src crates/storage/tests | grep -v test_support.rs
（只剩 test_support.rs 自身）
```

⇒ `crates/storage` 里**不再有第二份**临时路径生成逻辑。

---

## 4. 编译与门禁

| 检查 | 结果 |
|---|---|
| `cargo check --tests -p sqlrustgo-storage` | **0 error** |
| `cargo test -p sqlrustgo-storage --lib` | **913 passed / 0 failed / 3 ignored** |
| `cargo clippy --all-features --all-targets` | 见 §5（CI 侧由 `lint-build` job 覆盖） |

---

## 5. 与 #5573 的边界（**两件事，不要混**）

批次 35 同期还定位到**另一条** CI 红的机制，与本 issue **无关**：

* 本 issue（#5571）：**代码侧**缺陷 —— 测试用固定全局临时路径 ⇒ 并发必红。
  本 PR 修它。
* #5573：**基础设施侧** —— act_runner 每 6–9 分钟被要求优雅关机 ⇒
  在飞 job 容器被 `pause` ⇒ 5 分钟宽限到点 ⇒ job 判红。
  **本仓只登记不实施**（修法落在宿主机 / 跨仓范围）。

⇒ PR #5572 的 CI 全红是 **#5573** 那条；PR（本批次）的改动是 **#5571**。
两者互不顶替。

---

## 6. 变更清单

```
 crates/storage/Cargo.toml                        | 12 +++-
 crates/storage/src/lib.rs                        | 10 ++++
 crates/storage/src/test_support.rs               | 130 +++++++++++++++++++ (新增，唯一实现)
 crates/storage/src/{file_storage,binary_storage,…}.rs
 crates/storage/src/columnar/parquet.rs           |
 crates/storage/tests/**.rs                       | 33 个文件
```

**未触碰** `crates/storage/src/engine.rs`（另一 Agent 在飞）。
