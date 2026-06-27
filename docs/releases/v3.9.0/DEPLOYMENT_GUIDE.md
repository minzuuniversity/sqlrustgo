# Deployment Guide — SQLRustGo v3.9.0

> Production deployment of SQLRustGo v3.9.0 with systemd, TLS, and
> observability.

## System requirements

| Component | Minimum | Recommended |
|---|---|---|
| CPU | 2 cores | 4+ cores |
| RAM | 2 GB | 8 GB |
| Disk | 1 GB | 100 GB SSD |
| OS | Linux 5.x+ | Ubuntu 22.04 LTS |
| Network | 100 Mbps | 1 Gbps |
| Filesystem | ext4 | xfs or ext4 |

## Pre-built binary install

```bash
# Download and verify
curl -L -o sqlrustgo.tar.gz https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz
curl -L -o sqlrustgo.tar.gz.sig https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz.sig
gpg --verify sqlrustgo.tar.gz.sig sqlrustgo.tar.gz

# Extract
tar xzf sqlrustgo.tar.gz
sudo install -m 0755 sqlrustgo /usr/local/bin/sqlrustgo

# Verify
sqlrustgo --version
```

## From source

```bash
git clone https://github.com/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout v3.9.0
cargo build --release
sudo install -m 0755 target/release/sqlrustgo /usr/local/bin/
```

## Data directory layout

```
/var/lib/sqlrustgo/
├── data/         # Heap pages
├── wal/          # Write-ahead log segments
├── snapshots/    # Checkpoint files
└── sqlrustgo.toml  # Optional config
```

Initialize:

```bash
sudo mkdir -p /var/lib/sqlrustgo/{data,wal,snapshots}
sudo chown -R sqlrustgo:sqlrustgo /var/lib/sqlrustgo
sudo chmod 0750 /var/lib/sqlrustgo
```

## systemd unit

`/etc/systemd/system/sqlrustgo.service`:

```ini
[Unit]
Description=SQLRustGo v3.9.0
Documentation=https://docs.sqlrustgo.example.com/v3.9.0/
After=network.target

[Service]
Type=simple
User=sqlrustgo
Group=sqlrustgo
WorkingDirectory=/var/lib/sqlrustgo
Environment="RUST_LOG=info"
Environment="SQLRUSTGO_DATA_DIR=/var/lib/sqlrustgo"
ExecStart=/usr/local/bin/sqlrustgo server \
  --port 5432 \
  --data-dir /var/lib/sqlrustgo \
  --max-connections 200 \
  --statement-cache-size 1024
Restart=on-failure
RestartSec=5s
LimitNOFILE=65535

# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/sqlrustgo
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable sqlrustgo
sudo systemctl start sqlrustgo
sudo systemctl status sqlrustgo
```

## TLS configuration

Generate self-signed cert (production: use Let's Encrypt or
internal CA):

```bash
sudo mkdir -p /etc/sqlrustgo/tls
sudo openssl req -x509 -nodes -days 365 \
  -newkey rsa:2048 \
  -keyout /etc/sqlrustgo/tls/server.key \
  -out /etc/sqlrustgo/tls/server.crt \
  -subj "/CN=sqlrustgo.example.com"
sudo chown -R sqlrustgo:sqlrustgo /etc/sqlrustgo
sudo chmod 0600 /etc/sqlrustgo/tls/server.key
```

Start with TLS:

```bash
sqlrustgo server \
  --port 5432 \
  --data-dir /var/lib/sqlrustgo \
  --tls-cert /etc/sqlrustgo/tls/server.crt \
  --tls-key /etc/sqlrustgo/tls/server.key
```

Connect:

```bash
psql "sslmode=require host=localhost port=5432"
```

## Observability

### Metrics (Prometheus)

SQLRustGo exposes Prometheus metrics on `/_/metrics`:

```
http://localhost:5432/_/metrics
```

Sample scrape config:

```yaml
scrape_configs:
  - job_name: 'sqlrustgo'
    static_configs:
      - targets: ['sqlrustgo.example.com:5432']
    metrics_path: '/_/metrics'
```

### Logs (structured JSON)

```bash
RUST_LOG=info,sqlrustgo_executor=debug \
  sqlrustgo server --log-format json
```

Forward to Loki/Elasticsearch with Promtail/Filebeat.

### Health check

```bash
curl -s http://localhost:5432/_/health
# {"status":"ok","uptime_secs":3600}
```

## Backup and restore

### Snapshot backup

```bash
# Online snapshot
sqlrustgo admin snapshot --output /backup/sqlrustgo-$(date +%F).snap

# Verify
sqlrustgo admin verify-snapshot /backup/sqlrustgo-2026-06-11.snap
```

### Restore

