# Slow Query Log

## Overview

Log queries that exceed a configurable threshold to a file in JSON format.

## Usage

```bash
sqlrustgo-cli serve --slow-query-time=1.0 --slow-query-log=/var/log/sqlrustgo-slow.log
```

## Log Format

```json
{
  "timestamp": "2026-07-18T12:00:00Z",
  "query_time": 1.234,
  "rows_examined": 100000,
  "sql": "SELECT * FROM orders WHERE customer_id = 12345"
}
```

## Configuration

- `--slow-query-time` — Threshold in seconds (default: 1.0)
- `--slow-query-log` — Path to log file (default: /tmp/sqlrustgo-slow.log)
