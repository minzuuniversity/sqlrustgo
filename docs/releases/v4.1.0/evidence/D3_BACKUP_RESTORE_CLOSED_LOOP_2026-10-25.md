# D3 — #5525「备份→恢复→验证数据」闭环验收凭据 · 2026-10-25

**依据**：Hub Issue #5 comment `198163`（Owner 执行指令，2026-10-25 14:48 CST）

> **D3 数据安全红线五连**（并行前置，P0）：#5525 备份假成功、#5557 I/O hang、#5514/#5582 reaper 误杀空闲事务、
> #5414 WAL 多库恢复。验收=**真实恢复/真实并发场景证据**，禁止「仅不报错」。

> **D1**（PR 队列审核）：#5587（备份/恢复真实化，**必须独立 Reviewer 实跑「备份→恢复→验证数据」闭环**，
> 仅「不报错」不算过）。

**结论先行**：**闭环已实跑通过（4 passed / 0 failed），且逐值验数**，并提供**一条命令**的独立验收入口。
本文件是 #5525 这一条 P0 红线的验收凭据；**其余四条红线（#5557 / #5514 / #5582 / #5414）本轮未开工**。

| 项 | 值 |
|---|---|
| 分支 | `mcode-macmini/backup-restore-closed-loop-5525`（base = `1ab5908c3e` = 批次 46 冻结候选 SHA） |
| 新增文件 | `crates/tools/tests/backup_restore_closed_loop_5525.rs`（4 个用例）<br>`scripts/gate/check_backup_restore_closed_loop.sh`（Reviewer 一条命令入口） |
| 实跑结果 | `cargo test -p sqlrustgo-tools --test backup_restore_closed_loop_5525` → **4 passed / 0 failed / 0 ignored**（rc=0） |
| 证据行 | `EVIDENCE closed_loop backup_id=backup_1792913263 tables=2 rows=5 payload_sha256=decb34e1…27bb6c` |
| 独立 Reviewer | **未做** —— 本文件由 PR 作者自测，**独立复核仍待属主指派**（Owner 指令要求） |
| 需要人工批准 | **是**（合并本 PR、关单 #5525、指派独立 Reviewer） |

---

## 0. 为什么 base 是批次 46 的冻结 SHA，而不是 develop

Owner 指令 D0 要求**冻结候选 SHA**。#5525 的实现（`create_backup` 收类型化表清单、空表集直接报错、
payload 自带 SHA-256）只存在于批次 46 的提交 `1ab5908c3e` 上。`develop/v4.1.0`（`5d4817a06b`）上的
`create_backup` 仍是旧签名：

```rust
// develop/v4.1.0 (5d4817a06b) —— 无类型表映射，且没有空表守卫
pub fn create_backup(&self, database: &str,
                     tables: HashMap<String, Vec<HashMap<String, String>>>) -> ...
```

⇒ **「假成功」的原始形态仍在 develop 上**：一个空表映射会写出一个空文件并被报告为成功。
本闭环跑在**被修复的那份代码**上，所以它证明的是修复，而不是意图。

---

## 1. 闭环做了什么（4 个用例）

| 用例 | 证明的事 | 关键断言 |
|---|---|---|
| `closed_loop_backup_restore_verifies_the_data` | 备份 → 产物 → 恢复 → **逐值比对** → 落入**真实存储引擎** → 读回逐行比对 | `.sql` 里必须出现表名**和**行值（`alice`/`carol`/`12.5`）；`BackupContent` 全等（`PartialEq` 深比较，不是行数）；`restore_into` 落到 `MemoryStorage` 后读回的行集合与源行集合**完全相等** |
| `empty_table_set_is_refused_instead_of_reported_as_success` | 空表集必须**报错**，且**不留产物** | `create_backup(DB, &[])` 必须 `Err`；目录里不得留下 `.sql` / `.data.json` |
| `corrupted_payload_is_detected_before_restore` | 两种破坏都必须在恢复**之前**被拒 | 语义破坏（JSON 合法但内容变）⇒ 错误必须点名 **checksum**；结构破坏（字节翻转，JSON 不可解析）⇒ 错误必须点名 parse/checksum/corrupt |
| `restore_refuses_to_overwrite_an_existing_table` | 恢复是真的写库，且不默认覆盖 | 二次恢复必须 `Err`（`already exists`）；带 `drop_first` 才允许，且行数仍为 3 |

