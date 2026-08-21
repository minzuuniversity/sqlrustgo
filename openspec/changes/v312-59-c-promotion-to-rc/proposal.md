## Why

`docs/releases/v3.12.0/STAGE.yaml` 第 94-105 行列出 11 项 `promotion_to_RC_requires`，但只有 3 项 (#6 TPC-H cross-engine, #9-10 V312-57 week01-04) 有对应 issue/evidence。其余 8 项完全无对应 issue，必须本总控子 issue 整改（issue #4386, V312-59-C）。

Risk: 如果不在 RC promotion 之前补齐 8 项 evidence 与独立 gate 校验，RC promotion 容易「声明满足但实际未跑」（Anti-Fabrication-Policy-v1.0 §5）。

## What Changes

- **NEW** `scripts/gate/check_v312_promotion_to_rc.sh` — composite gate enforcing 11 RC items with both file-existence + integration test PASS
- **NEW** `docs/releases/v3.12.0/evidence/v312-59/RC{1,2,3,4,5,7,8,10,11}_*.md` — per-RC-item evidence reports (8 new, 1 reusing r11_claim_audit)
- **NEW** `tests/compat/bustubx_edu_sqlite_cli/week05/` + `week06/` — RC-10 missing fixtures (executor + join/aggregate 3 cases each = 6 new fixtures)
- **MOD** `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` — register 6 new week05-06 cases
- **MOD** `scripts/gate/check_bustubx_edu_cli_v312.sh` — auto-detect new weeks
- **MOD** `docs/releases/v3.12.0/STAGE.yaml` — explicit RC gate references in `promotion_to_RC_requires`

## Inventory (existing evidence per RC item)

| # | RC Item | Existing evidence | Action |
|---|---|---|---|
| 1 | gmp-md ingestion | `v312-03-gmp-ingestion-report.md` (PR #3916) | wrap as `RC1_GMP_MD_INGESTION_REPORT.md` |
| 2 | retrieval quality | `v312-05-hybrid-retrieval-report.md` | wrap as `RC2_RETRIEVAL_QUALITY_REPORT.md` |
| 3 | backup/restore | `v312-09-backup-restore-report.md` | wrap as `RC3_BACKUP_RESTORE_REPORT.md` |
| 4 | security/RBAC | `evidence/gmp_compliance/V312-53-REPORT.md` (12 ACL tests) | wrap as `RC4_SECURITY_RBAC_REPORT.md` |
| 5 | SQLLogicTest curated | `sqllogictest-baseline/sqlite-corpus-manifest.json` (V312-11) | wrap as `RC5_CURATED_SQLLOGICTEST_REPORT.md` |
| 6 | TPC-H SF=1 cross-engine | (covered by #4374-#4382 V312-58) | NO-OP, reference in gate |
| 7 | wire + LOAD DATA | `evidence/wire_load_data/V312-13-REPORT.md` | wrap as `RC7_WIRE_LOAD_DATA_REPORT.md` |
| 8 | crash recovery + upgrade | `crash-recovery-upgrade-verification-report.md` | wrap as `RC8_CRASH_UPGRADE_REPORT.md` |
| 9 | V312-57 week01-04 | (covered by #4359, #4370, #4373) | NO-OP, reference in gate |
| 10 | V312-57 week05-06 | **MISSING** | create 6 fixtures + manifest entries |
| 11 | claim cleanup | `evidence/r11_claim_audit/CLAIM_AUDIT_2026-08-19.md` | wrap as `RC11_CLAIM_CLEANUP_REPORT.md` |

## Anti-patterns (Anti-Fabrication-Policy-v1.0 §5)

- ❌ `expiry 2027-06-30` 推回任何 RC blocker（11 项全部 in-v3.12）
- ❌ 把多 RC 项合并到一个 PR 而不复用每项独立 commit
- ❌ "现有 evidence 文档已足够" — 必须有 commit hash + exit code + evidence_hash
- ❌ `cargo test --lib --quiet` 代替 integration test
- ❌ 不附实际 backup-restore 二进制对比 hash 的"已验证"声明

## Provenance

- discovered_during: v312-beta-remediation-2026-08-20
- generated_by: claude-code v3.12.0
- source_run: v3.12.0-rc-requires-gap
- evidence_root: `docs/releases/v3.12.0/STAGE.yaml` lines 94-105
- branch: `develop/v3.12.0` @ 73069c6a6
- cross_ref: #4383 V312-59 (grand-parent), #4359/#4370/#4373 V312-57 (overlap #9-10), #4374-#4382 V312-58 (overlap #6), #3887 V312-MASTER
- policy: Anti-Fabrication-Policy-v1.0