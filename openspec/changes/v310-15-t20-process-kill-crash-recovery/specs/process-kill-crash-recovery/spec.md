## ADDED Requirements

### Requirement: 子进程管理框架
系统 SHALL 提供 `ProcessKillTestHarness` 辅助结构体，管理 sqlrustgo server 子进程的生命周期：启动、kill -9、重启、清理。

#### Scenario: 启动 server 子进程
- **WHEN** 调用 `harness.start_server(data_dir)` 
- **THEN** 返回 `Child` 句柄，server 监听在指定端口
- **THEN** 可通过 MySQL client 连接并执行 SQL

#### Scenario: kill -9 终止 server
- **WHEN** 调用 `harness.kill_server(child)`
- **THEN** 进程被 SIGKILL 终止，exit code 不为 0
- **THEN** WAL 文件在磁盘上保持上次 fsync 后的状态

### Requirement: BEGIN→INSERT→UPDATE→(kill -9)→重启→验证
系统 SHALL 验证在活跃事务中 kill -9 后重启，数据处于一致状态（事务要么完全提交，要么完全回滚）。

#### Scenario: 未提交事务在 crash 后回滚
- **WHEN** 执行 `BEGIN; INSERT INTO t VALUES (1, 'a'); UPDATE t SET name='b' WHERE id=1;` 后 kill -9 server
- **WHEN** 重启 server，执行 `SELECT * FROM t`
- **THEN** 结果 SHALL 符合事务原子性语义
- **THEN** WAL recovery 报告 SHALL 至少包含一条未完成事务记录

#### Scenario: 已提交事务在 crash 后持久化
- **WHEN** 执行 `BEGIN; INSERT INTO t VALUES (2, 'c'); COMMIT;` 后 kill -9 server
- **WHEN** 重启 server
- **THEN** `SELECT * FROM t` SHALL 包含已提交的数据

### Requirement: BEGIN→DELETE→(kill -9)→重启→验证
系统 SHALL 验证 DELETE 操作在 kill -9 后的恢复正确性。

#### Scenario: DELETE 在 crash 后回滚
- **WHEN** 预先插入数据 `INSERT INTO t VALUES (3, 'x')`
- **WHEN** 执行 `BEGIN; DELETE FROM t WHERE id=3;` 后 kill -9 server
- **WHEN** 重启 server
- **THEN** id=3 的数据 SHALL 仍然存在（未提交的 DELETE 被回滚）

#### Scenario: 已提交 DELETE 在 crash 后生效
- **WHEN** 预先插入数据 `INSERT INTO t VALUES (4, 'y'); COMMIT;`
- **WHEN** 执行 `BEGIN; DELETE FROM t WHERE id=4; COMMIT;` 后 kill -9 server
- **WHEN** 重启 server
- **THEN** id=4 的数据 SHALL 已被删除

### Requirement: WAL recovery 验证
系统 SHALL 记录和报告 WAL recovery 的统计信息，包括已提交事务数、未完成事务数、回滚事务数。

#### Scenario: WAL recovery 报告
- **WHEN** 在 kill -9 后重启 server
- **THEN** WAL recovery SHALL 产生 `RecoveryReport` 
- **THEN** 报告 SHALL 包含 `committed_txns`, `incomplete_txns`, `rolled_back_txns` 计数
