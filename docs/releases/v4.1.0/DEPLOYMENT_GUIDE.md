# SQLRustGo v4.1.0 — Deployment Guide

> **Date**: 2026-10-23
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Purpose**: Production deployment instructions for v4.1.0.

## 1. Deployment topology

```
┌─────────────────────────────────────────────────┐
│  Client Applications (MySQL wire protocol)      │
└──────────────────┬──────────────────────────────┘
                   │ TCP 3306
┌──────────────────▼──────────────────────────────┐
│  SQLRustGo Server                                │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐            │
│  │ Worker 1│ │ Worker 2│ │ Worker N│  (thread pool)
│  └─────────┘ └─────────┘ └─────────┘            │
│  ┌──────────────────────────────────────────┐    │
│  │ Storage Engine (FileStorage + MVCC + WAL) │    │
│  └──────────────────────────────────────────┘    │
│  ┌──────────────────────────────────────────┐    │
│  │ Data Directory (WAL, tables, catalog)    │    │
│  └──────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

## 2. Pre-deployment checklist

| Item | Requirement | Verified |
|---|---|---|
| Rust 1.75+ installed | `cargo --version` | ☐ |
| C compiler installed | `gcc --version` | ☐ |
| 10 GB+ disk free | `df -h` | ☐ |
| 16 GB+ RAM available | `free -g` | ☐ |
| Port 3306 available | `ss -tlnp \| grep 3306` | ☐ |
| Firewall configured | Allow 3306 from app servers only | ☐ |
| Backup plan in place | See §7 | ☐ |
| Monitoring configured | See §8 | ☐ |

## 3. Build the release binary

```bash
# On the build server
cd sqlrustgo
git checkout develop/v4.1.0
cargo build --release --all-features

# Verify the binary
./target/release/sqlrustgo --version
# Expected: sqlrustgo 4.1.0-draft
```

## 4. Deploy to production

### 4.1 Copy the binary

```bash
# From build server to production server
scp target/release/sqlrustgo production:/usr/local/bin/sqlrustgo
ssh production "chmod 755 /usr/local/bin/sqlrustgo"
```

### 4.2 Set up the data directory

```bash
# On the production server
sudo mkdir -p /var/lib/sqlrustgo
sudo chown sqlrustgo:sqlrustgo /var/lib/sqlrustgo
sudo chmod 750 /var/lib/sqlrustgo
```

### 4.3 Install the service

```bash
# Copy the systemd unit
scp sqlrustgo.service production:/etc/systemd/system/
ssh production "sudo systemctl daemon-reload"
ssh production "sudo systemctl enable sqlrustgo"
ssh production "sudo systemctl start sqlrustgo"
```

### 4.4 Verify

```bash
ssh production "sudo systemctl status sqlrustgo"
ssh production "mysql -h 127.0.0.1 -P 3306 -u root -e 'SELECT 1;'"
```

## 5. Firewall rules

```bash
# Allow only application servers on port 3306
sudo iptables -A INPUT -p tcp --dport 3306 -s 10.0.0.0/8 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 3306 -j DROP

# Or with ufw
sudo ufw allow from 10.0.0.0/8 to any port 3306 proto tcp
sudo ufw deny 3306
```

## 6. TLS configuration (recommended)

```bash
# Generate self-signed cert (for development)
openssl req -x509 -newkey rsa:4096 -days 365 -nodes \
    -keyout /etc/sqlrustgo/key.pem \
    -out /etc/sqlrustgo/cert.pem \
    -subj "/CN=sqlrustgo"

# Start with TLS
sqlrustgo serve --port 3306 --tls \
    --cert /etc/sqlrustgo/cert.pem \
    --key /etc/sqlrustgo/key.pem
```

> **Note**: For production, use a CA-signed certificate from your internal PKI.

## 7. Backup strategy

### 7.1 Online backup (WAL-aware)

```bash
# Stop accepting writes (drain connections)
mysql -h 127.0.0.1 -e "SET GLOBAL read_only = 1;"

# Copy data directory
tar -czf /backup/sqlrustgo-$(date +%Y%m%d).tar.gz -C /var/lib .

