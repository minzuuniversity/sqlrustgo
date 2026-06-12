# 05 - Security Scan Report

## Scan Tools

- **Tool**: `cargo audit` (RustSec Advisory Database)
- **Date**: 2026-06-13
- **Crates scanned**: 591

## Findings

| ID | Crate | Version | Severity | Status |
|----|-------|---------|----------|--------|
| RUSTSEC-2026-0179 | postgres-protocol | 0.6.11 | High (8.7) | ⚠️ DRIFT |
| RUSTSEC-2026-0180 | postgres-protocol | 0.6.11 | Medium (6.9) | ⚠️ DRIFT |
| RUSTSEC-2026-0181 | tokio-postgres | 0.7.17 | Medium (6.9) | ⚠️ DRIFT |

## DRIFT Scope Analysis

All 3 vulnerabilities are in `crates/bench` (benchmark tool, **not production binary**):
- `sqlrustgo-bench` depends on `tokio-postgres 0.7.17` → `postgres-protocol 0.6.11`
- Production binary `sqlrustgo-mysql-server` does **NOT** depend on these crates
- 0 vulnerabilities in production code paths

## Remediation

For v3.9.0 GA:
- Production binary unaffected (verified by `cargo tree -p sqlrustgo-mysql-server`)
- `crates/bench` upgrade tracked separately (post-GA)

For v3.9.1 (post-GA):
- Upgrade `tokio-postgres` to >=0.7.18
- Upgrade `postgres-protocol` to >=0.6.12
- Re-run `cargo audit`

## Security Score

- **Production binary**: 0 critical, 0 high, 0 medium (clean)
- **Bench crate**: 1 high, 2 medium (DRIFT, non-production)
- **Overall for GA**: ✅ ACCEPTABLE
