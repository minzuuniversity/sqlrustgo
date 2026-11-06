待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**主张/待决**：#5601（看板 BIN-1）—— `--storage binary` 模式的产品口径该怎么定：
binary 模式下 `CREATE/DROP DATABASE`、`CREATE/DROP TRIGGER` 现在**明确报错**
（`crates/storage/src/binary_storage.rs`，PR #5610）；此前它们返回 `Ok(())` ⇒
「报成功却零效果」的假成功。

要裁的是**产品口径**，不是代码写法：
  选项 A：维持「明确拒绝」。binary 是只读 benchmark 后端（数据目录即全部，
    无 per-db 子目录、无 WAL ⇒ 触发器无法登记也无法执行）。CREATE DATABASE /
    CREATE TRIGGER 一律报错，文档写明该模式不支持 DDL。
  选项 B：实现库命名空间（binary 模式按 db 名分目录）与触发器登记（需要为
    非 WAL 存储单独设计触发器持久化与执行语义）。
  选项 C：停止对外提供 `--storage binary`（仅保留内部 benchmark 用途）。

**材料（可核对）**：
- 修复与回归：`crates/storage/src/binary_storage.rs`（四条 DDL 改报错）；
  `crates/storage/tests/binary_storage_ddl_honesty_5601.rs`（5 passed）；
  `crates/mysql-server/tests/binary_storage_ddl_honesty_5601.rs`（2 passed，走真
  wire + 真 binary 后端）。
- 取证探针：`audit/v4.1.0/binmode_probe.py`（修复前 `false_success_count=3`、
  `create_database_rc=0`；修复后 `0` 与 `1`）、`audit/v4.1.0/binmode_lock.py`
  （锁值已从「缺陷存在」翻到「缺陷已修」，缺陷回归即报红）。
- 另一条线的独立复测（PR #5615 内容、归档 run `20261106-readback-001`）：
  `binmode-judge` 读 002 的 stdout JSON ⇒ `false_success_count=0`，与本线一致。
- 既有设计证据：`crates/storage/src/binary_storage.rs` 顶部说明 binary 是
  「读预生成 .bin 文件」的后端（`--storage binary` 的 CLI 语义见
  `crates/mysql-server/src/lib.rs` 的 `storage` 参数说明：operator 需先用
  `tools/tbl2bin` 生成 .bin）。

**判据**：请给出「选 A / B / C」并说明理由与**必须同时做的事**（例如 A 需要补
哪段文档、是否需要在 CLI 上直接拒绝该模式与 DDL 的组合）。
材料不足（尤其是选项 B 的实现代价无法从材料判断）⇒ 输出 INCOMPLETE。
