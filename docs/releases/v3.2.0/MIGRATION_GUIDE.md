# v3.2.0 迁移指南

> 本文档说明从 v3.1.0 迁移到 v3.2.0 的必要变更。

## 变更概述

v3.2.0 聚焦于 Trust Convergence（可信收敛），确保系统满足 GMP（良好制造规范）工业标准。主要变更：

- Evidence Export with JSON Signature
- Electronic Signature（21 CFR Part 11 合规）
- Immutable Record / Evidence Chain（证据链）
- Correction Chain（更正链）
- Provenance Tracking（溯源追踪）
- Trusted Timestamp（可信时间戳）
- Audit Chain Verification Tool（审计链验证工具）

## 破坏性变更

**无破坏性变更**。v3.2.0 完全向后兼容 v3.1.0。

## 配置迁移

### 新增配置项

```toml
[gmp]
evidence_enabled = true
signature_enabled = true

[gmp.evidence]
json_signature = true

[gmp.timestamp]
trusted = true
rfc3161_server = "http://timestamp.digicert.com"

[audit]
chain_verification = true
```

## API 变更

### 新增端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/gmp/evidence` | CRUD | 证据管理 |
| `/api/v1/gmp/signature` | POST | 电子签名 |
| `/api/v1/gmp/audit/verify` | POST | 审计链验证 |
| `/api/v1/gmp/timestamp` | POST | 可信时间戳 |
| `/api/v1/gmp/provenance` | GET | 数据溯源 |

### 原有端点

无变更。

## 回滚方案

如遇到问题，可回滚到 v3.1.0：

```bash
git checkout v3.1.0
cargo build --release
```
