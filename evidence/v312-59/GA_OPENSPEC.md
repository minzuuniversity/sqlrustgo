# V312-59-D OpenSpec Analysis — promotion_to_GA_requires 8 项整改

> **provenance:** discovered_during=v312-59-d-ga-requires-8x,
> generated_by=claude-code v3.12.0,
> source_run=v3.12.0-ga-requires-gap-analysis,
> branch=fix/v312-59-d-ga-requires-8x @ TBD (will cut after this spec),
> issue=#4387, umbrella=#4383,
> policy=Anti-Fabrication-Policy-v1.0
> **related:** Issue #4387 (V312-59-D)

---

## 1. Spec 概述

按 Issue #4387 要求，逐一分析 `docs/releases/v3.12.0/STAGE.yaml` 第 107-116 行 8 项 `promotion_to_GA_requires` 的当前实现状态、缺口、整改方案与可执行性。

### 1.1 OpenSpec 协议

本 spec 采用以下结构：
- **Spec 编号**: GA-N（N=1..8）
- **现状 (Now)**: 当前实现/文档/脚本情况
- **缺口 (Gap)**: 与验收条件的差距
- **方案 (Plan)**: 具体落地路径
- **证据 (Evidence)**: commit hash + evidence_hash + log path
- **可执行性 (Feasibility)**: 立即可落 / 需条件 / 长期运行

### 1.2 反 deferral 边界

按 #4383 umbrella + #4385 anti-pattern:
- ❌ 任何 GA blocker **禁止**用 `expiry 2027-06-30` 或 `v3.13 follow-up` 推回
- ❌ 168h SOAK **禁止**「GA 之后再跑」论据
- ❌ GA_GATE_REPORT.md 必须附 **真实** evidence_hash（无 placeholder）
- ❌ `cargo audit` HIGH/CRITICAL **禁止** `RUSTSEC-XXX ignored` 一笔带过
- ❌ SOAK 必须 SQL + GMP ingest + retrieval + audit + backup/restore 5 类组合

---

## 2. GA-1: comprehensive GA gate script (聚合所有 gates)

### 2.1 Spec
**要求**: 新增 `scripts/gate/check_ga_v3.12.0.sh`，聚合 BETA gate (40/40) + RC requires 11/11 + GA requires 8/8 + thresholds_override 8/8。
**输出**: JSON `ga_gate_report.json` 含 pass/total/warn/blockers + 每项 evidence_hash。
**失败阻断**: 任一 FAIL 即 exit 1。

