# TPC-H Benchmark on Server 250 (Z440)

> Status: 2026-06-14 — environment prepared, blocked by OOM during SF=0.01 LOAD.
> This document describes the complete deployment procedure and known issues.

## Server 250 Specs

| Resource | Value |
|----------|-------|
| Hostname | gaoyuan |
| IP | 192.168.0.250 |
| OS | Ubuntu 22.04.4 LTS (kernel 6.8.0-110) |
| CPU | Intel Xeon E5-2687W v4 × 2 (20 cores / 40 threads) |
| RAM | 94 GB DDR4 ECC |
| Disk | 916 GB (/dev/sdb2, 152 GB free) |
| Swap | 2 GB |

## SSH Access

- **Primary alias**: `ss4` (ai user, ed25519 key, passwordless)
- **Fallback**: ssh `liying@192.168.0.250` (password required)
- **Gitea**: http://192.168.0.250:3000 (also via :80, :8080)
- **Gitea password**: openclaw / details8848

## Pre-installed Software

| Tool | Version | Path |
|------|---------|------|
| MariaDB | 10.6.23 | /usr/bin/mysql, /usr/bin/mariadb |
| PostgreSQL | 16 (s6 container) | /var/run/postgresql |
| SQLite | 3.37.2 | /usr/bin/sqlite3 |
| Python | 3.10.12 | /usr/bin/python3 |
| Git | 2.34.1 | /usr/bin/git |
| rsync | 3.2.7 | /usr/bin/rsync |
| zstd | 1.4.8 | /usr/bin/zstd |
| Rust | 1.94.1 | /home/liying/.cargo/bin/cargo |
| sqlrustgo binary | 11.7 MB (release) | /home/liying/sqlrustgo/target/release/sqlrustgo-mysql-server |

**Missing tools** (need install):
- `psql` (PostgreSQL client) — not in PATH, not installed
- `dbgen` (TPC-H data generator) — not installed
- `socat` / `sshpass` — not needed for our workflow

## Directory Layout on 250

```
/home/liying/bench/tpch-benchmark/        # Owner: liying (mode 755, files 600)
├── data/
│   ├── sf0.001_data/                       # 8 .tbl files, ~2 MB
│   ├── sf0.01_data/                        # 8 .tbl files, ~8 MB (60K lineitem)
│   ├── sf0.1_data/                         # 8 .tbl files, ~103 MB (600K lineitem)
│   └── sf1_data/                           # 8 .tbl files, ~1.1 GB (6M lineitem)
├── scripts/
│   ├── run_tpch_benchmark.py               # mode 600 (liying only)
│   └── sync_data.sh                        # mode 600 (liying only)
├── queries/
│   └── q{1-22}.sql                         # mode 644 (world readable)
└── baseline/                               # empty (results to be written)
```

## Deployment Procedure (Proven)

### Step 1: Clone & Build sqlrustgo
```bash
ssh ai@ss4
git clone --depth=1 -b develop/v3.9.0 http://liying@192.168.0.250:3000/openclaw/sqlrustgo.git ~/sqlrustgo
export PATH=$HOME/.cargo/bin:$PATH
cd ~/sqlrustgo
cargo build --release -p sqlrustgo-mysql-server
# → 2m15s, produces 11.7 MB binary
```

### Step 2: Sync Data (one-time)
```bash
# On Mac, generate SF=0.01, SF=0.1, SF=1 via /Users/liying/tpch-tools/dbgen/dbgen
# Then rsync to 250:
for sf in sf0.01 sf0.1 sf1; do
  rsync -avz -e ssh bench/tpch-benchmark/data/${sf}_data/ \
    liying@192.168.0.250:/home/liying/bench/tpch-benchmark/data/${sf}_data/
done
```

### Step 3: Run MySQL Baseline
```bash
ssh ai@ss4
# DROP existing test DB
sudo mysql -e "DROP DATABASE IF EXISTS tpch_sf0_01; CREATE DATABASE tpch_sf0_01"
# Or for SF=1:
sudo mysql -e "DROP DATABASE IF EXISTS tpch_sf1; CREATE DATABASE tpch_sf1"

# Load data with trailing-pipe (TPC-H spec):
for tbl in region nation supplier customer part partsupp orders lineitem; do
  mysql -u liying -e "USE tpch_sf0_01;
    CREATE TABLE ${tbl} (...);
    LOAD DATA LOCAL INFILE '/home/liying/bench/tpch-benchmark/data/sf0.01_data/${tbl}.tbl'
    INTO TABLE ${tbl} FIELDS TERMINATED BY '|' LINES TERMINATED BY '|';"
done
```

