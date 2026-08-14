# Spec — V312-56C: Transaction/Crash Recovery Teaching

## Overview

补齐用于教学的事务和恢复实验，形成可验证的闭环。

## Specification

### Transaction Teaching Experiments

#### BEGIN/COMMIT/ROLLBACK

| Step | SQL | Expected |
|---|---|---|
| 1 | BEGIN | Transaction started |
| 2 | INSERT INTO t VALUES (1) | 1 row affected |
| 3 | SELECT * FROM t | 1 row |
| 4 | ROLLBACK | Transaction rolled back |
| 5 | SELECT * FROM t | 0 rows |

#### SAVEPOINT

| Step | SQL | Expected |
|---|---|---|
| 1 | BEGIN | Transaction started |
| 2 | INSERT INTO t VALUES (1) | 1 row affected |
| 3 | SAVEPOINT sp1 | Savepoint created |
| 4 | INSERT INTO t VALUES (2) | 1 row affected |
| 5 | ROLLBACK TO SAVEPOINT sp1 | Rolled back to sp1 |
| 6 | SELECT COUNT(*) FROM t | 1 row |

### Crash/Recovery Fixtures

#### kill -9 Scenario

1. Insert N rows, commit
2. Begin transaction, insert M rows (uncommitted)
3. kill -9 server
4. Restart server
5. Verify: committed N rows, uncommitted M rows rolled back

#### WAL Replay

1. Insert rows, commit (WAL written)
2. Simulate crash before checkpoint
3. Restart server
4. Verify WAL replay recovers all committed data

### Backup/Restore Verification

After backup/restore, verify:
- GMP documents: row count + SHA256 hash
- embeddings: count + hash
- graph edges: count
- audit chain: previous_hash continuity

## Boundaries

- 所有实验命令必须本地可复跑
- 失败不得被文档描述为 PASS