### 2.2 Now (现状)
- BETA gate 脚本 `scripts/gate/check_beta_v3.12.0.sh` 已存在 (40 checks, 38/40 PASS, 2 WARN promoted to check via PR #4395)
- RC/GA gates **不存在独立聚合脚本**
- thresholds_override 13 个 boolean 字段在 STAGE.yaml 中定义，**无独立验证**

### 2.3 Gap
- **缺失**: `scripts/gate/check_ga_v3.12.0.sh`
- **缺失**: `ga_gate_report.json` JSON 报告格式
- **缺失**: thresholds_override 13 项 boolean 验证逻辑

### 2.4 Plan
1. 新建 `scripts/gate/check_ga_v3.12.0.sh`:
   - 复用 `check_beta_v3.12.0.sh` 作为子项
   - 新增 GA-1..GA-8 验证调用
   - 新增 thresholds_override 13 项 boolean 验证
   - 输出 JSON `docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json`
2. JSON schema:
   ```json
   {
     "version": "v3.12.0",
     "generated_at": "2026-08-21TXX:XX:XX+08:00",
     "commit": "<HEAD>",
     "beta_gate": {"pass": 40, "warn": 0, "fail": 0, "evidence_hash": "..."},
     "rc_requires": {"pass": 11, "fail": 0, "items": [...]},
     "ga_requires": {"pass": 0, "fail": 8, "items": [...]},  // initially 0/8
     "thresholds_override": {"pass": 0, "total": 13},
     "overall": "BLOCKED"
   }
   ```
3. 集成到 `scripts/gate/check_full_gate_verification.sh`（如存在）

### 2.5 Feasibility: ✅ 立即可落
依赖: 无（其他 GA-N 项可异步完成）

---

## 3. GA-2: 168h mixed SOAK (SQL + GMP ingest + retrieval + audit + backup/restore)

### 3.1 Spec
**要求**: harness `tests/soak/v312_mixed_soak.rs` 或 `tests/soak/mixed_workload.py`
**工作负载**:
- 60% 标准 SQL (TPC-C 简化模型)
- 15% GMP ingest (`gmp-md` 周期增量导入)
- 15% GMP retrieval (SQL + vector 混合)
- 5% audit chain verification (周期性 hash-chain 校验)
- 5% backup/restore (每 24h)

**时长**: 168h 不间断
**通过条件**: 0 崩溃 / 0 数据不一致 / 内存增长 <10% / P99 延迟无 degradation
**报告**: `docs/releases/v3.12.0/evidence/v312-59/GA2_168H_SOAK_REPORT.md`
**evidence_hash**: SOAK log SHA-256 (前 16 字节)

### 3.2 Now (现状)
- 已存在 SOAK 脚手架：
  - `tests/integration/stress/soak_test.rs`
  - `tests/integration/stress/soak_test_harness.rs`
  - `tests/integration/stress/chaos_soak_test.rs`
  - `tests/integration/tpch/tpch_soak_test.rs`
  - `tests/integration/tpch/tpch_soak_qps.rs`
  - `tests/e2e/sqlrustgo_cli_soak_e2e_test.rs`
- **缺失**: 168h 7-day mixed workload harness (含 GMP ingest/retrieval/audit/backup/restore 5 类组合)
- **缺失**: harness 自动调度 24h 周期 backup/restore 触发

### 3.3 Gap
- **缺失**: `tests/soak/v312_mixed_soak.rs` 或 `tests/soak/mixed_workload.py`
- **缺失**: GMP ingest/retrieval/audit 在 SOAK 中的集成
- **缺失**: 168h 持续运行的真实执行 (≥7 days)

### 3.4 Plan
1. **Phase A (立即)** - SOAK harness scaffold:
   - 新建 `tests/soak/v312_mixed_soak.rs` 框架（含 5 类工作负载 dispatch）
   - 新建 `tests/soak/mixed_workload.py` (Python orchestrator，跨进程调度)
   - 配置文件 `tests/soak/mixed_workload_config.yaml` (workload distribution + duration + metrics)
   - 报告模板 `docs/releases/v3.12.0/evidence/v312-59/GA2_168H_SOAK_REPORT.md`
2. **Phase B (立即)** - 短期 dry-run (1h 可执行 demo):
   - 1h demo run 验证 harness 工作 + 报告模板
   - 输出 `docs/releases/v3.12.0/evidence/v312-59/GA2_DEMO_1H_REPORT.md`
3. **Phase C (持续集成)** - 168h 真实运行:
   - 启动 background task (172h 超时，留 4h buffer)
   - 每 24h checkpoint 一份 partial report
   - 最终生成 `GA2_168H_SOAK_REPORT.md`

### 3.5 Feasibility: ⏳ Scaffold 立即可落 + 168h 真实运行需 background
- Phase A/B: 立即可落（PR 范围）
- Phase C: 需 background process + 7+ days wait
- **妥协方案**: scaffold + 1h demo run 已可证明 harness 可工作；168h full run 由 CI/automation 启动，**不在本 PR 范围**
- **诚实记录**: PR body + GA2 报告都明确标注 "demo run completed, 168h full run scheduled via background job" + 提供 background job ID

---

## 4. GA-3: security scan (cargo audit + deny + secret scan)

### 4.1 Spec
**要求**: 新增 `scripts/gate/check_security_scan_v312.sh`，含：
- `cargo audit` 0 HIGH/CRITICAL (或每条 CVE 都有 Gitea issue 关联 + expiry)
- `cargo deny check` 0 errors (license + advisory + bans)
- 静态扫描: 无 hardcoded secret (regex 扫描 `crates/` + `tests/`)
- wire protocol 无 plaintext password (grep)

**报告**: `docs/releases/v3.12.0/evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md`

### 4.2 Now (现状)
- `cargo-audit 0.22.1` **已安装** (`/home/openclaw/.cargo/bin/cargo-audit`)
- `cargo-deny` **未安装** (workaround: 可用 `cargo audit` + 手动 license check)
- 现有 `scripts/gate/check_security.sh` 是 v1.0.0-rc1 版本（基于 grep critical/high 字数）
- **缺失**: v3.12.0 specific security scan + 真实 cargo audit 输出 + secret scan

### 4.3 Gap
- **缺失**: `scripts/gate/check_security_scan_v312.sh`
- **缺失**: hardcoded secret 正则扫描
- **缺失**: plaintext password scan (wire protocol)

### 4.4 Plan
1. 新建 `scripts/gate/check_security_scan_v312.sh`:
   ```bash
   # 1. cargo audit --json (输出 JSON 解析)
   # 2. 检查 HIGH/CRITICAL count == 0
   # 3. cargo deny (如果可用，否则 fallback to manual license check via cargo metadata)
   # 4. secret scan: regex (password|secret|api_key|token)\s*=\s*['"][^'"]+['"]
   # 5. plaintext password scan: wire protocol tests/
   # 6. 输出报告
   ```
2. 报告 `docs/releases/v3.12.0/evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md`:
   - cargo audit JSON output
   - CVE table (id, package, severity, status, gitea_issue if any, expiry)
   - secret scan results (0 matches expected)
   - plaintext scan results (0 matches expected)

### 4.5 Feasibility: ✅ 立即可落
依赖: cargo-audit (已装), cargo-deny (需安装或 fallback)

---

## 5. GA-4: SQLLogicTest selected targets

### 5.1 Spec
**要求**: 在 RC-5 基础上升级为 "selected targets" 集
- 必须 ≥100 个文件 / 每个 PASS 或有 issue 关联
**报告**: `docs/releases/v3.12.0/evidence/v312-59/GA4_SQLLOGICTEST_SELECTED_REPORT.md`
**gate**: `bash scripts/gate/check_sqllogictest_selected_v312.sh` PASS

### 5.2 Now (现状)
- `scripts/gate/check_sqllogictest_v312.sh` 已存在 (smoke 25/25 PASS per smoke-report.md)
- `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml` 有 16 个 registered exclusions (RC-5 范围)
- `docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json` 有 manifest
- **缺失**: selected targets ≥100 (当前 25)

### 5.3 Gap
- **缺失**: selected target set (≥100 files)
- **缺失**: `check_sqllogictest_selected_v312.sh` gate 脚本
- **缺失**: `GA4_SQLLOGICTEST_SELECTED_REPORT.md`

### 5.4 Plan
1. 扩展 `sqlite-corpus-manifest.json` 从 25 → ≥100 entries (curated per RC-5 baseline)
2. 新建 `scripts/gate/check_sqllogictest_selected_v312.sh`:
   - 读取 manifest，遍历 selected targets
   - 对每个 file: PASS 或有 issue 关联（v3.12/v3.13 issue #）才算合规
   - 输出 summary: total/100, pass/total, exclusions/total
3. 新建 `docs/releases/v3.12.0/evidence/v312-59/GA4_SQLLOGICTEST_SELECTED_REPORT.md`

### 5.5 Feasibility: ⏳ 需 RC-5 进度 + 扩展 manifest
- manifest 扩展可立即落（手动 curated）
- gate 脚本可立即落
- 真实运行 ≥100 files 需 corpus 在 CI 可访问（依赖 existing fixtures）

---

## 6. GA-5: TPC-H SF=1 zero-row gap

### 6.1 Spec
**要求**: 在 V312-58 #4374-#4382 关闭后
**报告**: `docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_ZERO_ROW_GAP_REPORT.md`
**必证明**: 22/22 query 全部 row_count 匹配 SQLite oracle + 0 ZERO_ROW + 0 TIMEOUT

### 6.2 Now (现状)
- 已有 V312-48 evidence (Q5/Q8/Q9/Q10/Q13/Q16/Q18/Q21 7x zero-row) + V312-48-TPCH-SF1-CORRECTNESS.md
- 已有 `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/` (cross-engine oracle)
- **缺失**: V312-58 #4374-#4382 7 个未解 query (Q2/Q7/Q11/Q12/Q17/Q20/Q22) closure

### 6.3 Gap
- **缺失**: V312-58 7 个 query closure (依赖其他 sub-issues)
- **缺失**: 22/22 全集 GA-level 报告

### 6.4 Plan
1. 等 V312-58 #4374-#4382 关闭 (依赖其他 sub-issues)
2. 重新运行 22/22 cross-engine oracle at SF=1
3. 报告 `GA5_TPCH_SF1_ZERO_ROW_GAP_REPORT.md`:
   - 22/22 query row_count vs SQLite oracle (MATCH/MISMATCH)
   - 0 ZERO_ROW assertion
   - 0 TIMEOUT assertion
   - 引用每个 sub-issue (#4374-#4382) closure evidence

### 6.5 Feasibility: ⏳ 依赖 V312-58 #4374-#4382 关闭
本 PR 不阻塞，但 GA_GATE_REPORT 关闭时此项必须 PASS

---

## 7. GA-6: wire + LOAD DATA + crash recovery + backup/restore + upgrade/downgrade

### 7.1 Spec
**要求**: 在 RC-3/RC-7/RC-8 基础上升级为 GA 级别
**必证明**: 0 个 wire protocol 用例 FAIL + 0 个 LOAD DATA 用例 FAIL + 0 个 recovery 用例 FAIL + 0 个 upgrade 用例 FAIL
**报告**: `docs/releases/v3.12.0/evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md`

### 7.2 Now (现状)
- `docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md` (wire + LOAD DATA)
- `docs/releases/v3.12.0/evidence/wire_load_data/V312-50-REPORT.md`
- `docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY.md` + RECHECK
- `scripts/gate/check_v312_13_wire_load_data.sh` (wire + LOAD DATA gate)
- `scripts/gate/check_v312_14_crash_recovery.sh` (crash recovery gate)
- `scripts/gate/check_upgrade_v310_v311.sh` + `check_p14_upgrade_test.sh`
- `tests/upgrade_chain_v3_6_to_v3_9_test.rs` + `upgrade_v310_v311_test.rs`

### 7.3 Gap
- **缺失**: GA-level 聚合报告 (当前 RC-level 分立)
- **缺失**: 0 FAIL assertion 跨所有 5 类

### 7.4 Plan
1. 新建 `scripts/gate/check_ga_wire_recovery_upgrade.sh`:
   - 调用所有 RC-level 报告脚本
   - 收集每个分类的 PASS/FAIL/SKIP count
   - 0 FAIL assertion
2. 新建 `docs/releases/v3.12.0/evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md`:
   - 5 类各 PASS/FAIL/SKIP 总数
   - 每类的 sub-gate 输出引用
   - 0 FAIL 全集证明

### 7.5 Feasibility: ✅ 升级 RC 报告可立即落

---

## 8. GA-7: docs links + consistency

### 8.1 Spec
**要求**: `scripts/gate/check_docs_links_v312.sh` + `check_docs_consistency_v312.sh` 升级
- 0 broken link + 0 stale relative path
- 跨 `docs/`、`README.md`、`RELEASE_NOTES.md`、`docs/releases/v3.12.0/*.md` 全扫
**报告**: `docs/releases/v3.12.0/evidence/v312-59/GA7_DOCS_LINKS_REPORT.md`

### 8.2 Now (现状)
- `scripts/gate/check_docs_links.sh` 已存在 (general)
- `scripts/gate/check_docs_consistency.sh` 已存在 (general)
- **缺失**: v3.12.0 specific version (覆盖 `docs/releases/v3.12.0/` + README + RELEASE_NOTES)

### 8.3 Gap
- **缺失**: v3.12.0 范围 docs links 验证
- **缺失**: v3.12.0 范围 docs consistency 验证
- **缺失**: GA7 报告

### 8.4 Plan
1. 新建 `scripts/gate/check_docs_links_v312.sh`:
   - 复用 check_docs_links.sh 核心
   - 扩展 scope: `docs/releases/v3.12.0/*.md` + `README.md` + `RELEASE_NOTES.md`
   - 输出 broken link count
2. 新建 `scripts/gate/check_docs_consistency_v312.sh`:
   - 复用 check_docs_consistency.sh 核心
   - 扩展 v3.12.0 scope
   - 输出 stale path count
3. 新建 `docs/releases/v3.12.0/evidence/v312-59/GA7_DOCS_LINKS_REPORT.md`

### 8.5 Feasibility: ✅ 立即可落

---

## 9. GA-8: GMP compliance matrix signed + GA_GATE_REPORT

### 9.1 Spec
**要求**:
- `docs/releases/v3.12.0/GA_GATE_REPORT.md` 新建
- 必包含: 8 项 GA requires 全部 ✓ + 每项 commit hash + evidence_hash + exit code + owner signature
- `docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md` 必须有 "Signed off by: openclaw (2026-09-XX)" 行

### 9.2 Now (现状)
- `docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md` 存在 (14 行 PLANNED 状态)
- `docs/releases/v3.12.0/GA_GATE_REPORT.md` **不存在**

### 9.3 Gap
- **缺失**: GA_GATE_REPORT.md
- **缺失**: GMP_COMPLIANCE_MATRIX.md 签字行

### 9.4 Plan
1. 升级 GMP_COMPLIANCE_MATRIX.md:
   - 14 控制项部分 PASS（基于现有 evidence）
   - 添加 "Signed off by: openclaw (2026-09-XX)" 行
   - 注: 签字日期可填 placeholder 如 2026-09-15 (GA 日期)，需明确说明
2. 新建 `docs/releases/v3.12.0/GA_GATE_REPORT.md`:
   - 8 项 GA-1..GA-8 表格（commit + evidence_hash + exit code + status）
   - Owner signature
   - Threshold override 13 项断言
   - 引用 Issue #4387 + umbrella #4383

### 9.5 Feasibility: ⏳ 依赖前 7 项 PASS
- GMP_COMPLIANCE_MATRIX 签字行可立即加 (partial sign-off)
- GA_GATE_REPORT 需等前 7 项 evidence 完整

---

## 10. 总体实施计划

### 10.1 PR 范围 (in-v3.12)
| PR 提交 | 内容 | 依赖 |
|---------|------|------|
| PR #4396 (本轮) | GA-1 (gate 脚本) + GA-3 (security scan) + GA-7 (docs v3.12) + GA-6 (升级) + GA-2 Phase A scaffold + GA-8 scaffold | 无 |
| PR #4397 (后续) | GA-2 Phase B 1h demo run | GA-2 Phase A |
| PR #4398 (后续) | GA-4 selected targets ≥100 | RC-5 进度 |
| PR #4399 (后续) | GA-5 TPC-H 22/22 全集 | V312-58 closure |
| PR #4400 (GA) | GA-8 完整签字 + GA_GATE_REPORT | 所有前 7 项 PASS |

### 10.2 当前 PR (本轮) 落地
**Branch**: `fix/v312-59-d-ga-requires-8x`
**Files**:
1. `scripts/gate/check_ga_v3.12.0.sh` (新增, GA-1 聚合)
2. `scripts/gate/check_security_scan_v312.sh` (新增, GA-3)
3. `scripts/gate/check_docs_links_v312.sh` (新增, GA-7)
4. `scripts/gate/check_docs_consistency_v312.sh` (新增, GA-7)
5. `scripts/gate/check_ga_wire_recovery_upgrade.sh` (新增, GA-6)
6. `tests/soak/v312_mixed_soak.rs` (新增, GA-2 scaffold)
7. `tests/soak/mixed_workload.py` (新增, GA-2 scaffold)
8. `tests/soak/mixed_workload_config.yaml` (新增, GA-2 config)
9. `docs/releases/v3.12.0/evidence/v312-59/GA_OPENSPEC.md` (本文件, OpenSpec 分析)
10. `docs/releases/v3.12.0/evidence/v312-59/GA3_SECURITY_SCAN_REPORT.md` (GA-3 报告)
11. `docs/releases/v3.12.0/evidence/v312-59/GA7_DOCS_LINKS_REPORT.md` (GA-7 报告)
12. `docs/releases/v3.12.0/evidence/v312-59/GA6_WIRE_RECOVERY_UPGRADE_REPORT.md` (GA-6 报告)
13. `docs/releases/v3.12.0/evidence/v312-59/GA2_DEMO_1H_REPORT.md` (GA-2 Phase B demo)
14. `docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md` (升级, GA-8 partial)
15. `docs/releases/v3.12.0/GA_GATE_REPORT.md` (新增, GA-8 scaffold)

### 10.3 验证
```bash
# GA-1 聚合
bash scripts/gate/check_ga_v3.12.0.sh  # 期望 0 FAIL on each enabled GA-N

# GA-3 security
bash scripts/gate/check_security_scan_v312.sh  # 0 HIGH/CRITICAL

# GA-6 wire/recovery/upgrade
bash scripts/gate/check_ga_wire_recovery_upgrade.sh  # 0 FAIL

# GA-7 docs
bash scripts/gate/check_docs_links_v312.sh  # 0 broken
bash scripts/gate/check_docs_consistency_v312.sh  # 0 stale

# GA-2 demo (1h)
python3 tests/soak/mixed_workload.py --duration 1h --config tests/soak/mixed_workload_config.yaml
# 输出 GA2_DEMO_1H_REPORT.md
```

### 10.4 strict-proof-mode 合规
- 每个 gate 脚本都有实际执行输出
- 每个 evidence 文件都有 sha256 verified
- 不使用 placeholder evidence_hash
- 不使用 `RUSTSEC ignored` workaround
- 168h SOAK 真实运行**不在本 PR 范围**（明确披露）+ scaffold 立即落

---

## 11. 出处
- issue: #4387
- umbrella: #4383
- parent: #4385 (V312-59-B 已 closure via PR #4395)
- related evidence: `evidence/v312-59/integration-test-baseline.txt` (V312-59-B)
- policy: Anti-Fabrication-Policy-v1.0
- related: [[v312-59-beta-gate-remediation]], [[v312-59b-closure]]