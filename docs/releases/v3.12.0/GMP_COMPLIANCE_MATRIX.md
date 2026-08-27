# SQLRustGo v3.12.0 GMP 合规矩阵

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0; v312-59-d-ga8-signoff appended 2026-08-22, gate_issue=#4387

> **版本**: v3.12.0
> **状态**: 规划中
> **日期**: 2026-08-08

本矩阵把 GMP 内审检索所需控制项映射到 SQLRustGo 的计划实现和测试证据。所有条目在真实测试和执行证据产生前，都必须保持 `PLANNED`，不得提前写成 PASS 或已完成。

| 控制项 | 合规要求 | 计划实现 | 计划测试 | 状态 |
|---|---|---|---|---|
| Attributable 可归因 | 每个受监管动作都有明确操作者 | `gmp_audit_log.actor` | audit insert 必须带 actor | PLANNED |
| Legible 可读 | 检索证据可读且能链接到来源 | chunk text + source path + citation | retrieval result 必须包含 citation text | PLANNED |
| Contemporaneous 同步记录 | 事件时间在动作发生时记录 | 系统时钟写入 audit timestamp | import/search/export 均有 timestamp | PLANNED |
| Original 原始性 | 保留源文档版本和 hash | document source hash + version table | re-import 创建新版本而不覆盖原文 | PLANNED |
| Accurate 准确 | 检索证据可复核 | chunk hash + embedding hash | 返回 chunk hash 与存储文本一致 | PLANNED |
| Complete 完整 | 语料导入报告覆盖所有文件 | ingestion report | 无未分类失败 | PLANNED |
| Consistent 一致 | 数据模型确定性可复现 | stable document/chunk ids | 同一 corpus 生成相同 ID | PLANNED |
| Enduring 持久 | backup/restore 保留记录 | backup/restore gate | restore 后 hash 与源一致 | PLANNED |
| Available 可用 | 授权用户可检索证据 | ACL + retrieval API | permitted role 可检索 | PLANNED |
| Access Control 访问控制 | 未授权访问 fail closed | role checks | restricted chunk 被拒绝 | PLANNED |
| Audit Trail 审计追踪 | 审计链可防篡改 | previous hash + event hash | tamper test fail closed | PLANNED |
| E-Signature 电子签名 | 审批/导出可签名 | signature hook | approval 需要 signature payload | PLANNED |
| Traceability 可追溯 | findings 链接 SOP/CAPA/clause 证据 | graph projection tables | path query 返回 evidence bundle | PLANNED |
| Retrieval Quality 检索质量 | 内审问题能命中相关文档 | hybrid retrieval + RRF | fixture hit-rate report | PLANNED |

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v3.12.0 GMP Compliance Matrix

> **Version**: v3.12.0
> **Status**: PLANNED
> **Date**: 2026-08-08

This matrix maps GMP internal-audit retrieval controls to implementation and test evidence. Rows must remain PLANNED until tests exist and have execution evidence.

| Control | Requirement | Planned implementation | Planned test | Status |
|---|---|---|---|---|
| Attributable | Every regulated action has an actor | `gmp_audit_log.actor` | audit insert requires actor | PLANNED |
| Legible | Retrieved evidence is readable and source-linked | chunk text + source path + citation | retrieval result includes citation text | PLANNED |
| Contemporaneous | Event timestamps are recorded at action time | audit timestamp from system clock | timestamp exists on import/search/export | PLANNED |
| Original | Source document version and hash are preserved | document source hash + version table | re-import creates version, not overwrite | PLANNED |
| Accurate | Search evidence can be verified | chunk hash + embedding hash | returned chunk hash matches stored text | PLANNED |
| Complete | Corpus ingestion reports all files | ingestion report | no unclassified failures | PLANNED |
| Consistent | Data model is deterministic | stable document/chunk ids | same corpus yields same ids | PLANNED |
| Enduring | Backup/restore preserves records | backup/restore gate | restored hashes match source | PLANNED |
| Available | Authorized users can retrieve evidence | ACL + retrieval API | permitted role can retrieve | PLANNED |
| Access Control | Unauthorized access fails closed | role checks | restricted chunk denied | PLANNED |
| Audit Trail | Tamper-evident audit chain | previous hash + event hash | tamper test fails closed | PLANNED |
| E-Signature | Approval/export can be signed | signature hook | approval requires signature payload | PLANNED |
| Traceability | Findings link to SOP/CAPA/clause evidence | graph projection tables | path query returns evidence bundle | PLANNED |
| Retrieval Quality | Internal-audit questions find relevant docs | hybrid retrieval + RRF | fixture hit-rate report | PLANNED |

## v3.12.0 GA-8 Signoff (Issue #4387 / V312-59-D)

> **signed_at:** 2026-08-22 (v3.12.0 GA promotion cycle)
> **signed_by:** v3.12.0 GA Release Engineering (OpenClaw)
> **gate_issue:** #4505 (V312-59-D v2; re-activated from #4387)
> **umbrella:** #4497 (V312-59-D v2; umbrella re-activated from #4383)
> **verdict:** GA-8 GMP matrix signoff recorded. Rows above remain PLANNED for the
> GMP retrieval/audit subsystem; this signoff attests that the **release-level**
> GMP controls (audit log, access control, backup/restore, retrieval quality)
> are gated by the GA-3 security scan, GA-6 backup/restore aggregator, and
> GA-7 docs consistency checks. No row is marked PASS prematurely.
>
> **Evidence:**
> - `docs/releases/v3.12.0/evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md`
> - `docs/releases/v3.12.0/evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md`
> - `docs/releases/v3.12.0/evidence/v312-59/GA7_DOCS_LINKS_REPORT.md`
> - `docs/releases/v3.12.0/evidence/v312-59/GA7_DOCS_CONSISTENCY_REPORT.md`
> - `docs/releases/v3.12.0/GA_GATE_REPORT.md` (overall verdict)

## v3.12.0 GA-8 Signoff (English Appendix — Issue #4387 / V312-59-D)

> **signed_at:** 2026-08-22 (v3.12.0 GA promotion cycle)
> **signed_by:** v3.12.0 GA Release Engineering (OpenClaw)
> **gate_issue:** #4505 (V312-59-D v2; re-activated from #4387)
> **umbrella:** #4497 (V312-59-D v2; umbrella re-activated from #4383)
> **verdict:** GA-8 GMP matrix signoff recorded. The release-level GMP
> controls are gated by GA-3 (security), GA-6 (backup/recovery/upgrade),
> and GA-7 (docs consistency). Subsystem rows above remain PLANNED until
> retrieval/audit test evidence exists; this signoff does **not** flip
> any row from PLANNED to PASS.
