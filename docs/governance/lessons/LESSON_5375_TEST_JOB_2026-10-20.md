# LESSON_5375_TEST_JOB_2026-10-20.md

> **来源 Issue**: #5375
> **来源 Run**: fix/5375-test-job-compile-failures @ `7a73c42397`
> **日期**: 2026-10-20
> **证据**: `docs/releases/v4.1.0/evidence/CI_TEST_JOB_5375_2026-10-20.md`

---

## 概述

本轮迭代修的是 CI `test` job 的编译失败。修的过程中暴露了 4 条与「表面
症状」无关的元教训，全部指向同一件事：**报告的失败面 ≠ 真实的失败面**。

---

## Lesson-01: 第一个失败目标 ≠ 全部失败目标

### Observation

issue #5375 记录「编译失败的测试目标数 = 1」（`partition_e2e_test`）。
用 `cargo check --tests --keep-going` 实测：**11 个目标、46 个错误**。

### Root Cause

`cargo` 默认**遇错即停**——第一个目标失败后就不再编译其余目标。CI 日志
里因此永远只能看到第一个失败目标，而 issue 作者把那个数字当成了全部。

### Impact

如果只按 issue 正文修，`test` job 只会从「第 1 个失败目标」变成「第 2 个
失败目标」，看起来像修好了，实际一步没前进。

### Mitigation

- 任何「编译失败」类调查，第一条命令必须是
  `cargo check --workspace --all-features --tests --keep-going`；
- issue 的「失败目标数」只能当作**下界**，不能当作事实。

### Future Prevention

- 在 issue 模板里把「失败目标数」改成「`--keep-going` 下的全部失败目标」；
- 本地复现编译失败时，默认加 `--keep-going`。

---

## Lesson-02: 测试基础设施的路径 bug 会伪装成「环境缺件」

### Observation

4 条崩溃测试全部以 `spawn server: No such file or directory (os error 2)`
瞬间失败，看上去像「本地没编 server 二进制」。

### Root Cause

`server_bin()` 返回**相对**路径 `target/release/sqlrustgo-mysql-server`，
而 cargo 跑集成测试时的工作目录是**包目录** `crates/storage/`。相对路径
因此指向 `crates/storage/target/release/…`——一个从不存在、也永远不会
被 cargo 创建的目录。

### Impact

这 4 条测试**从未真正执行过**，却一直占据「测试矩阵」的名额。它们声称
证明「在 WAL fsync 边界崩溃后数据仍在」，实际连崩溃都没发生过。

### Mitigation

- 从 `CARGO_MANIFEST_DIR` 推导工作区根，不要用相对路径指向工作区产物；
- 找不到时给出**可操作**的报错（缺哪个文件、怎么编），而不是让
  `spawn` 抛出 `No such file or directory` 这种无指向性的错误。

### Future Prevention

- 「环境问题」这个结论在写进 issue 前必须先排除**路径解析错误**；
- 测试里凡是 spawn 外部二进制的，路径一律用 `CARGO_BIN_EXE_*` 或
  `CARGO_MANIFEST_DIR` 推导。

---

## Lesson-03: 「测试通过」不等于「测试证明了它声称证明的事」

### Observation

`committed_transaction_survives_sigkill_at_commit_fsync` 通过了（PASS），
但实测耗时 **240.53s ≈ 2 × 120s**。120 秒正是
`fault_injection::reach()` 的自旋超时。

### Root Cause

`reach()` 写完信号后自旋 120 秒；而 harness 的顺序是**先同步跑 SQL
（客户端阻塞等响应）、再等信号**。于是 SIGKILL 至少晚 120 秒才发出，
落在服务**已恢复服务之后**。

测试证明的是「进程在空闲时被 SIGKILL 后数据仍在」，而不是它文档里写的
「在 COMMIT 的 fsync 边界崩溃后数据仍在」。

### Impact

**假证据**：真正的边界崩溃如果会丢数据，当前测试照样绿。这类缺陷只有
在 harness 本身被审计时才会暴露。

### Mitigation

- 断言测试**证明了什么**，而不只是「它绿了」：control 场景的职责是
  「证明 harness 到达了边界」，就必须直接断言信号文件的存在与身份
  （run_id / pid / 故障点），而不是断言一个**假前提**（本次不发 COMMIT
  ——但 autocommit INSERT 本身就是 COMMIT）。
- 观察**耗时**：一个「毫秒级」的崩溃测试若耗时以分钟计，说明它在等
  超时而不是等事件。

### Future Prevention

- 崩溃/超时类测试必须记录「边界达成」与「信号发出」的**时间差**；
- control 场景的前提要逐条对着产品的真实语义核验。

---

## Lesson-04: 姊妹测试互相矛盾 = 一方在测错的东西

### Observation

`file_storage_concurrent_inserts_via_bufferpool_do_not_corrupt`（64 页池 /
100 条记录 / 断言回读 100 行）与它的 MemoryStorage 姊妹测试（256 页池 /
断言 `pages_visited >= 100`）**互相矛盾**。前者要求的 100 行在任何实现
下都不可达：BufferPool 模式下数据只在池里，64 槽 LRU 在插入时就淘汰
36 页，且没有恢复路径。

### Root Cause

有人按「先写测试再写实现」的顺序补了一个**不可能满足**的断言，而没有
对照姊妹实现核对不变量。

### Mitigation

- 改写时**保意图、换契约**：把「100 行都要回来」拆成两件事——
  「并发插入不损坏页」（用**大于**行数的池验证）与「超配池只损失真正
  被淘汰的页」（用**小于**行数的池验证）。
- 两者分别有独立的回归测试，淘汰行为由新测试的不变量钉住：
  回读行数 == 池容量，且扫描本身零淘汰。

### Future Prevention

- 新增与既有测试同名的测试前，先读同名的姊妹实现；
- 断言一个「不可达的结果」时，先算一遍：容量、淘汰策略、数据在哪。

---

## 遗留（已立项，不在本轮自批）

- **#5376**：崩溃 harness 的 SIGKILL 落在边界之后——测试没有证明它声称
  证明的事；`index_update_crash_does_not_serve_dangling_rows` 的 FAILED
  与「>10 分钟未结束」都发生在 kill 落点错误的前提下，尚不能判定是恢复
  逻辑缺陷还是 harness 缺陷。
- **#5377**：`create_index` 没有 `_in_db` 变体，命名库的索引既建不了也
  查不到。
