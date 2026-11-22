# 权威认证轮 008 —— **主动中止**（ABORTED）

> 本目录是一轮**未跑满**的认证轮。中止者是操作者（Agent），不是门禁判红。
> 本文件如实登记中止的时点、已跑完的部分、以及**中止原因**。

## 1. 中止事实

| 项 | 值 |
|---|---|
| run_id | `20261117-ga-gates-008` |
| 被测树 | `1579950048b37628eb19bb9f2fe624ab87198ada`（`develop/v4.1.0`） |
| plan | 35 项（`plan.jsonl` 的 sha256 与 007 **完全一致** ⇒ 分母与判据未漂移） |
| 已执行 | **19 / 35**，全部 **PASS**（seq1–seq19） |
| 未执行 | seq20–seq35（**从未开始**，不是失败、不是 INCOMPLETE） |
| 中止时点 | 2026-11-17T04:50Z（seq20 运行中） |
| 中止后处理 | 进程树全部终止；已跑的 19 项结果保留在本目录 `results.jsonl` |

## 2. 中止原因（**责任在操作者，不在被测代码**）

seq20 = `scripts/gate/check_full_gate_verification.sh`，其内部维度 **D6b Test Inventory**
由 `scripts/gate/check_test_inventory.sh` 实现。该脚本对 `tests/` 下 **487 个测试目标**
**逐个**执行：

```bash
OUTPUT=$(timeout 180 cargo test --test "$name" -- --test-threads=1 2>&1 || echo "0")
```

每一项都是**独立的一次 cargo 调用**。本轮被测树相对上一轮**改过**
（#5834 的 `crates/storage/src/file_storage.rs` undo 归属按连接修复），
⇒ 依赖 `crates/storage` 的**全部测试目标**的构建缓存同时失效。
于是 D6b 每跑一个目标，都要**重新编译并链接一个测试二进制**。

**实测**：D6b 于 03:59Z 开始；04:44:45 跑到第 193 个目标
（`anti_join_main_path_test`），04:47:04 跑到第 197 个
（`bufferpool_pages_visited_memory_test`）⇒ **约 35 s / 目标**。
剩余 290 个目标 ⇒ **约 2.8 小时**，而单门禁预算 `--timeout-sec 7200`（2 小时）
在 **05:31Z** 到期 ⇒ seq20 必然超时判红。

**这不是被测代码的缺陷**：同一条 `check_test_inventory.sh` 在 007 轮只用了约 1100 s
（那时测试目标的构建缓存是热的）。**这是本轮启动条件的缺陷** ——
只预构建了 3 个二进制（#5836 的教训），**没有预构建 482 个测试目标**。

## 3. 处置

1. **中止本轮**，保留本目录全部已跑证据（seq1–seq19 全 PASS）。
2. 在交付树上执行 `cargo test --no-run`（默认 features）**一次性预构建全部测试目标**，
   把「逐个编译」的成本从门禁时间里挪到轮次之外。
3. 后续权威轮在**缓存已预热**的交付树上重跑，seq20 的 D6b 回到 007 的耗时量级。
4. **不改任何判据、不改阈值、不改门禁脚本** —— 本轮中止不影响 35 项的分母与判据。

### 3.1 处置是否生效 —— 有实测

预构建后，在**同一棵交付树**上探一个靠后的测试目标（排序第 300 位
`integration_sql_repro_v312_95_4809`）：

```
cargo test --test integration_sql_repro_v312_95_4809 --no-run
real 9.21   user 0.09   sys 0.08
```

**9.21 s 的墙钟里只有 0.17 s 是自己的 CPU** —— 其余全是在等另一个 cargo 进程
释放 target 目录锁，**说明它没有触发任何编译**（产物已是热的）。
⇒ D6b 的 487 次 `cargo test --test <name>` 现在都是「跑已编译好的二进制」，
耗时回到 007 轮那个量级（**~2.3 s/目标**，而不是中止前的 34.8 s/目标）。

