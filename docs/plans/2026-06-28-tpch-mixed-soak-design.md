# TPC-H Mixed Workload SOAK Test Design

> **Date**: 2026-06-28
> **Author**: claude-macmini
> **Status**: APPROVED
> **Based on**: brainstorming with user

---

## 1. Goal

Implement a realistic TPC-H SOAK test that combines **dynamic CRUD operations** (INSERT/UPDATE/DELETE) with **TPC-H Q1-Q22 queries** on SF=0.1 dataset, to surface real-world issues that pure read-only SOAK cannot detect:

- WAL unbounded growth (only triggered by writes)
- MVCC visibility bugs (read-write conflicts)
- Lock contention / deadlocks
- Memory leaks under write pressure
- Thread leaks under mixed workload

---

## 2. Design Decisions

| Dimension | Decision |
|-----------|----------|
| CRUD scope | INSERT + UPDATE + DELETE randomly mixed |
| Read/write ratio | **80% queries / 20% CRUD** |
| Target tables | orders, lineitem, customer, partsupp, part |
| Data maintenance | **Cyclic replenishment** — DELETE triggers automatic INSERT to maintain stable table size |
| Primary key generation | **Pre-allocated ID pool per thread** — each worker reserves ID ranges, no locking needed |
| Concurrency | **Dynamic 4-32 random fluctuation** — simulates real traffic patterns (day/night peaks) |
| Monitoring | Enhanced: CRUD QPS, read-write conflicts, lock wait time |

---

## 3. Architecture

### 3.1 Components

```
tpch_soak_driver.py (extended)
├── Query Layer: 22 TPC-H query files (Q1-Q22)
├── CRUD Layer: 5 tables × CRUD templates
│   ├── orders:   INSERT(new), UPDATE(status), DELETE(old)
│   ├── lineitem: INSERT(new), UPDATE(qty), DELETE(old)
│   ├── customer: INSERT(new), UPDATE(acctbal), DELETE(inactive)
│   ├── partsupp: INSERT(new), UPDATE(supplycost), DELETE(orphan)
│   └── part:     INSERT(new), UPDATE(retailprice), DELETE(obsolete)
├── Worker Pool: Dynamic 4-32 workers
│   ├── Each worker has independent ID pool per table
│   ├── Each round: 80% query / 20% CRUD decision
│   └── Monitors: RSS, FD, thread count, WAL size, CRUD QPS, conflicts
└── Replenishment Thread: Monitors row counts, auto-replenishes on DELETE
```

### 3.2 ID Pool Strategy

```python
class IdPool:
    def __init__(self, thread_id, table_name, base_id, block_size=1000):
        self.base = base_id + thread_id * block_size * 100
        self.current = self.base
        self.block_size = block_size
    
    def next(self):
        val = self.current
        self.current += 1
        if self.current >= self.base + self.block_size:
            # Refresh from DB (SELECT MAX)
            self.current = self._fetch_max() + 1
        return val
```

### 3.3 Data Replenishment

```python
MIN_ROWS = {
    'orders': 10000,
    'lineitem': 50000,
    'customer': 1000,
    'partsupp': 50000,
    'part': 1500,
}

def replenishment_loop():
    while not stop:
        time.sleep(60)  # Check every minute
        for tbl, min_rows in MIN_ROWS.items():
            current = db.select(f"SELECT COUNT(*) FROM {tbl}")
            if current < min_rows:
                # Bulk insert to restore row count
                bulk_insert(tbl, min_rows - current)
```

### 3.4 Dynamic Concurrency

```python
def adjust_concurrency():
    while not stop:
        target = random.randint(4, 32)
        current = len(workers)
        if target > current:
            for _ in range(target - current):
                workers.append(spawn_worker())
        elif target < current:
            for _ in range(current - target):
                workers.pop().stop()
        time.sleep(random.randint(30, 120))  # Re-evaluate every 30-120s
```

---

## 4. CRUD Templates

### 4.1 Orders

```sql
-- INSERT
INSERT INTO orders VALUES ({orderkey}, {custkey}, 'O', {totalprice}, '{date}', 
    'speech', 'O', {priority}, '{clerk}', {shippriority});

-- UPDATE (status change)
UPDATE orders SET o_orderstatus = 'F' WHERE o_orderkey = {key} AND o_orderstatus = 'O';

-- DELETE (old orders)
DELETE FROM orders WHERE o_orderdate < '2015-01-01' AND o_orderstatus IN ('C', 'X');
```

