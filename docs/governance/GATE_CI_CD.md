# Governance Gate CI/CD Automation

> **Version**: 1.0
> **Date**: 2026-05-02
> **Gate**: G-04

---

## Overview

This document describes the automated CI/CD pipeline for governance gate verification in SQLRustGo v2.9.0.

## Gates Covered

| Gate | Name | Description |
|------|------|-------------|
| R1-R7 | Core Gates | Build, Test, Clippy, Format, Coverage, Security, Docs |
| R8 | SQL Compatibility | SQL Corpus ≥80% |
| R9 | Performance | Performance baseline no regression |
| R10 | Formal Proof | ≥10 proof files verified |
| G-Gate | Attack Surface | AV1-AV10 verified |

## CI/CD Pipeline

### Workflow File

Location: `.github/workflows/r-gate.yml`

### Pipeline Stages

```
┌─────────────────────────────────────────────────────────────┐
│                    Pull Request Created                        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                 R1-R7: Core Gates (Parallel)                 │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │   R1    │ │   R2    │ │   R3    │ │   R4    │           │
│  │  Build  │ │  Test   │ │ Clippy  │ │ Format  │           │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                       │
│  │   R5    │ │   R6    │ │   R7    │                       │
│  │Coverage │ │Security │ │  Docs   │                       │
│  └─────────┘ └─────────┘ └─────────┘                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Extended Gates (Parallel)                  │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │   R8    │ │   R9    │ │  R10    │ │G-Gate   │           │
│  │   SQL   │ │  Perf   │ │  Proof  │ │   AV    │           │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Merge Eligibility                        │
│                    All Gates Passed?                         │
└─────────────────────────────────────────────────────────────┘
```

## Webhook Notifications

### Configuration

Set environment variables:
```bash
export GATE_WEBHOOK_URL=https://your-webhook.com/gate
export GATE_WEBHOOK_SECRET=your-secret
```

### Usage

```bash
# Send violation notification
./scripts/gate/gate_webhook.sh violation R8 "SQL Corpus below 80%"

# Send pass notification
./scripts/gate/gate_webhook.sh passed R1-R7
```

### Webhook Payload

```json
{
  "event": "gate_violation",
  "gate": "R8",
  "status": "VIOLATION",
  "message": "SQL Corpus below 80%",
  "details": "Current: 75%, Required: 80%",
  "timestamp": "2026-05-02T12:00:00Z",
  "branch": "refs/heads/develop/v2.9.0",
  "commit": "abc123..."
}
```

## Local Verification

Before pushing, verify locally:

```bash
# Run all gates locally
cargo build --all-features
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check
bash scripts/gate/check_coverage.sh
bash scripts/gate/check_security.sh
bash scripts/gate/check_sql_compat.sh
bash scripts/gate/check_proof.sh
bash scripts/gate/check_attack_surface.sh
```

## Integration with Gitea Actions

For Gitea Actions integration, add to `.gitea/workflows/r-gate.yml`:

```yaml
name: R-Gate Verification
on: [pull_request]

jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: bash scripts/gate/run_hermes_gate.sh
```

## Failure Handling

When a gate fails:

1. CI blocks the PR
2. Webhook notification sent (if configured)
3. PR author notified via comments
4. Failure details in CI logs

## Success Handling

When all gates pass:

1. CI marks PR as mergeable
2. Webhook notification sent (optional)
3. Merge button enabled

---

*Document verified by: openclaw*
*Date: 2026-05-02*

---

## 附录 A：v3.8.0 起生效的 A2 步骤更新

`scripts/gate/check_alpha_v380.sh` 的 A2 步骤在 v3.8.0 引入 `A2_EPHEMERAL_SMOKE` 子检查，强制走线协议：

```
A2_TEST          cargo test --lib (核心 6 crates)
A2_EPHEMERAL_SMOKE  cargo test --test embedded_harness_smoke --test embedded_harness_isolation
```

任何 e2e / 集成测试若声明"通过 A2"，必须同时通过 `A2_EPHEMERAL_SMOKE`（即通过 `sqlrustgo-mysql-server` 的 wire protocol 路径）。详细规范见 [SPEC-v3.8.0-001](../releases/v3.8.0/SPEC-v3.8.0-001-mysql-server-canonical-entry.md) §5。
