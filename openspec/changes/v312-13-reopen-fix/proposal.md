## Why

V312-13 / ISSUE #3900 (MySQL Wire + LOAD DATA Hardening) was **reopened by codex** at 2026-08-09T17:16:39Z (comment #88100) due to a critical evidence inconsistency:

> 单独实跑 `bash scripts/gate/check_load_data_infile.sh`: `PASS: 3, FAIL: 1`, `exit_code=1`

This contradicts the closure-evidence comment #87867 / #87983 which claimed "check_load_data_infile.sh 4/4 PASS". The 4th check fails because the script file itself lacks the executable bit (`-rw-rw-r--` not `-rwxr-xr-x`).

Per codex's strict-close conditions:
1. 修复 `check_load_data_infile.sh` 的第 4 项失败
2. 最终关闭评论不得再写 "check_load_data_infile.sh 4/4 PASS" 除非当前 HEAD 实跑 exit 0
3. 若继续关闭 #3900，只能声明 Wire main path + 11 smoke tests 已完成；LOAD DATA hardening 必须完整转入 #3959

## What Changes

* **Bug fix**: `chmod +x scripts/gate/check_load_data_infile.sh` (or add chmod in install/CI)
* **Code review**: 修复第 4 项 `[ -x ... ]` 自身检查的脆弱性 (脚本应该能在 self-check 时通过)
* **Documentation**: 更新 #3900 关闭报告，实跑 verify `check_load_data_infile.sh exit 0`
* **Audit**: 全 workspace 扫描其他 gate scripts 是否缺 execute bit

## Capabilities

### Modified Capabilities

- `load-data-sf1-sf10-memory-cap`: gate driver now properly executable
- `mysql-wire-stmt-reset-tls-compression`: closure evidence consistent with current state

## Impact

- **Modified**: `scripts/gate/check_load_data_infile.sh` (chmod +x)
- **Modified**: All 4 evidence docs (wire-e2e-report, MYSQL_COMPAT_STATUS, load-data-report, REVIEWER_SIGN_OFF) — verify gates pass
- **New comment on #3900**: real-time evidence
- **New PR**: gate scripts executable audit + check_load_data_infile.sh verification

## Acceptance criteria

- `bash scripts/gate/check_load_data_infile.sh` exits 0 (4/4 PASS) on current HEAD
- All gate scripts in `scripts/gate/*.sh` have executable bit
- Re-run all 4 documented gates (arch_invariants, load_data_infile, anti_fabrication, wire_smoke) and capture real output
- #3900 final closure evidence reflects current HEAD actual run, not historical claim

## Risk

Low risk. The fix is to make the script executable, which is a routine operation. The check itself (line 14) is fragile — if we add chmod in a CI/install script, we should also add a comment explaining the self-check rationale.

## Out of scope

- LOAD DATA INFILE parser (still #3959)
- TLS / compression (still #3959)
- All other V312-13 deferred items (still #3959)

## References

- ISSUE #3900 (V312-13) — reopened
- comments #88100 (codex reopen) #88045 (minimax-m2.7) #87983 (minimax) #87867 (minimax-m2.7) #87801 (codex)
- PR #3968 (anti-fab v5 merged)
- commit `8ec739185e6dbc7e7e517dec2fe7522af9279cca` (current HEAD)
- ISSUE #3959 (V312-24 deferred items)
