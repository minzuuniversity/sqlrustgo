# 00 - Release Summary

## v3.9.0 Release Information

| Field | Value |
|-------|-------|
| **Version** | v3.9.0-rc7 (in progress to GA) |
| **Release Type** | Production Readiness Release |
| **Tag (latest)** | v3.9.0-rc7 @ `642ff9cf9` |
| **Tag (current tip)** | `8a83e2553` (post-#3378 REMOTE_LIMITS + #3377 .gitattributes) |
| **GA Target** | 2026-12-15 (per Hermes audit #3252) |
| **Release Manager** | yinglichina8848 (with Hermes Agent) |
| **Build Environment** | Mac mini (M2) + Z6G4 (build) + Z440 (backup) |

## Build Status

- **Compile**: ✅ PASS (cargo build --release)
- **Tests**: ✅ PASS (330+ tests, 36 substance)
- **Gates**: ✅ G1-G13, G16 PASS
- **Lint**: ✅ clippy --all-features (D warnings)
- **Format**: ✅ cargo fmt --check

## Distribution

- **Primary**: http://192.168.0.252:3000/openclaw/sqlrustgo
- **Backup**: http://192.168.0.250:3000/openclaw/sqlrustgo
- **Mirror**: https://github.com/minzuuniversity/sqlrustgo
- **CI/CD**: Gitea Actions (B-Gate, E-Gate, evidence-graphs)
