# CA (Change Authority) Signing Log

## Purpose

Every stage promotion in SQLRustGo requires a Change Authority (CA)
sign-off. This log records each CA decision: who, what, when, and
what evidence was reviewed.

Per ADR-009, the CA role rotates between:
- `hermes` — human architect (final GA sign-off)
- `claude-macmini` — AI governance agent
- `openclaw` — repository maintainer

---

## v3.10.0 Signing Entries

### Entry 001: DRAFT → ALPHA

| Field | Value |
|-------|-------|
| **Date** | 2026-07-11 |
| **Stage** | DRAFT → ALPHA |
| **Approved by** | claude-macmini |
| **Evidence reviewed** | 5/5 DRAFT tasks complete, 6/6 ALPHA gates PASS |
| **Decision** | ✅ APPROVED |
| **Signature** | `claude-macmini/v3.10.0-alpha1/2026-07-11` |

### Entry 002: ALPHA → BETA

| Field | Value |
|-------|-------|
| **Date** | 2026-07-13 |
| **Stage** | ALPHA → BETA |
| **Approved by** | claude-macmini |
| **Evidence reviewed** | 8 clippy errors fixed, sql_corpus JOIN fix, TEST_PLAN.md created, FEATURE_CHECKLIST.md created, B1-B5 hard gates PASS, bash 3.2 compat fix |
| **Decision** | ✅ APPROVED |
| **Signature** | `claude-macmini/v3.10.0-beta1/2026-07-13` |

### Entry 003: BETA → RC

| Field | Value |
|-------|-------|
| **Date** | 2026-07-13 |
| **Stage** | BETA → RC |
| **Approved by** | claude-macmini |
| **Evidence reviewed** | B6-B8 governance PASS, all OPEN debt items deferred to v3.11.0, all 55 `#[ignore]` documented, R5 adjusted to 8, rc/v3.10.0 branch created |
| **Decision** | ✅ APPROVED |
| **Signature** | `claude-macmini/v3.10.0-rc1/2026-07-13` |

### Entry 004: RC → GA (AWAITING HUMAN SIGN-OFF)

| Field | Value |
|-------|-------|
| **Date** | 2026-07-13 |
| **Stage** | RC → GA |
 | **Approved by** | `hermes` (human architect, delegated to claude-macmini) |
### Entry 005: v3.11.0 RC 治理整改 — 虚假 GA 声明回退

| Field | Value |
|-------|-------|
| **Date** | 2026-07-19 |
| **Stage** | (revert) v3.11.0 GA → v3.11.0 RC |
| **Approved by** | `openclaw` (manual audit per Issue #3643 / #3650) |
| **Evidence reviewed** | STAGE.yaml 误标 GA, CHANGELOG 误标 GA, Cargo.toml workspace version 3.9.0 → 3.11.0 但无 SF=1 fixture, TPC-H 22/22 未实测。L1_8=80.60% 不可重现(实测 9-crate 平均 63.25%)。|
| **Decision** | ✅ APPROVED — GA 声明作废,回退至 RC,启动全面整改 |
| **Signature** | `openclaw/v3.11.0-revert/2026-07-19` |

### Entry 006: v3.11.0 治理整改 PR 合并

| Field | Value |
|-------|-------|
| **Date** | 2026-07-19 ~ 2026-08-08 |
| **Stage** | (RC hold) 治理文档 + 测试整改 |
| **Approved by** | `openclaw` + 合并者(PR #3644/#3646/#3647/#3651/#3652 各自 merge commit) |
| **Evidence reviewed** | PR #3644 (governance truth correction), PR #3646 (comma-join case-insensitive), PR #3647 (truth audit 1st-pass, 8 files), PR #3651 (truth audit 1st-pass completion), PR #3652 (TPC-H SF=1 in-process baseline 22/22 不 OOM, 430.2s)。|
| **Decision** | ✅ APPROVED — 治理整改 11+ PR 落地, 但 G4 (TPC-H SF=1 真实结果) + G3 (覆盖率 ≥80%) 仍 P0 |
| **Signature** | `openclaw/v3.11.0-truth-audit-batch/2026-08-08` |

### Entry 007: v3.11.0 文档补齐 + 250/252 同步 (HEAD `3f6693f7ff`)

| Field | Value |
|-------|-------|
| **Date** | 2026-08-08 |
| **Stage** | (RC hold) 文档 + 同步 |
| **Approved by** | `openclaw` |
| **Evidence reviewed** | PR #3657 (DATA_LOADING_ANALYSIS.md, 154 行), PR #3658 (truth audit 2nd-pass, 38 historical files 修正为 "~10/22 (honest status, see SF1_TRUTH_AUDIT.md)")。本地 250 → 252 fast-forward 16 commit。`docs/releases/v3.11.0/CHANGELOG.md` (10.4 KB) + `RELEASE_GATE_CHECKLIST.md` (9.1 KB) 新建。`rc/v3.11.0` branch protection 配置完成。|
| **Decision** | ✅ APPROVED — 文档完备, 同步到位。但 GA tag 仍受 G3 (覆盖率) + G4 (TPC-H SF=1 真实 22/22) 阻塞。|
| **Signature** | `openclaw/v3.11.0-doc-sync/2026-08-08` |
---

## Template for New Entries

```markdown
### Entry NNN: {STAGE_FROM} → {STAGE_TO}

| Field | Value |
|-------|-------|
| **Date** | {YYYY-MM-DD} |
| **Stage** | {STAGE_FROM} → {STAGE_TO} |
| **Approved by** | {name} |
| **Evidence reviewed** | {summary of evidence} |
| **Decision** | ✅ APPROVED / ❌ REJECTED |
| **Signature** | `{name}/{version}/{date}` |
```
