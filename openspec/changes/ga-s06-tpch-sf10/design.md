## Overview

TPC-H SF=10 基准测试 (~10GB 数据)，验证生产级扩展性。

## Architecture

### Data Generation

```
SF=1: ~1GB, ~70MB per table (LINEITEM)
SF=10: ~10GB, ~700MB per table (LINEITEM)
```

### Test Process

```
1. Generate SF=10 data using dbgen
2. Load data into sqlrustgo
3. Run 22 queries with timeout (10min each)
4. Record RSS, execution time
5. Verify no OOM
```

## Implementation Details

### 1. `scripts/tpch/setup_sf10.sh`

```bash
#!/bin/bash
SF=${1:-10}
DATA_DIR="/tmp/tpch-sf${SF}"

# Generate SF=10 data
./dbgen -s $SF -f -d $DATA_DIR

# Expected output:
# customer.tbl ~175MB
# orders.tbl ~1.3GB
# lineitem.tbl ~6.8GB
```

### 2. `scripts/tpch/run_sf10.sh`

```bash
#!/bin/bash
SF=${1:-10}
TIMEOUT=600  # 10 min per query

for q in $(seq 1 22); do
    start=$(date +%s)
    timeout $TIMEOUT ./tpch_runner -q $q --sf $SF
    end=$(date +%s)
    echo "Q$q: $((end - start))s"
done
```

## Dependencies

- `dbgen` (TPC-H data generator)
- `tpch_runner` (existing test runner)
- Sufficient disk space (~15GB for SF=10)
- Sufficient memory (~16GB RAM recommended)