**对照**：中止前实测 34.8 s/目标（见 `abort_facts.yaml` 的 `derived` 段）。

## 4. 读这份证据时必须知道的事

- 本目录的 `results.jsonl` **只有 19 行**，不是 35 行。
- **不得**把「19/19 PASS」读成「19/35 通过」或「这轮比别的轮好」——
  剩下 16 项**根本没有运行**。
- 本轮**不产生**总账结论；权威结论以跑满 35 项的后续轮为准。
- 本目录的 `manifest.yaml` 里 `results` 段只有 19 条，与 `plan.jsonl` 的 35 项
  **不是一一对应**，这是**故意的**（如实记录中止事实），不是清单损坏。

## 5. 后续实测补充（2026-11-17，010 轮起跑前）—— **第 2 节的「35 s/目标」要打折读**

第 2 节把 D6b 的 34.8 s/目标归因为「树改过 ⇒ 依赖 crates/storage 的测试目标缓存失效 ⇒
逐个重新编译并链接」。这个归因**方向正确但不是全部**：010 轮起跑前的体检发现，
`/Volumes/workspace` 当时 **97% 满（可用 34 GiB）**，该卷上并存约 30 个 sqlrustgo 工作树、
每个都带一个 `target/`。在这种状态下实测：

- `ls target/debug/deps` 单次 **> 25 s 超时**；
- 元测试预检里**每一个嵌套 cargo 都要 45–60 s**，而对应测试本身
  `finished in 0.00s` ⇒ 时间不在测试上，也不在单纯的编译上，
  而在 **cargo 启动 + 指纹扫描 + 文件系统颠簸**。

处置：回收废弃工作树与专用 target 目录的构建缓存（`cargo clean`，**纯产物、可重建**；
交付树/修复树/材料树的目标目录一律未动）⇒ 可用空间 **34 GiB → 236 GiB**，
同机同命令 `cargo build --all-features` 从 50 s → **1 s**（命中缓存）。

⇒ **⇒ 本轮中止时的 34.8 s/目标，应当读作「冷缓存 + 磁盘 97% 满」下的数字**；
它**不是** D6b 在健康环境下的固有成本。第 3 节写的「预构建 482 个测试目标」这条处置方向正确，
但**仅预构建还不够，磁盘余量同样属于启动条件**。

（第 2 节引用的 007 轮 ~2.3 s/目标，正是「缓存热 + 磁盘没满」时的数字。）

补充：009 轮进一步查明，嵌套链里 `cargo test -p sqlrustgo-tools -- --nocapture` 这一层
**本来就要 660 s（11 min）** —— tools 包的 `v400_multi_model_soak.rs` 是压缩时间等价的
soak，24h/72h/168h 三档分别等价 60 s / 180 s / 420 s（源码行 370/411/442）。
详见 009 轮 `ABORTED.md` 第 6 节。

### 5.1 **真凶：rustc 每次启动要全目录扫描 76 万个文件**

010 轮起跑前用 `sample` 抓到 rustc 的栈：

```
rustc_session::search_paths::SearchPath::from_cli_opt → SearchPath::new
  → std::fs::ReadDir → readdir_r → __getdirentries64      ← 99.5% 的采样
```

rustc 启动时要 readdir 它的搜索路径，其中就有 `target/debug/deps`；
该目录实测 **764 176 个文件**（历轮构建 + 增量缓存累积）。

⇒ **每次 rustc 启动 = 一次 76 万项目录扫描 = 30–60 s**
⇒ 本轮实测的「D6b 每目标 34.8 s」就是这么来的：**不是编译慢，是数文件慢**
（同一时刻 `ls target/debug/deps` 单次 > 25 s 超时，也是同一个 readdir）。

⇒ 处置（010 轮已实测生效）：`cargo clean` 清空 target/ + `CARGO_INCREMENTAL=0`
⇒ deps 目录只剩「二进制 + .d + rlib」量级 ⇒ readdir 回到毫秒级。
