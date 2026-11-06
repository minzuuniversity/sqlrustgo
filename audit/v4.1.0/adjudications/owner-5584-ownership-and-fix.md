待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**待决**：#5584（看板 ENG-13）—— 归属与修法。
  事实 1：该 issue 带 `state/claimed` + `agent:openclaw-mcode`（另一条线认领）。
  事实 2：认领方**至今没有对该缺陷提交任何修复**；看板行 ENG-13 一直是「阻塞」。
  事实 3：缺陷**在本轮仍然复现**：归档 run `20261106-p22fl1-001` 里
  `crash-wal-index` 仍红（`crash_wal_index_b1` 的
  `index_update_crash_does_not_serve_dangling_rows` FAILED）—— 修法见 run
  `20261106-board-001` 的同一条记录。
  事实 4：约束：`crates/storage/src/fault_coordination.rs` 与 fault-injection
  harness 属于另一条线的 claim 范围，本线未改动。

要裁的是：
  (1) **归属**：认领方长期不动 ⇒ 归属是否移交给本线（或解除认领、走公开认领流程）？
  (2) **修法取舍**（若归属移交）：
      甲：索引更新边界的 WAL 顺序性问题（把索引更新的 WAL 记录与数据页写入的
          顺序对齐，使崩溃后索引与数据一致）；
      乙：崩溃恢复时按数据重建索引（以数据为准，索引可重建）；
      丙：把「索引更新边界崩溃」标记为不支持场景（收窄承诺，需产品口径）。
  (3) **能不能在不动 fault-injection harness 的前提下**做出可验证的修复
      （验收要求：armed 构建下 `crash_wal_index_b1` 两个测试全绿；SIGKILL 重启后
      SELECT 必须返回已提交的行，不得以 `Table not found` 结束 seed）。

**材料**：
- `crates/storage/tests/crash_wal_index_b1.rs`（两个用例的当前断言）。
- 归档 run `20261106-p22fl1-001` 的 `logs/025-crash-wal-index.stdout`
  （1 passed / 1 failed，失败项即上面那条）。
- 看板行 `ENG-13`（含现场实测与验收条款）。
- `audit/v4.1.0/adjudications/` 下与 #5376 相关的记录（崩溃 harness 的 claim 事实）。

**判据**：对 (1)(2)(3) 给出明确结论；若你认为归属移交前必须先由人解除 claim，
请指出需要人做的**最小动作**。
材料不足 ⇒ INCOMPLETE。