### Step 4: Run SQLRustGo Wire Baseline
```bash
ssh ai@ss4
# Start sqlrustgo on port 3307 with ai-writable data_dir
mkdir -p /home/ai/sqlrustgo_sf01
cp /home/liying/bench/tpch-benchmark/data/sf0.01_data/*.tbl /home/ai/sqlrustgo_sf01/
/home/liying/sqlrustgo/target/release/sqlrustgo-mysql-server serve \
  --data-dir /home/ai/sqlrustgo_sf01 --port 3307 &
# DDL + LOAD
mysql -h 127.0.0.1 -P 3307 -u tester --local-infile=1 -e "..." (CREATE 8 tables)
# LOAD 7 small tables (instant), lineitem takes ~120s for 60K rows
# Run 22 queries via Python driver
```

## Known Issues & Workarounds

### Issue 1: macOS MariaDB Client (Resolved)
- macOS MariaDB 12.3 client has Unix socket auth bug with `-u tester`
- **Workaround**: use OS user `liying` with socket auth, no `-h/-P`
- On 250 (Linux), this is NOT an issue — use `tester` user freely

### Issue 2: SF=0.01 LOAD Triggers OOM (Known)
- SQLRustGo v3.9.0-rc7 LOAD for 60K lineitem uses **>78 GB RSS**
- Root cause: no streaming during LOAD; full table in memory
- **Workaround**: scale to SF=0.001 (501 rows, ~50 MB RSS, works)
- **Status**: Issue filed for v3.10; test SF=0.1/1 deferred

### Issue 3: Liying Process D-state Lockup
- LOAD 60K lineitem enters D state (disk sleep) writing WAL
- Even after kill -9, process stays in D until IO completes
- 78 GB RSS cannot be reclaimed until process exits
- **Workaround**: physical restart of 250, or wait 30-60 min for IO

### Issue 4: PG Client Missing on 250
- `psql` not installed in default Ubuntu
- No sudo available to ai user without password
- **Workaround**: use SQLRustGo wire protocol for all queries
  (or install postgresql-client via pip: `pip install psycopg2-binary`)

### Issue 5: SSH Banner Hangs When System Thrashing
- During OOM, sshd cannot fork() to handle new connections
- TCP port 22 accepts but no SSH protocol exchange
- **Workaround**: wait for OOM to clear or physical restart

## Baseline Coverage

| Scale | Data | MySQL | PG | SQLRustGo | Status |
|-------|------|:-----:|:--:|:---------:|--------|
| SF=0.001 (501 lineitem) | 2 MB | 22/22 ✅ | 19/22 ⚠️ | 22/22 ✅ | **DONE** (Mac local) |
| SF=0.01 (60K lineitem) | 8 MB | 22/22 ✅ | 19/22 ⚠️ | OOM | **DEFERRED** (server OOM bug) |
| SF=0.1 (600K lineitem) | 103 MB | TBD | TBD | TBD | pending sqlrustgo fix |
| SF=1 (6M lineitem) | 1.1 GB | TBD | TBD | TBD | pending sqlrustgo fix |

## Files Pushed to 250

| Path | Size | Owner | Mode |
|------|------|-------|------|
| data/sf0.001_data/*.tbl | 2 MB | liying | 644 |
| data/sf0.01_data/*.tbl | 8 MB | liying | 644 |
| data/sf0.1_data/*.tbl | 103 MB | liying | 644 |
| data/sf1_data/*.tbl | 1.1 GB | liying | 644 |
| queries/q{1-22}.sql | 8.7 KB | liying | 644 |
| scripts/run_tpch_benchmark.py | 28 KB | liying | **600** (locked) |
| sqlrustgo binary | 11.7 MB | liying | 755 |
| sqlrustgo source | 1.2 GB | liying | various |

## Recovery Procedure After 250 Crash

```bash
# 1. Check 250 is up
ping -c 1 192.168.0.250

# 2. Verify SSH responsive
ssh -o ConnectTimeout=5 ai@ss4 "uptime"

# 3. Check no hung sqlrustgo process
ssh ai@ss4 "ps aux | grep sqlrustgo | grep -v grep"

# 4. If D-state process: cannot kill, must wait or restart
# 5. If clean: re-run baseline from Step 3 above
```

## Reproducible Baseline Command (Post-Recovery)

```bash
# On Mac, run the 250 driver:
python3 /tmp/tpch250.py 3307 /tmp/250_sf0.001_result.json
# 22/22 queries against 250 SQLRustGo at port 3307
```

## Team Use Instructions

For team members wanting to use the TPC-H benchmark environment:

1. **Connect**: `ssh ss4` (uses ai user with public key)
2. **Run existing benchmarks**: see `bench/tpch-benchmark/SERVER250.md`
3. **Add new scale**: generate via dbgen, rsync to /home/liying/bench/tpch-benchmark/data/
4. **Modify schema**: edit `scripts/run_tpch_benchmark.py` (need sudo to /home/liying/)
5. **Compare results**: see `baseline/baseline_sf*.json` for ground truth