### 4.2 Lineitem

```sql
-- INSERT
INSERT INTO lineitem VALUES ({orderkey}, {partkey}, {suppkey}, {linenum}, {qty}, 
    {extended_price}, {discount}, {tax}, 'R', '{date}', '{shipdate}', 
    '{commit_date}', '{receipt_date}', 'DELIVERED', 'LOCAL');

-- UPDATE (quantity adjust)
UPDATE lineitem SET l_quantity = l_quantity + 1 
    WHERE l_orderkey = {key} AND l_linenumber = {linenum};

-- DELETE (old lineitems for delivered orders)
DELETE FROM lineitem WHERE l_orderkey IN (
    SELECT o_orderkey FROM orders WHERE o_orderdate < '2015-01-01'
);
```

### 4.3 Customer

```sql
-- INSERT
INSERT INTO customer VALUES ({custkey}, '{name}', '{address}', {nationkey}, 
    '{phone}', {acctbal}, '{mktsegment}', '{comment}');

-- UPDATE (balance adjust)
UPDATE customer SET c_acctbal = c_acctbal + {delta} WHERE c_custkey = {key};

-- DELETE (inactive, low balance)
DELETE FROM customer WHERE c_acctbal < 0 AND c_nationkey = {nationkey};
```

### 4.4 Part

```sql
-- INSERT
INSERT INTO part VALUES ({partkey}, '{name}', '{mfgr}', '{brand}', 
    '{type}', {size}, '{container}', {retailprice}, '{comment}');

-- UPDATE (price adjust)
UPDATE part SET p_retailprice = p_retailprice * 1.01 WHERE p_partkey = {key};

-- DELETE (obsolete)
DELETE FROM part WHERE p_partkey = {key} AND p_retailprice < {threshold};
```

### 4.5 Partsupp

```sql
-- INSERT
INSERT INTO partsupp VALUES ({partkey}, {suppkey}, {availqty}, {supplycost}, '{comment}');

-- UPDATE (supply cost adjust)
UPDATE partsupp SET ps_supplycost = ps_supplycost * 1.05 
    WHERE ps_partkey = {key} AND ps_suppkey = {suppkey};

-- DELETE (orphan)
DELETE FROM partsupp WHERE ps_partkey = {key} AND ps_availqty = 0;
```

---

## 5. Metrics

### 5.1 SoakReport.json Extension

```json
{
  "level": "mixed_80_20",
  "read_write_ratio": "80:20",
  "concurrency": { "min": 4, "max": 32 },
  
  "queries_executed": 150000,
  "errors": 0,
  "p50_latency_ms": 12.5,
  "p99_latency_ms": 85.3,
  
  "crud": {
    "insert_qps": 8.2,
    "update_qps": 6.1,
    "delete_qps": 4.3,
    "total_crud_operations": 185000
  },
  
  "read_write_conflicts": 12,
  "lock_wait_time_avg_ms": 0.45,
  
  "memory_baseline_bytes": 524288000,
  "memory_final_bytes": 558036480,
  "memory_growth_pct": 6.44,
  "alert_triggered": false,
  
  "queries_per_second": 125.5
}
```

---

## 6. Error Handling

| Failure Mode | Handling |
|--------------|----------|
| INSERT duplicate key | Catch, refresh ID pool, retry |
| UPDATE affected 0 rows | Log warning, continue |
| DELETE affected 0 rows | Log warning, continue |
| Connection lost | Reconnect with backoff, resume |
| Server crash | Restart server, resume from checkpoint |
| Row count below threshold | Trigger replenishment immediately |

---

## 7. References

- TPC-H V3.0 Specification (tpc.org)
- CH-benCHmark (TUM) — mixed OLTP+OLAP workload
- Amazon Redshift Synthetic Mixed Benchmark
- `scripts/soak/tpch_soak_driver.py` — existing pure-read SOAK driver
- `scripts/stability/raw_mysql_soak.py` — existing CRUD pattern

---

## 8. Open Questions

None. All decisions have been validated with user.