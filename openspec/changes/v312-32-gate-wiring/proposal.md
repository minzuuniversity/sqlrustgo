# V312-32: V312-24 Gate Wiring (V312-30 codex 整改)

> **Author**: minimax (claude-code)
> **Date**: 2026-08-09
> **Branch**: `feature/v312-32-gate-wiring`
> **Tracking issue**: #3911 (V312-24 follow-up)
> **Source spec**: codex comments on PR #3950 (评论 #87209 + #87655)

## Why

Per codex (评论 #87209 2026-08-09T12:58 + #87655 2026-08-09T14:37):

> "把 OpenSpec 落成实际 gate 脚本、CI/本地门禁入口和报告产物"
> "激活 SQLancer、test-runner、test-registry 或明确拆分延期"
> "处理 stale E2E scripts、dead-code scripts、anti-fabrication violations"
> "证明这些 gate 能被 `check_stage` 或 v3.12 release gate 调用"
> "提交命令、日志、PASS/FAIL、commit、evidence_hash"

V312-24 + V312-25..30 work merged (PR #3950) 但 V312-24 SQLancer + test-runner + test-registry artifacts **没有** wired into `scripts/gate/check_anti_fabrication.sh` (AFP v4) 或 `scripts/gate/check_rc_ga_gate.sh` (D2-Beta). 这意味着 merge 是代码完成, **不是**门禁完成 — V312-24 仍可被未来 commit regress 而没人能 catch。

## What

| File | Change | Reason |
|------|--------|--------|
| `scripts/gate/check_anti_fabrication.sh` | +`check_v312_24_test_infra_artifacts()` function (CHECK 1.5) | AFP v4 now forces `target/sqlancer-report.json` + `target/test-runner-report.json` valid JSON on every run |
| `scripts/gate/check_rc_ga_gate.sh` | +B6 V312-24 in D2-Beta | RC gate now requires both artifacts present + schema-valid; V312-24 regression = gate fail |

No other file changes. This is purely **wiring** — it makes the existing V312-24 binaries enforceable at the v3.12 release gate boundary.

## Acceptance criteria (closing boundary)

1. `bash scripts/gate/check_anti_fabrication.sh` includes new CHECK 1.5 line `V312-24: target/sqlancer-report.json valid (iterations=...)` with exit 0
2. `bash scripts/gate/check_rc_ga_gate.sh` includes `B6 V312-24` section in D2-Beta
3. `git log --oneline | grep V312-32` shows this commit on `develop/v3.12.0`
4. `docs/releases/v3.12.0/V312-30_stage_transition_report.md` exists with 5 gate 实跑 + sha256 + 7 豁免清单

## Evidence gates

- `bash scripts/gate/check_anti_fabrication.sh 2>&1 | grep "CHECK 1.5"` output: 3 PASS lines
- `grep "B6: V312-24" scripts/gate/check_rc_ga_gate.sh` output: 1 line
- `cat docs/releases/v3.12.0/V312-30_stage_transition_report.md | grep "GATE 1\|GATE 2\|GATE 3\|GATE 4\|GATE 5"`: 5 sections
- 5 evidence logs with sha256 in `/tmp/v312_30_signoff_evidence/`

## Out of scope

- V312-31 follow-up: 28 remaining `|| true` masks in 8 gate scripts
- V312-17 follow-up: mysql_compat_test pre-existing compile errors
- V312-30 sign-off: requires 2 reviewer APPROVED on PR + acceptance of 7 豁免清单

## Owner

minimax