```bash
# Stop server
sudo systemctl stop sqlrustgo

# Restore
sqlrustgo admin restore --from /backup/sqlrustgo-2026-06-11.snap \
  --into /var/lib/sqlrustgo

# Start server
sudo systemctl start sqlrustgo
```

### WAL archive (PITR)

```bash
# Enable continuous WAL archiving
sqlrustgo server --wal-archive-dir /archive/wal --wal-archive-compress zstd
```

## Scaling

### Read replicas (planned, not in v3.9.0)

v3.9.0 is single-node. Read replicas are on the v3.10.0 roadmap.

### Connection pooling

Use PgBouncer in front of SQLRustGo:

```ini
# /etc/pgbouncer/pgbouncer.ini
[databases]
sqlrustgo = host=localhost port=5432 dbname=sqlrustgo
[pgbouncer]
listen_port = 6432
auth_type = scram-sha-256
pool_mode = transaction
max_client_conn = 1000
default_pool_size = 25
```

## Troubleshooting

### "Too many connections"

```bash
# Check current
ss -tan | grep :5432 | wc -l

# Increase in systemd unit
# Edit /etc/systemd/system/sqlrustgo.service
# Add: --max-connections 500
sudo systemctl daemon-reload
sudo systemctl restart sqlrustgo
```

### "WAL archive lag"

```bash
sqlrustgo admin wal-status
# Check archive_lag_seconds; if > 60, check disk + network
```

### Crash loop

```bash
sudo systemctl status sqlrustgo
sudo journalctl -u sqlrustgo -n 100 --no-pager
# Look for stack trace; usually IR or storage panic
```

## Production checklist