**没有任何一处断言是「调用没有报错」。** 这正是 Owner 指令拒绝的那种证据形状。

---

## 2. Reviewer 的一条命令

```bash
bash scripts/gate/check_backup_restore_closed_loop.sh --json /tmp/closed_loop.json
```

脚本跑闭环、抽取证据行，并给出**三态**结论（与 D2 的判决口径一致）：

- `PASS` —— 闭环跑通且证据行存在；
- `FAIL` —— 闭环跑了但有检查失败；
- `INCONCLUSIVE` —— **没有证据行** ⇒ 测量没看到自己的结果，**绝不当成 PASS**。

退出码：`0=PASS` `1=FAIL` `2=INCONCLUSIVE`。JSON 产物带 `commit` / `backup_id` / `tables` / `rows` /
`payload_sha256`，可直接作为回写 #5 的证据字段。

---

## 3. 本轮实跑记录

```
running 4 tests
test empty_table_set_is_refused_instead_of_reported_as_success ... ok
EVIDENCE closed_loop backup_id=backup_1792913263 tables=2 rows=5 payload_sha256=decb34e1464f9df7415add7fb89cbd8de83c0cfc74b969de0f1e2d3cd527bb6c
test corrupted_payload_is_detected_before_restore ... ok
test closed_loop_backup_restore_verifies_the_data ... ok
test restore_refuses_to_overwrite_an_existing_table ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

编译：`cargo test -p sqlrustgo-tools --test backup_restore_closed_loop_5525`（1.61s 增量，rc=0）。

**一次自更正（本轮内，已修）**：第一版「字节级破坏」用例把 JSON 破坏成**不可解析**，于是命中的是解析错误
而不是校验和分支，断言过窄而失败。改为**语义破坏**（把 `alice` 改成 `mallory`，JSON 仍合法）后才真正命中
envelope 的 SHA-256 守卫；结构破坏另立一条断言，断言放宽为「被拒绝」即可。**失败的是我最初的断言，不是被测代码。**

---

## 4. NOT RUN / 未验证

- **独立 Reviewer 未做**：Owner 指令要求 #5587 由**独立 Reviewer** 实跑闭环；本文件的证据由 PR 作者自测，
  **独立复核仍待属主指派**。这是本条红线**尚未关闭**的唯一原因。
- **未覆盖并发场景**：Owner 的验收措辞是「真实恢复/**真实并发场景**证据」。本闭环覆盖的是**恢复真实性**
  （逐值比对 + 真实存储落库），**并发**部分未做。
- **未覆盖真实 SQL 客户端路径**：闭环走的是 `sqlrustgo_tools::backup_restore` 的 API 级入口
  （`create_backup` / `restore` / `restore_into`），**不是** `sqlrustgo backup` CLI 经由 server 的端到端路径。
- **未验证 `#5557` I/O hang、`#5514`/`#5582` reaper 误杀、`#5414` WAL 多库恢复** —— D3 的其余四条红线本轮未开工。

---

## 5. 复现命令

```bash
# 只跑闭环
cargo test -p sqlrustgo-tools --test backup_restore_closed_loop_5525 -- --nocapture

# Reviewer 一条命令 + 机器可读证据
bash scripts/gate/check_backup_restore_closed_loop.sh --json /tmp/closed_loop.json
echo "rc=$?"   # 0=PASS 1=FAIL 2=INCONCLUSIVE
```

---

**本文件不含任何密码、PAT、API Key、Cookie、私钥或凭据片段。**