# Resume writes
mysql -h 127.0.0.1 -e "SET GLOBAL read_only = 0;"
```

### 7.2 Restore

```bash
# Stop the service
sudo systemctl stop sqlrustgo

# Remove old data
sudo rm -rf /var/lib/sqlrustgo/*

# Extract backup
sudo tar -xzf /backup/sqlrustgo-YYYYMMDD.tar.gz -C /var/lib/sqlrustgo

# Start the service
sudo systemctl start sqlrustgo
```

### 7.3 Backup schedule

| Frequency | Type | Retention |
|---|---|---|
| Daily | Full (tar) | 7 days |
| Weekly | Full + WAL archive | 4 weeks |
| Monthly | Full + offsite copy | 12 months |

## 8. Monitoring

### 8.1 Health check

```bash
# Simple liveness check
mysql -h 127.0.0.1 -P 3306 -u root -e "SELECT 1;" && echo "OK" || echo "FAIL"
```

### 8.2 Metrics to monitor

| Metric | Threshold | Action |
|---|---|---|
| CPU usage | >80% for 5 min | Alert |
| Memory (RSS) | >8 GB | Alert |
| Disk usage (data dir) | >80% | Alert |
| Connection count | >90% of max | Alert |
| WAL file size | >1 GB | Alert |
| Query latency (p99) | >100 ms | Alert |

### 8.3 Log monitoring

```bash
# Tail the service log
journalctl -u sqlrustgo -f

# Alert on errors
journalctl -u sqlrustgo --since "1 hour ago" | grep -i "error\|panic\|fatal"
```

## 9. Scaling considerations

v4.1.0 is a **single-node** database. It does not support:
- Horizontal scaling (sharding)
- Read replicas
- Multi-node clustering

For high-availability requirements:
- Run two instances with manual failover
- Use a load balancer for read traffic
- Schedule regular backups

## 10. Rollback procedure

```bash
# 1. Stop the current version
sudo systemctl stop sqlrustgo

# 2. Restore previous binary
sudo cp /usr/local/bin/sqlrustgo.bak /usr/local/bin/sqlrustgo

# 3. Start the previous version
sudo systemctl start sqlrustgo

# 4. Verify
mysql -h 127.0.0.1 -e "SELECT 1;"
```

## 11. Known limitations for production use

- **DRAFT stage**: v4.1.0 is not GA. Do not use for critical production workloads.
- **No authentication**: User authentication is not yet implemented.
- **Single-node only**: No built-in replication or clustering.
- **WP-C..G gaps**: 19 SQL features are deferred (see DEFECTS_AND_ISSUES.md).

## 12. Operational runbooks

### 12.1 Service restart

```bash
# Graceful restart
sudo systemctl restart sqlrustgo

# Verify
sudo systemctl status sqlrustgo
mysql -h 127.0.0.1 -e "SELECT 1;"
```

### 12.2 Emergency stop

```bash
# If the service is hung
sudo systemctl kill -s SIGTERM sqlrustgo
sudo systemctl start sqlrustgo
```

### 12.3 WAL recovery

If the server crashes and WAL files are incomplete:

```bash
# 1. Stop the service
sudo systemctl stop sqlrustgo

# 2. Remove incomplete WAL files (last modified < 1 minute)
find /var/lib/sqlrustgo/wal -name "*.wal" -mmin -1 -delete

# 3. Start the service (it will replay remaining WAL)
sudo systemctl start sqlrustgo

# 4. Verify data integrity
mysql -h 127.0.0.1 -e "SELECT COUNT(*) FROM information_schema.tables;"
```

### 12.4 Disk full emergency

```bash
# 1. Check what's using space
du -sh /var/lib/sqlrustgo/* | sort -rh

# 2. Remove old WAL files (older than 7 days)
find /var/lib/sqlrustgo/wal -name "*.wal" -mtime +7 -delete

# 3. Remove old checkpoints
find /var/lib/sqlrustgo -name "checkpoint_*.json" -mtime +7 -delete

# 4. Verify service is still running
sudo systemctl status sqlrustgo
```

### 12.5 Connection pool exhaustion

```bash
# 1. Check current connections
mysql -h 127.0.0.1 -e "SHOW STATUS LIKE 'Threads_connected';"

# 2. Kill idle connections (older than 30 min)
mysql -h 127.0.0.1 -e "
  SELECT id FROM information_schema.processlist
  WHERE command = 'Sleep' AND time > 1800;
"

# 3. Restart if needed
sudo systemctl restart sqlrustgo
```

## 13. Capacity planning

| Workload | Min CPU | Min RAM | Min Disk | Notes |
|---|---|---|---|---|
| Development | 4 cores | 4 GB | 10 GB | Single developer |
| Staging | 8 cores | 8 GB | 50 GB | CI/CD + load testing |
| Production (small) | 16 cores | 16 GB | 100 GB | <100 concurrent clients |
| Production (medium) | 32 cores | 32 GB | 500 GB | <500 concurrent clients |
| Production (large) | 64 cores | 64 GB | 1 TB | <1000 concurrent clients |

> **Note**: These are estimates based on the 8h SOAK data
> (`evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md`). Actual requirements
> depend on query mix, data size, and concurrency level.

## 14. Deployment automation

For automated deployments, use the following script:

```bash
#!/usr/bin/env bash
# deploy.sh — Automated SQLRustGo deployment
set -euo pipefail

VERSION="4.1.0"
REMOTE="production"
BINARY="target/release/sqlrustgo"

# 1. Build
cargo build --release --all-features

# 2. Stop service on remote
ssh "$REMOTE" "sudo systemctl stop sqlrustgo"

# 3. Backup old binary
ssh "$REMOTE" "sudo cp /usr/local/bin/sqlrustgo /usr/local/bin/sqlrustgo.bak.$(date +%Y%m%d)"

# 4. Deploy new binary
scp "$BINARY" "$REMOTE:/usr/local/bin/sqlrustgo"
ssh "$REMOTE" "sudo chmod 755 /usr/local/bin/sqlrustgo"

# 5. Start service
ssh "$REMOTE" "sudo systemctl start sqlrustgo"

# 6. Verify
ssh "$REMOTE" "mysql -h 127.0.0.1 -e 'SELECT 1;'"
echo "Deployment complete: v$VERSION"
```

## 15.1 Backup verification

After each backup, verify integrity:

```bash
# Verify tar archive
tar -tzf /backup/sqlrustgo-YYYYMMDD.tar.gz | head -20

# Verify WAL replay
mysql -h 127.0.0.1 -e "
  SELECT COUNT(*) FROM information_schema.tables
  WHERE table_schema NOT IN ('mysql', 'information_schema');
"
```

### 15.2 Offsite backup

For offsite copies, use encrypted archives:

```bash
# Encrypt and upload
tar -czf /tmp/backup.tar.gz -C /var/lib .
openssl enc -aes-256-cbc -pbkdf2 -in /tmp/backup.tar.gz \
    -out /backup/encrypted.tar.gz -pass file:/etc/sqlrustgo/.backup_pass
rsync -az /backup/encrypted.tar.gz offsite:/backups/
rm /tmp/backup.tar.gz
```

## 15.3 Backup testing

Test restore quarterly:

```bash
# 1. Create a test data directory
mkdir -p /tmp/restore-test
tar -xzf /backup/sqlrustgo-YYYYMMDD.tar.gz -C /tmp/restore-test

# 2. Start a test instance
sqlrustgo serve --port 3307 --data-dir /tmp/restore-test

# 3. Verify data
mysql -h 127.0.0.1 -P 3307 -e "SELECT COUNT(*) FROM users;"

# 4. Clean up
mysql -h 127.0.0.1 -P 3307 -e "SHUTDOWN;"
rm -rf /tmp/restore-test
```

## 16. Security hardening

### 16.1 Network security

- **Firewall**: Only allow connections from known application server IP ranges
- **TLS**: Always enable TLS in production (`--tls`)
- **No public exposure**: Port 3306 should never be exposed to the internet
- **Rate limiting**: Use a reverse proxy (nginx, haproxy) for rate limiting

### 16.2 File permissions

```bash
# Data directory: only service user can read/write
sudo chown -R sqlrustgo:sqlrustgo /var/lib/sqlrustgo
sudo chmod 750 /var/lib/sqlrustgo

# Binary: no world-write
sudo chmod 755 /usr/local/bin/sqlrustgo

# TLS certs: only service user can read key
sudo chmod 640 /etc/sqlrustgo/key.pem
sudo chmod 644 /etc/sqlrustgo/cert.pem
sudo chown sqlrustgo:sqlrustgo /etc/sqlrustgo/key.pem
```

### 16.3 Log security

- Do not log sensitive data (passwords, tokens, PII)
- Rotate logs daily to prevent disk fill
- Encrypt log archives for offsite storage

### 16.4 Update policy

- Patch within 7 days of CVE disclosure
- Test patches in staging before production
- Maintain a rollback plan for every update

## 17. Disaster recovery

### 17.1 RPO and RTO targets

| Metric | Target |
|---|---|
| RPO (Recovery Point Objective) | < 1 hour (daily backup + WAL) |
| RTO (Recovery Time Objective) | < 4 hours (restore + verification) |

### 17.2 DR procedure

1. **Declare disaster** — confirm data corruption or service failure
2. **Isolate affected node** — stop the service, preserve WAL
3. **Restore from latest backup** — tar extraction + WAL replay
4. **Verify data integrity** — run consistency checks
5. **Resume service** — start the service, verify connections
6. **Post-incident review** — document root cause, update runbooks

### 17.3 DR test schedule

| Frequency | Test type |
|---|---|
| Quarterly | Full restore test in staging |
| Semi-annually | DR drill with timeline measurement |
| Annually | Full DR simulation with failover |

## 18. Compliance and auditing

### 18.1 Audit trail

The server maintains:
- **WAL entries** — every write is logged in the WAL
- **Catalog changes** — schema modifications are recorded
- **Access logs** — connection and query logs (when enabled)

### 18.2 Compliance notes

- v4.1.0 does not currently meet SOC 2, HIPAA, or PCI-DSS requirements
- For regulated workloads, use a compliant database until v4.1.0 reaches GA
- Audit log retention: minimum 1 year (configurable)

## 19. Performance tuning

### 19.1 Key tuning parameters

| Parameter | Default | Tuning guidance |
|---|---|---|
| `--workers` | 4 | Set to CPU core count for OLTP workloads |
| `--max-connections` | 100 | Increase for high-concurrency apps |
| `--data-dir` | ./data | Use SSD-backed storage |
| `--log-level` | info | Use `warn` in production to reduce I/O |

### 19.2 Query optimization tips

- Use primary keys for point lookups (19× faster after #5168 fix)
- Batch INSERTs for bulk loads
- Avoid `SELECT *` — specify columns
- Use `EXPLAIN` to check query plans (limited support in v4.1.0)

### 19.3 Monitoring performance

```bash
# Run a quick benchmark
cargo run --all-features -- bench --duration 60

# Check latency distribution
mysql -h 127.0.0.1 -e "
  SELECT AVG(time), MAX(time), MIN(time)
  FROM performance_schema.events_statements;
"
```

### 19.4 WAL tuning

| Parameter | Default | Guidance |
|---|---|---|
| WAL flush interval | 100ms | Reduce for durability, increase for throughput |
| WAL buffer size | 64MB | Increase for write-heavy workloads |
| Checkpoint interval | 300s | Reduce for faster recovery, increase for less I/O |

### 19.5 Connection pool tuning

For high-concurrency workloads:
- Set `--workers` to CPU core count
- Set `--max-connections` to 2× expected peak concurrent clients
- Monitor connection pool utilization via `SHOW STATUS`
- Use application-level connection pooling (e.g., HikariCP, Go's database/sql)

## 20. References

- `docs/releases/v4.1.0/INSTALL.md` — installation instructions
- `docs/releases/v4.1.0/QUICK_START.md` — quick start
- `docs/releases/v4.1.0/MIGRATION_GUIDE.md` — migration from other versions
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` — evaluation results
- `docs/governance/GATE_CONDITIONS.md` — gate conditions
- `docs/releases/v4.1.0/evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` — capacity data
