# v3.11.0 TPC-H 性能报告

## 1. 文档定位

本文件记录 TPC-H 相关性能结果。当前正式结论应以 `TPCH_SF1_22_22_PASS_REPORT.md` 和 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 为准；本文件中的英文原文保留为历史记录。

## 2. 当前结论

- v3.11.0 已证明 TPC-H SF=1 22/22 query 可完整运行。
- 总耗时约 519.15s，运行过程 0 OOM、0 panic。
- 部分 query 返回 0 行，仍需 correctness 分析。
- 性能比较需要同 fixture、同硬件、同 query、同输出校验的外部数据库基线。

## 3. 后续工作

v3.12 应补齐：cross-engine row-count/SHA256、wire protocol per-query artifact、LOAD DATA 导入性能、内存峰值、timeout 策略和 baseline regression gate。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# TPC-H Performance Report v3.11.0

> **Version**: v3.11.0
> **Date**: 2026-08-08
> **Status**: GA G4 FAIL — real TPC-H SF=1 22/22 evidence missing
> **Evidence**: `../GA_GATE_REPORT.md`, `../TPCH_SF1_VERIFICATION_REPORT.md`, `../../../../SF1_TRUTH_AUDIT.md`

## 1. Executive Summary

This report supersedes the 2026-07-18 performance report that claimed "All 22 queries PASS at SF=1.0". That claim is not valid GA evidence.

| Metric | Previous claim | Current evidence-bound status |
|--------|----------------|-------------------------------|
| TPC-H SF=1 | 22/22 PASS | FAIL for GA: fixture missing, real 22/22 not executed |
| Q2/Q5/Q21 fixes | Fixed by PRs | Code fixes may exist, but they do not prove full SF=1 22/22 |
| SQLite baseline | 22 query rows/times | SQLite data is not sqlrustgo SF=1 gate evidence |
| GA G4 | PASS implied | FAIL per `GA_GATE_REPORT.md` |

## 2. What Can Be Claimed

- sqlrustgo has historical fixes related to Q2, Q4, Q5, and Q21.
- `queries/q1.sql` through `queries/q22.sql` exist.
- `scripts/tpch/run_sf1.sh` and `scripts/gate/check_tpch_sf1.sh` exist.
- `bash scripts/gate/check_tpch_sf1.sh` defaults to dry-run and reports missing `/tmp/tpch-sf1` fixture on this machine.

## 3. What Must Not Be Claimed

- Do not claim TPC-H SF=1 22/22 PASS for v3.11.0.
- Do not use SQLite-only timings as sqlrustgo SF=1 performance evidence.
- Do not promote v3.11.0 to GA while `STAGE.yaml` is RC and `GA_GATE_REPORT.md` is NOT READY.

## 4. Required Evidence Before GA

| Requirement | Required evidence |
|-------------|-------------------|
| Fixture | `/tmp/tpch-sf1/*.tbl` generated from dbgen with official SF=1 row counts |
| Execution | `cargo test --release --test tpch_sf1_22_vs_3engines_test -- --include-ignored --nocapture` exits 0 |
| Correctness | PostgreSQL SHA256 comparison for all 22 queries |
| Documentation | `GA_GATE_REPORT.md`, `STAGE.yaml`, and release notes updated after the real run |

## 5. References

- `docs/releases/v3.11.0/GA_GATE_REPORT.md`
- `docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md`
- `docs/releases/v3.11.0/AUDIT_V311_REALITY_CHECK.md`
- `SF1_TRUTH_AUDIT.md`
