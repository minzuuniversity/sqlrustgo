## Overview

Admin 命令增强。

## admin status

```bash
sqlrustgo-admin status --host 127.0.0.1 --port 3307
# Output:
# Innodb_rows_read: 1234567
# Threads_connected: 5
# Uptime: 3600s
```

## admin flush-logs

```bash
sqlrustgo-admin flush-logs
# Flushes all log files
```
