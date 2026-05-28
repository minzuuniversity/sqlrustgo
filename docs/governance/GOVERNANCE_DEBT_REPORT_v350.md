# SQLRustGo v3.5.0 Governance Debt Report

> **Generated**: 2026-05-29
> **Scope**: v3.5.0 GA Analysis for v3.6.0 Planning
> **Status**: Initial Findings (Revised)

---

## 1. Documentation Gaps

### 1.1 Incomplete Gate Checklist
- **File**: `docs/releases/v3.5.0/GA_GATE_CHECKLIST.md`
- **Issue**: All 35 check items marked as ⬜ (unchecked), including execution record showing "TODO"
- **Evidence**: 
  ```
  | 2026-05-24 | hermes-agent | 初始创建 | TODO |
  ```
- **Action Required**: Finalize checklist with actual pass/fail status

### 1.2 Missing Governance Audit for v3.5.0
- **Expected**: `docs/governance/audit/v3.5.0/` directory
- **Actual**: Directory does not exist
- **Available**: audit/v1.x through v3.3.0 only
- **Impact**: No formal audit trail for v3.5.0 release

### 1.3 Deprecated Branch Governance Document
- **File**: `BRANCH_GOVERNANCE.md` (root)
- **Status**: Marked deprecated, superset replaced by `docs/governance/RELEASE_LIFECYCLE.md`
- **Issue**: Document still present and may cause confusion
- **Action Required**: Either remove or clearly mark as obsolete in all references

---

## 2. Process Gaps

### 2.1 Open Exemptions Not Closed
- **File**: `docs/governance/GATE_EXEMPTIONS.md`
- **Open Items**:
  | ID | Version | Issue | Due Date |
  |----|---------|-------|----------|
  | EX-v320-001 | v3.2.0 | executor coverage 70.7% < 85% | v3.3.0 Alpha |
  | EX-v320-002 | v3.2.0 | MySQL Protocol handshake failure | v3.3.0 Alpha |
  | EX-v320-003 | v3.2.0 | TPC-H SF=1 data missing | v3.3.0 Alpha |

- **Issue**: These were due v3.3.0 Alpha but still show as 🔴 Open

### 2.2 Gate Script vs Spec Mismatch — 已解决
- **Spec** (`GATE_SPEC_MASTER.md` §2.4): Coverage measured with `cargo llvm-cov test --lib`
- **Actual RC/GA Practice**: `--tests` 优先，`--lib` fallback（综合方法）
- **Resolution**: RC_GATE_REPORT.md §覆盖率度量标准说明 记录了综合方法（commit aa830bcd）已确立为统一方法
- **Action**: 需更新 GATE_SPEC_MASTER.md §2.4 以反映实际使用的综合测量方法

---

## 3. Metric Inconsistencies

### 3.1 Coverage Measurement Flag — SPEC 待更新
| 文档 | 定义 | 状态 |
|------|------|------|
| GATE_SPEC_MASTER.md §2.4 | `--lib` only | ⚠️ 需更新 |
| RC_GATE_REPORT.md §覆盖率度量标准说明 | `--tests` + `--lib` fallback | ✅ 正确 |
| GA_GATE_REPORT.md | 综合方法 | ✅ 正确 |

> **说明**: `--lib only` 已被证明会忽略大量外部集成测试的覆盖贡献（最高 +6.30pp），综合方法更全面。GATE_SPEC_MASTER.md 尚未更新反映此变更。

### 3.2 GA_GATE_CHECKLIST vs gate_spec Threshold
- **GA_GATE_CHECKLIST.md**: Claims G5 threshold 82%
- **GATE_SPEC_MASTER.md**: Specifies GA threshold 85%
- **Discrepancy**: 82% vs 85% - GA_GATE_CHECKLIST is outdated

---

## 4. Outdated References

### 4.1 Old Version References in Governance Docs
| File | Old Reference | Context |
|------|---------------|---------|
| `GOVERNANCE_INDEX.md` | v3.1.0, v3.0.0 examples | Governance index |
| `GATE_SPEC_v310.md` | v3.1.0 GA Gate | Deprecated spec |
| `GATE_SPEC_v320.md` | v3.2.0 GA Gate | Deprecated spec |
| `GATE_PHASES_AND_TRACKING.md` | v3.0.0, v3.1.0 examples | Tracking examples |
| `BRANCH_GOVERNANCE.md` | v1.2.0, v1.x references | Deprecated document |

