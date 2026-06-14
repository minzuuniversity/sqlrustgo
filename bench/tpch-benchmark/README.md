# TPC-H Benchmark (Multi-Engine, Multi-Scale)

> **Status**: 2026-06-13 — SF=0.001 + SF=0.01 baselines captured
> **Reference engine**: MySQL/MariaDB (ground truth row counts)

---

## 1. Layout

```
bench/tpch-benchmark/
├── README.md              # this file
├── data/                  # TPC-H .tbl files (3 scales)
│   ├── sf0.001_data/      # 501 lineitem rows
│   ├── sf0.01_data/       # 60K lineitem rows
│   └── (sf1_data/ optional, 6M rows)
├── baseline/              # Captured ground-truth row counts per engine
│   └── baseline_sf0.01.json
├── scripts/
│   ├── run_tpch_benchmark.py   # Multi-engine benchmark runner
│   ├── capture_baseline.sh     # One-shot baseline capture
│   └── sync_data.sh            # scp data + baseline to remote servers
└── reports/               # Output JSON from each run
```

## 2. Quick Start

### Local

```bash
# SF=0.01 baseline (use MySQL/MariaDB row counts as reference)
python3 scripts/run_tpch_benchmark.py \
    --sf sf0.01 \
    --engines sqlite,mysql \
    --capture \
    --queries-dir /Users/liying/workspace/dev/yinglichina163/sqlrustgo/queries \
    --data-root /Users/liying/workspace/dev/yinglichina163/sqlrustgo/bench/tpch-benchmark/data

# Run benchmark + compare to baseline
python3 scripts/run_tpch_benchmark.py \
    --sf sf0.01 \
    --engines sqlite,mysql,postgresql,sqlrustgo \
    --baseline baseline/baseline_sf0.01.json
```

### Sync to remote server (Z6G4 / Z440)

```bash
bash scripts/sync_data.sh push
bash scripts/sync_data.sh status
```

---

## 3. Engines Supported

| Engine    | Connection              | Notes                          |
|-----------|-------------------------|--------------------------------|
| SQLite    | `sqlite3` CLI           | local file DB                  |
| MySQL     | `mysql` CLI             | MariaDB on 127.0.0.1:3306      |
| PG        | `psql` CLI              | socket `/tmp`, port 5432       |
| SQLRustGo | `mysql` CLI → server    | starts `sqlrustgo-mysql-server serve --port 3307` |

## 4. Known Limitations (macOS MariaDB 12.3 client)

The bundled MariaDB client has a bug where `USE <db>` in `-e` context
re-authenticates as `''@'localhost'` and loses all privileges.
**Workaround**: pipe SQL via stdin instead of `-e`. See `load_mysql()`
and `run_query_mysql()` in `run_tpch_benchmark.py`.

Linux: standard `-e` works fine.

## 5. Baseline Format

```json
{
  "sf": "sf0.01",
  "engines": {
    "mysql": {"queries": {"Q01": {"rows": 4, "duration_ms": 60.0}, ...}},
    "sqlite": {...},
    "sqlrustgo": {...}
  }
}
```

`Q07`/`Q08`/`Q09` often fail on SQLite due to vendor-specific subquery
syntax — this is **expected** and not a bug in SQLRustGo.

## 6. Sync Strategy

`sync_data.sh push` packages `data/sf0.001_data/`, `data/sf0.01_data/`,
and `baseline/baseline_sf0.01.json` to `/tmp/tpch_benchmark_data/` on
each server. Use `rsync` (preferred) or scp+tar fallback.

Servers: 192.168.0.252 (Z6G4) + 192.168.0.250 (Z440).

## 7. Current State (2026-06-13)

| Engine    | SF=0.01 Result | Notes |
|-----------|----------------|-------|
| MySQL/MariaDB | 22/22 ✅ | Reference baseline |
| SQLRustGo (wire) | 22/22 ✅ | 100% row match with MySQL |
| SQLite | 19/22 | Q7-Q9 fail (SQLite vendor limit) |
| PG | 19/22 | Same as SQLite (TPCH_SF01 DB) |
