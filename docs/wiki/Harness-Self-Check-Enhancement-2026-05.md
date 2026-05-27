# Harness Self-Check & Enhancement Plan (2026-05-08)

## Nomad CI/CD Stack Status

| Component | Status | Details |
|-----------|--------|---------|
| Nomad HP Z6G4 | ✅ Ready | hp-z6g4/high-cpu/40core |
| Nomad 250 MacMini | ✅ Ready | 250MacMini |
| nomad-runner container | ✅ Up 34h | docker ps verified |
| Gitea Actions Runner | ✅ Online | registered in harness system |
| SSH Access | ✅ | openclaw@192.168.0.252 |

## Harness Self-Check Protocol

### Daily Health Check Commands

```bash
# 1. Nomad node status
ssh openclaw@192.168.0.252 "nomad node status"

# 2. Runner container
ssh openclaw@192.168.0.252 "docker ps --format 'table {{.Names}}\t{{.Status}}'"

# 3. Gitea Actions runners (API)
curl -s http://192.168.0.252:3000/api/v1/actions/runners \
  -H "Authorization: token cc19b2ad677e18c96b9d049f6dc2b46e02176883"

# 4. Nomad jobs
ssh openclaw@192.168.0.252 "nomad job status"
```

## Test System Enhancement

### Gate Coverage Target

| Gate | Current | Target | Status |
|------|---------|--------|--------|
| Unit Test Pass Rate | ~90% | ≥90% | B2 in progress |
| Coverage | <75% | ≥75% | B5 pending |
| TPC-H SF=1 | 22 queries | 22/22 pass | TPCH-SF1 pending |

### Self-Check Script

```bash
# check_harness_health.sh
#!/bin/bash
set -e

echo "=== Nomad Nodes ==="
ssh openclaw@192.168.0.252 "nomad node status" | grep -E "ready|down"

echo "=== Runner Container ==="
ssh openclaw@192.168.0.252 "docker ps --filter name=nomad-runner --format '{{.Status}}'"

echo "=== Gitea Actions Runners ==="
curl -s http://192.168.0.252:3000/api/v1/actions/runners \
  -H "Authorization: token ${GITEA_TOKEN}" | jq '.runners[] | {name, status}'

echo "=== Nomad Jobs (last 5) ==="
ssh openclaw@192.168.0.252 "nomad job history -max 5" 2>/dev/null || \
  ssh openclaw@192.168.0.252 "nomad job status" | head -20
```

## Documentation System Enhancement

### Wiki Sync Strategy

| Source | Target | Method |
|--------|--------|--------|
| GBrain SSOT (192.168.0.250:8081) | All nodes | Pull on demand |
| Gitea Wiki | Local clone | git push |
| GitHub Wiki | Disabled | N/A |
| QMD docs | GBrain sync | hermes-gbrain-builder |

### GBrain Sync Protocol

```bash
# 1. Clone wiki
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.wiki.git /tmp/wiki

# 2. Update gbrain/ directory
git -C /tmp/wiki add gbrain/

# 3. Commit + push
git -C /tmp/wiki commit -m "Sync GBrain $(date +%Y-%m-%d)"
git -C /tmp/wiki push origin master

# 4. Sync to GBrain SSOT (each node pulls)
curl -s http://192.168.0.250:8081/memory/sync -X POST \
  -H "Content-Type: application/json" \
  -d '{"source": "wiki", "timestamp": "'$(date -Iseconds)'"}'
```

## Enhancement Roadmap

| Item | Priority | Owner | Status |
|------|----------|-------|--------|
| B2: Full Test Suite ≥90% | P0 | CI/CD | in_progress |
| B5: Coverage ≥75% | P0 | CI/CD | pending |
| TPCH-SF1: 22/22 queries | P0 | CI/CD | pending |
| Nomad runner auto-restart | P1 | DevOps | planned |
| GBrain sync automation | P1 | Hermes | planned |
| Wiki → GBrain pipeline | P2 | Hermes | planned |

## Gate Policy Reference

See: `gbrain/gate/README.md` (三权分离: AI/Policy/Nomad)

## Related

- [Nomad Scheduler](Nomad-Scheduler.md)
- [Hermes Agent](Hermes-Agent.md)
- [CI-CD-Platform](CI-CD-Platform.md)
- [GBrain Knowledge](GBrain-Knowledge.md)
