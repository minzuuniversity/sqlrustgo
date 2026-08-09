# SQLRustGo v3.11.0 升级指南

> **说明**: 本中文主文用于当前审阅；英文原文保留在附录。升级指南描述推荐流程，不等同升级/回滚测试已经全部通过。

## 1. 升级前检查

- 备份当前数据库文件、WAL、配置和 release metadata。
- 记录当前版本、commit、数据目录 hash 和关键表 row count。
- 确认目标部署场景是否属于受控、可回滚、SQL 范围明确的简单生产场景。

## 2. 升级流程

1. 停止写入或切换到维护窗口。
2. 完成完整备份并进行一次 restore rehearsal。
3. 部署 v3.11.0 binary/config。
4. 运行 schema/data consistency checks。
5. 运行核心业务 SQL regression、TPC-H 子集或实际业务 SQL checksum。
6. 观察 SOAK/health metrics 后再扩大流量。

## 3. 回滚要求

- 回滚必须有明确的数据目录快照和 WAL 策略。
- 回滚后必须验证 row count、hash、关键查询结果和权限策略。
- v3.12 应把 upgrade/downgrade verification 纳入正式 gate。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# v3.10.0 → v3.11.0 Upgrade Guide

## Breaking Changes

### Removed Extension Crates

The following extension crates have been removed:
- `agentsql` — Removed, use built-in SQL execution
- `distributed` — Distributed features deprecated

### Configuration Changes

- Default port changed from `3306` to `3307` for `sqlrustgo-cli serve`

## New Features

### TPC-H SF=1 Baseline

⚠️ **PENDING**: TPC-H 22/22 queries 验证待 fixture 生成后执行。`tpch_sf1_22_in_process_regression` 测试标记为 `#[ignore]`，依赖 `/tmp/tpch-sf1` fixture 数据。

运行方式（待 fixture 准备）:
```bash
bash scripts/tpch/setup_sf1.sh /tmp/tpch-sf1
bash scripts/tpch/run_sf1.sh
```

### GIS Support

New `WITHIN` operator for spatial queries:
```sql
SELECT * FROM regions WHERE geom WITHIN boundary;
```

### ALTER TABLE Improvements

Full RENAME and MODIFY column support:
```sql
ALTER TABLE t RENAME COLUMN old_name TO new_name;
ALTER TABLE t MODIFY col_name VARCHAR(255);
```

### Compression Support

Table compression with LZ4 or zstd:
```sql
CREATE TABLE t (...) COMPRESSION=lz4;
```

## Migration Steps

1. **Backup your data** before upgrading
2. **Stop v3.10.0 server**
3. **Install v3.11.0 binary**
4. **Start v3.11.0 server** — catalog migration is automatic
5. **Verify data integrity**:
```sql
SELECT COUNT(*) FROM your_tables;
```

## Compatibility Notes

- Wire protocol: v3.11.0 is compatible with v3.10.0 clients
- Storage format: v3.11.0 can read v3.10.0 data files
- SQL dialect: v3.11.0 adds new syntax, existing queries work unchanged
