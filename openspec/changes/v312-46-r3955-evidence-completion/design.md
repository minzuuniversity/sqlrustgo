# #3955 Evidence Completion — Design

## Current #3955 Body (现状)

```
## V312-19 R2.7 + Corpus Runner Fixes
**Commit:** 79289593c142d4f3c474e9bccaf137a05d0529ff
**Branch:** feature/v312-19-r2-followup-fixes
### Changes
#### R2.7 Anti-fabrication Gate Fix
... (3 sections)
### Verification
... (2 commands)
### Related Issues
- #3944
- #3945
agent: minimax-m2.7
```

## Missing fields per #3887 strict close conditions

| 字段 | 当前 | 补充 |
|------|------|------|
| source_agent | "minimax-m2.7" (有) | ✓ |
| source_run | "sqlrustgo-session-2026-08-09" (有) | ✓ |
| timestamp | "2026-08-09T15:30:00+08:00" (有) | ✓ |
| commit SHA | "79289593c14..." (有) | ✓ |
| 命令 | 部分 (Verification 块) | 补全 (含 4 个完整命令) |
| PASS/FAIL 摘要 | 部分 | 补全 (4 个命令的 PASS/FAIL 结果) |
| **evidence_hash** | ❌ 缺失 | 计算 4 个 log 的 sha256 |
| **log 路径** | ❌ 缺失 | 指向 `docs/releases/v3.12.0/evidence/...` 实际 log |
| **#3887 勾选状态** | ❌ 缺失 | 新增 #3887 comment 报告 |

## Evidence sources (4 commands)

1. **`bash scripts/gate/check_anti_fabrication.sh`** → log path: `docs/releases/v3.12.0/evidence/arch_invariants/R2.7.stdout` (或实跑重定向)
2. **`bash scripts/gate/test_sql_corpus.sh --fast`** → log path: `docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md`
3. **`cargo test -p sqlrustgo-mysql-server --test wire_smoke_mysql_cli`** → 实跑 (无持久 log)
4. **`bash scripts/gate/check_v312_21_mysql_compat.sh`** → log path: `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md`

## Evidence_hash calculation

```bash
sha256sum docs/releases/v3.12.0/evidence/arch_invariants/R2.7.stdout
sha256sum docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md
sha256sum docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md
```

## Implementation steps

1. Compute sha256 of 3 log files
2. Re-open #3955 body (post-close edit) with full evidence fields
3. Post comment to #3887 with #3955 evidence completion notification
4. Update #3887 master checklist if needed