### 4.2 Release Lifecycle References
- `RELEASE_LIFECYCLE.md` shows `develop/v2.9.0` as example but current is `develop/v3.6.0`
- Branch naming convention appears updated but examples not refreshed

---

## 5. Compliance Issues

### 5.1 SSOT Violation — GATE_SPEC_MASTER.md Needs Update
- **GOVERNANCE_STANDARD.md** states GATE_SPEC_MASTER.md is SSOT for gate definitions
- **Issue**: GATE_SPEC_MASTER.md §2.4 still specifies `--lib` only, but comprehensive method (`--tests + --lib fallback`) was established in RC/GA
- **Action**: Update GATE_SPEC_MASTER.md §2.4 to document comprehensive measurement method

### 5.2 GA_GATE_CHECKLIST Incomplete
- **GA_GATE_REPORT.md** shows G6-G12 as unchecked (⬜)
- **But**: G6-G12 are AI/performance benchmarks not applicable to v3.5.0 GMP-focused release
- **Action**: Mark G6-G12 as N/A with explanation, or update checklist scope

### 5.3 Documentation Evidence Gap
- **GA_GATE_REPORT.md** states coverage method per crate but doesn't show exact commands
- **No link** to `artifacts/gate/v3.5.0/` directory evidence
- **Requirement**: GOVERNANCE_STANDARD.md §5.3 requires proof files with `tool_output` field

---

## 6. Recommendations for v3.6.0

### 6.1 Critical (Must Fix)
1. **Update GATE_SPEC_MASTER.md §2.4**: Document comprehensive coverage measurement (`--tests` + `--lib` fallback) as the official method
2. **Create audit/v3.5.0/ directory** with formal governance audit report
3. **Close or escalate EX-v320-*** exemptions** - 3+ versions overdue
4. **Finalize GA_GATE_CHECKLIST.md**: Fill in actual results and mark N/A items

### 6.2 High Priority
5. **Refresh branch examples** in RELEASE_LIFECYCLE.md to current versions
6. **Remove/deprecate BRANCH_GOVERNANCE.md** or ensure clear deprecation banner
7. **Add v3.5.0 governance audit entry** to audit/ directory
8. **Align GA_GATE_CHECKLIST thresholds** with GATE_SPEC_MASTER (85% not 82%)

### 6.3 Medium Priority
9. **Update GOVERNANCE_INDEX.md** with v3.5.0/v3.6.0 references
10. **Add evidence links** in GA_GATE_REPORT.md to artifacts/
11. **Create migration guide** from old branch model to current model

### 6.4 Governance Process Improvements
12. **Add exemption aging alert**: Auto-flag exemptions overdue >2 versions
13. **Script-spec sync check**: Run `check_gate_sync.sh` on every gate spec update
14. **Coverage command validation**: Verify comprehensive method in CI

---

## 7. Summary

| Category | Count | Severity |
|----------|-------|----------|
| Documentation Gaps | 3 | Medium |
| Process Gaps | 2 (1 resolved) | Medium |
| Metric Inconsistencies | 2 (1 resolved) | Medium |
| Outdated References | 2 | Low |
| Compliance Issues | 3 | Medium |
| **Total** | **12** (2 resolved) | — |

**Overall Assessment**: v3.5.0 governance documentation is functional. The coverage measurement issue has been resolved — the comprehensive method (`--tests + --lib fallback`) is correct and documented in RC_GATE_REPORT. Remaining debt is primarily in SPEC documentation updates and exemption tracking.

**Risk for v3.6.0**: If EX-v320-* exemptions remain open and similar pattern continues, technical debt compounds. Recommend dedicated sprint to close 3 open exemptions before v3.6.0 Beta.

---

*Report generated for v3.6.0 planning. See GOVERNANCE_STANDARD.md §7.3 for full compliance checklist.*
