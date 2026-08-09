# v3.11.0 / v3.12.0 / v4.0.0 文档语言本地化审查报告

> **日期**: 2026-08-09
> **执行人**: Codex
> **范围**: `docs/releases/v3.11.0`、`docs/releases/v3.12.0`、`docs/releases/v4.0.0` 中的 Markdown 文档。
> **原则**: 中文主文优先；命令、路径、crate 名、gate ID、协议名等技术标识保留英文；原英文全文保留在 `## 附录：英文原文`。

## 1. 审查结论

本次审查发现 v3.12.0 和 v4.0.0 新规划文档原先几乎全为英文，v3.11.0 中也存在多份英文报告或英文主文占比较高的历史 gate/report 文档。已对关键文档新增中文正式主文，并把原英文移动到附录，避免丢失历史上下文。

中文主文不改变原有 gate、命令、路径、版本号或 evidence boundary。对存在旧 PENDING/TBD/PASS 混写的英文历史内容，中文主文明确要求以 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 和实际命令输出为准。

## 2. 已完成中文主文化的文件

| 文件 | 中文主文英文词数 | 中文字符数 | 处理方式 |
|---|---:|---:|---|
| `docs/releases/v3.11.0/ALPHA_GATE_REPORT.md` | 18 | 183 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/ARCHITECTURE.md` | 139 | 261 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/AUDIT_V311_REALITY_CHECK.md` | 25 | 209 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/BETA_GATE_REPORT.md` | 22 | 197 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/CHANGELOG.md` | 65 | 250 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/COVERAGE_TESTING_METHODOLOGY.md` | 69 | 267 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/EVIDENCE_STATUS.md` | 19 | 170 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md` | 33 | 218 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/FEATURE_CHECKLIST.md` | 58 | 243 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/G3_COVERAGE_REMEDIATION_PLAN.md` | 86 | 324 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md` | 97 | 305 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/GA_GATE_REPORT.md` | 45 | 217 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/GA_RELEASE_TIMELINE.md` | 38 | 283 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/GOVERNANCE_SELF_AUDIT_2026-08-09.md` | 19 | 228 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/INDEX.md` | 79 | 348 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/PERFORMANCE_REPORT.md` | 73 | 403 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/POST_GA_PLAN.md` | 47 | 117 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/PROGRESS.md` | 31 | 181 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/RC_GATE_REPORT.md` | 50 | 140 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/REGRESSION_TEST_SUITE.md` | 69 | 221 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/RELEASE_GATE_CHECKLIST.md` | 41 | 195 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/RELEASE_NOTES.md` | 96 | 302 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/SECURITY_AUDIT.md` | 108 | 249 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/SOAK_PERFORMANCE_ANALYSIS.md` | 61 | 209 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/TEST_PLAN.md` | 43 | 143 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md` | 44 | 211 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/UPGRADE_GUIDE.md` | 33 | 207 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/coverage-baseline/README.md` | 30 | 148 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/perf/HASH_ANTI_JOIN_PERF.md` | 30 | 139 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/perf/Q5_MEMORY_ANALYSIS.md` | 28 | 161 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/perf/TPCH_PERFORMANCE_REPORT.md` | 25 | 117 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/plans/V311_ISSUES_PLAN.md` | 76 | 198 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.11.0/security/security-summary.md` | 40 | 107 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.12.0/CHANGELOG.md` | 133 | 413 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.12.0/DEVELOPMENT_PLAN.md` | 634 | 865 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md` | 125 | 314 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.12.0/ISSUES_PLAN.md` | 358 | 513 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.12.0/README.md` | 140 | 360 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.12.0/TEST_PLAN.md` | 408 | 685 | 中文主文 + 英文原文附录 |
| `docs/releases/v3.12.0/VERSION_PLAN.md` | 353 | 473 | 中文主文 + 英文原文附录 |
| `docs/releases/v4.0.0/CHANGELOG.md` | 77 | 108 | 中文主文 + 英文原文附录 |
| `docs/releases/v4.0.0/README.md` | 71 | 185 | 中文主文 + 英文原文附录 |
| `docs/releases/v4.0.0/TEST_PLAN.md` | 133 | 182 | 中文主文 + 英文原文附录 |
| `docs/releases/v4.0.0/VERSION_PLAN.md` | 222 | 237 | 中文主文 + 英文原文附录 |

## 3. 未改写或仅保留技术英文的文件

未发现主文英文占比明显高于中文、且未加英文原文附录的 Markdown 报告。

## 4. 后续规则

- 新增 release 文档默认使用中文主文。
- 如需保留英文，应放在 `## 附录：英文原文` 后。
- 不翻译命令、路径、crate/package 名、gate ID、协议名、SQL 关键字和代码标识。
- 英文历史附录中的 PASS/PENDING/TBD 不作为当前 gate 证据。
- 当前版本判断优先读取 `docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md`。
