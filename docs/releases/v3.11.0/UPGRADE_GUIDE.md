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