- [ ] systemd unit with `Restart=on-failure`
- [ ] TLS cert (Let's Encrypt or internal CA)
- [ ] Backup script running nightly (cron)
- [ ] WAL archive to remote storage
- [ ] Prometheus scrape + alerting on `sqlrustgo_up == 0`
- [ ] Log aggregation (Loki/ELK)
- [ ] PgBouncer in front for connection pooling
- [ ] Firewall: only 5432 exposed
- [ ] Runbook documented
- [ ] On-call rotation established

See [`EVALUATION_REPORT.md`](EVALUATION_REPORT.md) for benchmark
numbers and [`RELEASE_NOTES.md`](RELEASE_NOTES.md) for what changed.

## Docker Compose

Quick local development stack:

`docker-compose.yml`:

```yaml
version: "3.9"

services:
  sqlrustgo:
    image: openclaw/sqlrustgo:v3.9.0
    container_name: sqlrustgo
    ports:
      - "5432:5432"
    volumes:
      - sqlrustgo_data:/var/lib/sqlrustgo
    environment:
      RUST_LOG: info
      SQLRUSTGO_MAX_CONNECTIONS: 100
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:5432/_/health"]
      interval: 30s
      timeout: 5s
      retries: 3

  pgbouncer:
    image: bitnami/pgbouncer:1.22.0
    container_name: pgbouncer
    depends_on:
      sqlrustgo:
        condition: service_healthy
    ports:
      - "6432:6432"
    environment:
      PGBOUNCER_DATABASES: "sqlrustgo host=sqlrustgo port=5432 dbname=postgres"
      PGBOUNCER_AUTH_TYPE: scram-sha-256
      PGBOUNCER_POOL_MODE: transaction
      PGBOUNCER_MAX_CLIENT_CONN: 1000
      PGBOUNCER_DEFAULT_POOL_SIZE: 25
    restart: unless-stopped

  prometheus:
    image: prom/prometheus:v2.50.0
    container_name: prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
    restart: unless-stopped

  grafana:
    image: grafana/grafana:10.4.0
    container_name: grafana
    depends_on:
      - prometheus
    ports:
      - "3000:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-admin}
    volumes:
      - grafana_data:/var/lib/grafana
    restart: unless-stopped

volumes:
  sqlrustgo_data:
  grafana_data:
```

Bring up the stack:

```bash
docker compose up -d
docker compose ps
docker compose logs -f sqlrustgo
```

## Kubernetes

### StatefulSet (single-node)

`sqlrustgo-statefulset.yaml`:

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: sqlrustgo
  namespace: database
spec:
  serviceName: sqlrustgo
  replicas: 1
  selector:
    matchLabels:
      app: sqlrustgo
  template:
    metadata:
      labels:
        app: sqlrustgo
    spec:
      containers:
        - name: sqlrustgo
          image: openclaw/sqlrustgo:v3.9.0
          ports:
            - containerPort: 5432
          env:
            - name: RUST_LOG
              value: "info"
            - name: SQLRUSTGO_DATA_DIR
              value: /var/lib/sqlrustgo
            - name: SQLRUSTGO_MAX_CONNECTIONS
              value: "500"
          resources:
            requests:
              cpu: "1"
              memory: "4Gi"
            limits:
              cpu: "4"
              memory: "8Gi"
          livenessProbe:
            httpGet:
              path: /_/health
              port: 5432
            initialDelaySeconds: 30
            periodSeconds: 30
            failureThreshold: 3
          readinessProbe:
            httpGet:
              path: /_/health
              port: 5432
            initialDelaySeconds: 5
            periodSeconds: 10
          volumeMounts:
            - name: data
              mountPath: /var/lib/sqlrustgo
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: sqlrustgo-data
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: sqlrustgo-data
  namespace: database
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: ssd
  resources:
    requests:
      storage: 100Gi
---
apiVersion: v1
kind: Service
metadata:
  name: sqlrustgo
  namespace: database
spec:
  type: ClusterIP
  selector:
    app: sqlrustgo
  ports:
    - port: 5432
      targetPort: 5432
```

Apply:

```bash
kubectl create namespace database
kubectl apply -f sqlrustgo-statefulset.yaml
kubectl -n database get pods -w
```

### Horizontal Pod Autoscaler (HPA)

Note: SQLRustGo v3.9.0 is single-node. HPA is for the connection
pooler (PgBouncer) sidecar, not the engine itself.

### Backup CronJob

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: sqlrustgo-snapshot
  namespace: database
spec:
  schedule: "0 2 * * *"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: backup
              image: openclaw/sqlrustgo:v3.9.0
              command: ["/bin/sh", "-c"]
              args:
                - "sqlrustgo admin snapshot --output /backup/$(date +%F-%H%M).snap"
              volumeMounts:
                - name: backup
                  mountPath: /backup
          restartPolicy: OnFailure
          volumes:
            - name: backup
              persistentVolumeClaim:
                claimName: sqlrustgo-backup
```

## Capacity planning

### Storage

Estimate:

```
storage_bytes =
  raw_data_bytes * 1.5      # heap + indexes
  + wal_archive_growth      # ~1 GB / hour under load
  + snapshot_size           # 1x raw_data_bytes
```

For a 100 GB raw database: 150 GB heap + 1 GB/hour WAL +
100 GB snapshots = ~300 GB.

### Connections

PostgreSQL clients: 1-2 MB per connection. PgBouncer with
transaction-mode pooling: ~50 KB per pooled backend. Default
PgBouncer config (25 pool size) supports ~1000 client connections.

### Memory

SQLRustGo's RSS:

```
memory_mb =
  200  # base
  + 50 * max_connections  # per-connection buffer pool
  + cache_size  # statement cache + page cache
```

Default 200 connections = ~10 GB resident.

## Multi-host setup

For multi-host (engine + separate WAL archiver):

```ini
# /etc/sqlrustgo/sqlrustgo.toml (engine)
[server]
bind = "0.0.0.0:5432"
max_connections = 200

[storage]
data_dir = "/var/lib/sqlrustgo"
wal_archive_dir = "s3://my-bucket/sqlrustgo-wal/"
wal_archive_compress = "zstd"

[observability]
metrics_addr = "0.0.0.0:9100"
log_format = "json"
```

WAL archiver runs as a separate process:

```bash
sqlrustgo wal-archiver \
  --source /var/lib/sqlrustgo/wal \
  --dest s3://my-bucket/sqlrustgo-wal/ \
  --compress zstd
```

## Postmortem template

```markdown
# Postmortem: <incident title>

**Date**: YYYY-MM-DD
**Duration**: HH:MM
**Severity**: SEV-1 / SEV-2 / SEV-3
**On-call**: <name>

## Summary

<1-2 sentence summary of what happened>

## Impact

<who was affected, what failed, how many users>

## Timeline (UTC)

- HH:MM — alert fired
- HH:MM — on-call paged
- HH:MM — investigation started
- HH:MM — root cause identified
- HH:MM — mitigation applied
- HH:MM — full recovery

## Root cause

<technical explanation>

## Resolution

<what was changed to fix>

## Lessons learned

<what we learned, what could be improved>

## Action items

- [ ] <action 1> (owner, due date)
- [ ] <action 2> (owner, due date)
```

## Observability stack

### Grafana dashboard

JSON dashboard for SQLRustGo (import via `/_/metrics`):

```json
{
  "title": "SQLRustGo v3.9.0",
  "panels": [
    {
      "title": "QPS",
      "type": "graph",
      "targets": [
        { "expr": "rate(sqlrustgo_queries_total[1m])" }
      ]
    },
    {
      "title": "p99 query latency",
      "type": "graph",
      "targets": [
        { "expr": "histogram_quantile(0.99, rate(sqlrustgo_query_duration_seconds_bucket[5m]))" }
      ]
    },
    {
      "title": "Active connections",
      "type": "singlestat",
      "targets": [
        { "expr": "sqlrustgo_active_connections" }
      ]
    },
    {
      "title": "WAL flushes / sec",
      "type": "graph",
      "targets": [
        { "expr": "rate(sqlrustgo_wal_flushes_total[1m])" }
      ]
    },
    {
      "title": "Heap page reads",
      "type": "graph",
      "targets": [
        { "expr": "rate(sqlrustgo_heap_page_reads_total[1m])" }
      ]
    },
    {
      "title": "WAL bytes written",
      "type": "graph",
      "targets": [
        { "expr": "rate(sqlrustgo_wal_bytes_written_total[5m])" }
      ]
    },
    {
      "title": "TPC-H 22 query status",
      "type": "table",
      "targets": [
        { "expr": "sqlrustgo_tpch_query_status" }
      ]
    }
  ]
}
```

### Alert rules (Prometheus)

```yaml
groups:
  - name: sqlrustgo
    rules:
      - alert: SQLRustGoDown
        expr: sqlrustgo_up == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "SQLRustGo is down"
          description: "Instance {{ $labels.instance }} has been down for 1+ minutes."

      - alert: HighQueryLatency
        expr: histogram_quantile(0.99, rate(sqlrustgo_query_duration_seconds_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "p99 query latency > 5s"

      - alert: HighWALLag
        expr: sqlrustgo_wal_lag_seconds > 60
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "WAL archive lag > 60s"

      - alert: ConnectionSaturation
        expr: sqlrustgo_active_connections / sqlrustgo_max_connections > 0.9
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Connection pool 90% full"
```

### Log shipping with Promtail

`/etc/promtail/config.yml`:

```yaml
server:
  http_listen_port: 9080

positions:
  filename: /var/lib/promtail/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: sqlrustgo
    static_configs:
      - targets:
          - localhost
        labels:
          job: sqlrustgo
          __path__: /var/log/sqlrustgo/*.log
    pipeline_stages:
      - json:
          expressions:
            level: level
            target: target
            msg: msg
      - labels:
          level:
          target:
```

## TLS with Let's Encrypt

```bash
# Install certbot
sudo apt install certbot

# Get cert (standalone mode — stop sqlrustgo first)
sudo systemctl stop sqlrustgo
sudo certbot certonly --standalone \
  -d sqlrustgo.example.com \
  --agree-tos --no-eff-email \
  --register-unsafely-without-email

# Certs land in /etc/letsencrypt/live/sqlrustgo.example.com/
sudo cp /etc/letsencrypt/live/sqlrustgo.example.com/fullchain.pem /etc/sqlrustgo/tls/server.crt
sudo cp /etc/letsencrypt/live/sqlrustgo.example.com/privkey.pem /etc/sqlrustgo/tls/server.key
sudo chown -R sqlrustgo:sqlrustgo /etc/sqlrustgo
sudo chmod 0600 /etc/sqlrustgo/tls/server.key

# Auto-renewal
sudo systemctl start sqlrustgo
sudo certbot renew --deploy-hook "systemctl reload sqlrustgo"
```

## Upgrades and rollback

### In-place upgrade (v3.8.0 → v3.9.0)

```bash
# 1. Take snapshot
sqlrustgo admin snapshot --output /backup/pre-v390.snap

# 2. Stop server
sudo systemctl stop sqlrustgo

# 3. Install new binary
sudo install -m 0755 sqlrustgo-v3.9.0 /usr/local/bin/sqlrustgo

# 4. Run migration (if any — see MIGRATION_GUIDE.md)
sqlrustgo admin migrate --from v3.8.0 --to v3.9.0

# 5. Start server
sudo systemctl start sqlrustgo
sudo systemctl status sqlrustgo
```

### Rollback

```bash
sudo systemctl stop sqlrustgo
sudo install -m 0755 sqlrustgo-v3.8.0 /usr/local/bin/sqlrustgo
sqlrustgo admin restore --from /backup/pre-v390.snap
sudo systemctl start sqlrustgo
```

### Blue-green deploy

For zero-downtime upgrades:

```bash
# 1. Start v3.9.0 on port 5433
sqlrustgo-v3.9.0 server --port 5433 --data-dir /var/lib/sqlrustgo-v390

# 2. Restore latest snapshot to v3.9.0
sqlrustgo admin restore --from /backup/latest.snap --into /var/lib/sqlrustgo-v390

# 3. Wait for replay to finish
sqlrustgo admin wait-ready --port 5433

# 4. Switch load balancer
# (Update PgBouncer backend list to point to 5433)

# 5. Stop v3.8.0
sudo systemctl stop sqlrustgo-v380
```
