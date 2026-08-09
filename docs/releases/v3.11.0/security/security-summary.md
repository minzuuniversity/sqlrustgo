# v3.11.0 安全摘要

## 1. 文档定位

本摘要用于快速查看 v3.11.0 安全状态。依赖漏洞和安全 gate 的最终判断必须以 `SECURITY_AUDIT.md`、`cargo audit` 实跑输出和 CI/gate artifact 为准。

## 2. 当前摘要

| 领域 | 状态 |
|---|---|
| Authentication | 使用 Argon2 password hashing |
| Authorization | RBAC + column-level permissions |
| SQL injection | 主要路径采用参数化或 parser 约束 |
| WAL integrity | WAL entry 带 checksum |
| TLS | 文档声明支持，仍需 E2E gate 复核 |
| Dependency audit | 有 transitive warnings，需定期复跑和例外登记 |

## 3. v3.12 要求

- 复跑 `cargo audit` 并归档输出。
- 补齐 TLS/compression/error packet 的 wire protocol E2E。
- 将 ACL 扩展到 GMP/RAG/vector/graph 检索路径。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# Security Scan Report Summary

- Version: v3.11.0
- Command: cargo audit
- Mode: stale-cache
- Advisory DB: 1237bbe09d2701e14e6593a630fbaf28928df712 2026-08-06T11:34:42+02:00
- Advisory DB Age Days: 1
- Warning Count: 0
- Vulnerability Count: 0
- Status: PASS
- Scan Date: 2026-08-08T07:37:07Z

## Interpretation

PASS requires zero warnings and zero vulnerabilities using either an online audit or an advisory DB cache no older than 7 days.
PARTIAL means the command completed with zero vulnerabilities but used an older cache and/or reported non-vulnerability warnings.

## Report Files

- Security Audit Output: `docs/releases/v3.11.0/security/security_audit_output.txt`
