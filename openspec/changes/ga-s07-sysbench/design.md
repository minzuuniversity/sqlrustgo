## Overview

Sysbench OLTP 混合负载压测脚本。

## Architecture

```
1. Prepare: Create OLTP schema (sbtest1)
2. Load: Insert ~100k rows
3. Run: oltp_read_write for 2 hours
4. Report: TPS, latency, deadlocks
```

## Dependencies

- sysbench (brew install sysbench or apt install sysbench)
- MySQL client
