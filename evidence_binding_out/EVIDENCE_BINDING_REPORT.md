# vv3.7.0 证据绑定检查报告

> **检查日期**: 2026-05-30
> **版本**: v3.7.0
> **Auditor**: Hermes Agent (Anti-Fabrication Policy v1.0)
> **检查工具**: check_evidence_binding.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | 12 |
| 警告 | 59 |
| 失败（违规） | 163 |
| 未验证声明 | 158 |

**总结**: ❌ 发现 163 个违规（Type A/B/C/D）

---

## 违规详情（按类型分类）

### Type A: 虚构执行（Execution Fabrication）

无 CI 证据声明"测试通过 / 编译成功"

- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: | 测试 | 命令 | 通过标准 |
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | WAL integration | 16/16 PASS | - |
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | SQL parser | 32/32 PASS | - |
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: - Alpha Gate: 14/14 PASS
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: - 所有 integration tests PASS
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | Unit Tests | 93/93 PASS |
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | E2E | 28/28 PASS |
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | TPC-H SF=1 | 22/22 PASS |
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: | A1 | cargo build --release | ✅ PASS |
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: | A2 | cargo test (mysql-server --lib) | ✅ 93/93 PASS |
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | B1 | E2E integration | ✅ 28/28 PASS |
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: | B2 | TPC-H SF=1 | ✅ 22/22 PASS |
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: | G1 | GA_GAP_REPORT exists | ✅ PASS |
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：RELEASE_CANDIDATE_GATE.md
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据:         // 必须通过 WAL 进行持久化
- ❌ Type A/B 违规：第 7 行声明无 CI/gate/commit 证据: - **成功定义**: 所有 Beta Gate B1-B8 全部 PASS，治
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | P0-4 | Beta Gate B1-B8 全部 PASS | — | 治理改进 |
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | P1-2 | TPC-H SF=1 22/22 PASS | — | 测试补充 |
- ❌ Type A/B 违规：第 115 行声明无 CI/gate/commit 证据: 步骤 5: 验证所有 DML 测试通过
- ❌ Type A/B 违规：第 127 行声明无 CI/gate/commit 证据: | Alpha CONDITIONAL PASS 语义模糊 | 门禁形同虚设 |
- ❌ Type A/B 违规：第 146 行声明无 CI/gate/commit 证据: 当 Alpha Gate 报告中出现 CONDITIONAL PASS 时，必须
- ❌ Type A/B 违规：第 148 行声明无 CI/gate/commit 证据: 1. **所有 A1-A4 硬性指标必须 PASS**
- ❌ Type A/B 违规：第 149 行声明无 CI/gate/commit 证据:    - A1: Build (release) PASS
- ❌ Type A/B 违规：第 150 行声明无 CI/gate/commit 证据:    - A2: Test (lib) PASS
- ❌ Type A/B 违规：第 151 行声明无 CI/gate/commit 证据:    - A3: Clippy PASS
- ❌ Type A/B 违规：第 152 行声明无 CI/gate/commit 证据:    - A4: Format PASS
- ❌ Type A/B 违规：第 250 行声明无 CI/gate/commit 证据: PASS_COUNT=0
- ❌ Type A/B 违规：第 256 行声明无 CI/gate/commit 证据:     echo "PASS"
- ❌ Type A/B 违规：第 257 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 265 行声明无 CI/gate/commit 证据: PASS_RATE=$(echo "$RESULT" | grep -o '[0-9]\+%' | head -1 | 
- ❌ Type A/B 违规：第 266 行声明无 CI/gate/commit 证据: if [ "$PASS_RATE" -ge 90 ]; then
- ❌ Type A/B 违规：第 267 行声明无 CI/gate/commit 证据:     echo "PASS ($PASS_RATE%)"
- ❌ Type A/B 违规：第 268 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 270 行声明无 CI/gate/commit 证据:     echo "FAIL ($PASS_RATE%)"
- ❌ Type A/B 违规：第 276 行声明无 CI/gate/commit 证据:     echo "PASS"
- ❌ Type A/B 违规：第 277 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 285 行声明无 CI/gate/commit 证据:     echo "PASS"
- ❌ Type A/B 违规：第 286 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 295 行声明无 CI/gate/commit 证据:     echo "PASS ($COVERAGE%)"
- ❌ Type A/B 违规：第 296 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 304 行声明无 CI/gate/commit 证据:     echo "PASS"
- ❌ Type A/B 违规：第 305 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 313 行声明无 CI/gate/commit 证据:     echo "PASS"
- ❌ Type A/B 违规：第 314 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 323 行声明无 CI/gate/commit 证据:     echo "PASS ($SQL_RATE%)"
- ❌ Type A/B 违规：第 324 行声明无 CI/gate/commit 证据:     ((PASS_COUNT++))
- ❌ Type A/B 违规：第 331 行声明无 CI/gate/commit 证据: echo "PASS: $PASS_COUNT/$TOTAL_COUNT"
- ❌ Type A/B 违规：第 333 行声明无 CI/gate/commit 证据: if [ "$PASS_COUNT" -eq "$TOTAL_COUNT" ]; then
- ❌ Type A/B 违规：第 373 行声明无 CI/gate/commit 证据: | Alpha | develop/v3.7.0 分支 | A1-A5 PASS (Coverage >= 75
- ❌ Type A/B 违规：第 374 行声明无 CI/gate/commit 证据: | Beta | Alpha PASS + P0 issues | B1-B8 PASS | ❌ PENDING |
- ❌ Type A/B 违规：第 375 行声明无 CI/gate/commit 证据: | RC | Beta PASS | R1-R4 PASS | ❌ PENDING |
- ❌ Type A/B 违规：第 376 行声明无 CI/gate/commit 证据: | GA | RC PASS | 全部检查 PASS | ❌ PENDING |
- ❌ Type A/B 违规：第 383 行声明无 CI/gate/commit 证据: | 2026-06-15 | Alpha Gate | A1-A5 PASS |
- ❌ Type A/B 违规：第 384 行声明无 CI/gate/commit 证据: | 2026-07-01 | Beta Gate | B1-B8 PASS + mysql-server 修复 
- ❌ Type A/B 违规：第 385 行声明无 CI/gate/commit 证据: | 2026-07-15 | RC Gate | R1-R4 PASS |
- ❌ Type A/B 违规：第 386 行声明无 CI/gate/commit 证据: | 2026-07-30 | GA Release | 全部 PASS |
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | Alpha Gate | 14/14 PASS | ✅ DONE | v3.7.0-RC1 tag |
- ❌ Type A/B 违规：第 55 行声明无 CI/gate/commit 证据: | Unit Tests | 93/93 PASS |
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: | E2E Integration | 28/28 PASS |
- ❌ Type A/B 违规：第 57 行声明无 CI/gate/commit 证据: | TPC-H SF=1 | 22/22 PASS |
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: - [ ] Alpha Gate 通过（文档 + 门禁检查）
- ❌ Type A/B 违规：第 76 行声明无 CI/gate/commit 证据: - Alpha Gate: 14/14 PASS
- ❌ Type A/B 违规：第 240 行声明无 CI/gate/commit 证据: | `cargo test --lib -p sqlrustgo-mysql-server` | ✅ 93/93 P
- ❌ Type A/B 违规：第 241 行声明无 CI/gate/commit 证据: | `cargo test --lib -p sqlrustgo` | ✅ 12/12 PASS | Yes |
- ❌ Type A/B 违规：第 242 行声明无 CI/gate/commit 证据: | E2E: CREATE TABLE | ✅ PASS | Yes |
- ❌ Type A/B 违规：第 243 行声明无 CI/gate/commit 证据: | E2E: INSERT | ✅ PASS | Yes |
- ❌ Type A/B 违规：第 244 行声明无 CI/gate/commit 证据: | E2E: SELECT | ✅ PASS | Yes |
- ❌ Type A/B 违规：第 245 行声明无 CI/gate/commit 证据: | E2E: UPDATE | ✅ PASS | Yes |
- ❌ Type A/B 违规：第 246 行声明无 CI/gate/commit 证据: | E2E: DELETE | ✅ PASS | Yes |
- ❌ Type A/B 违规：第 247 行声明无 CI/gate/commit 证据: | E2E: BEGIN | ✅ PASS | Session-level persistence works |
- ❌ Type A/B 违规：第 248 行声明无 CI/gate/commit 证据: | E2E: COMMIT | ✅ PASS | Data persists after commit |
- ❌ Type A/B 违规：第 250 行声明无 CI/gate/commit 证据: | E2E: `mysql -u mysql -pmysql` | ✅ PASS | Auth working |
- ❌ Type A/B 违规：第 7 行声明无 CI/gate/commit 证据: > **标准**: v3.7.0 GA 必须通过以下全部门禁方可
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: | 代码质量 | ✅ | cargo test / clippy / fmt 全通过 |
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | A1 | cargo build --release | `cargo build --release -p sql
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: | A2 | cargo test --all-features | `cargo test --all-feature
- ❌ Type A/B 违规：第 32 行声明无 CI/gate/commit 证据: | A3 | clippy all features | `cargo clippy --all-features --
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | A4 | cargo fmt | `cargo fmt -- --check` | 0 failures | 0 f
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: | B1 | E2E integration tests | 28 files | 28/28 PASS | ✅ P
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: | B2 | TPC-H SF=1 | 22/22 queries | 22/22 PASS | ✅ PASS |
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: | B4 | Transaction correctness | BEGIN/INSERT/COMMIT persist
- ❌ Type A/B 违规：第 53 行声明无 CI/gate/commit 证据: | RC1 | Security: SKIP_AUTH | `grep SKIP_AUTH lib.rs` | `fal
- ❌ Type A/B 违规：第 54 行声明无 CI/gate/commit 证据: | RC2 | Performance baseline | TPC-H SF=1 Q1-Q22 | 22/22 PAS
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: | RC4 | E2E runtime | mysql-server 可用 | PASS | ✅ PASS 
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: | G1 | GA_GAP_REPORT exists | `ls GA_GAP_REPORT.md` | 存在
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: | G2 | GA Score >= 56/80 | Score calculation | 65/100 | ✅ 
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | G3 | P0 blockers closed | 2/2 fixed | 2/2 | ✅ PASS |
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: | G4 | LEGACY_ISSUES documented | v3.7.x + INT issues | 6 is
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: | G5 | v3.8.0 plan exists | DEVELOPMENT_PLAN.md | 存在 | �
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: | G6 | Changelog updated | CHANGELOG.md | 存在 | ✅ PASS 
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: | G7 | Release notes | RELEASE_NOTES.md | 存在 | ✅ PASS 
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：RELEASE_GATE_CHECKLIST.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：RELEASE_GATE_CHECKLIST.md
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: | MySQL Protocol | ✅ PASS | Auth (mysql/mysql working), CO
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: | CREATE TABLE | ✅ PASS | DDL executed successfully |
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: | INSERT + COMMIT | ✅ PASS | Transaction state persists pe
- ❌ Type A/B 违规：第 18 行声明无 CI/gate/commit 证据: | UPDATE + DELETE | ✅ PASS | WHERE clause filtering works 
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: | SELECT | ✅ PASS | Result set returned with column header
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | BEGIN + COMMIT | ✅ PASS | Session-level engine cache fix
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: | Auth | ✅ PASS | SKIP_AUTH=false (P0-2 fixed); mysql/mysq
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: **Result**: ✅ PASS
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: **Result**: ✅ PASS
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: **Result**: ✅ PASS
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据: **Result**: ✅ PASS
- ❌ Type A/B 违规：第 91 行声明无 CI/gate/commit 证据: **Result**: ✅ PASS
- ❌ Type A/B 违规：第 110 行声明无 CI/gate/commit 证据: **Result**: ✅ PASS
- ❌ Type A/B 违规：第 309 行声明无 CI/gate/commit 证据: | 2026-05-30 | Initial integration test report — DDL/DML P
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: | Alpha (L1 Compile + L2 Unit Tests) | ✅ PASS | 0 |
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: | Beta (Integration) | ✅ PASS | 0 |
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: | Evidence Graph Gate v4.1 | ✅ PASS | 0 |
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | **Overall** | ⚠️ **CONDITIONAL PASS** | 见 Section 6 
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: **Condition**: RC Gate CI 在 sqlrustgo-runner:v1 Docker 镜
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | Full workspace build | 0 errors | 构建成功 | ✅ PASS 
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | graph-cli build | 0 errors | 构建成功 | ✅ PASS |
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: | sqlrustgo (root) | cargo test --lib | 12 passed; 0 failed 
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: | sqlrustgo-executor | cargo test --lib -p sqlrustgo-executo
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: | sqlrustgo-storage | cargo test --lib -p sqlrustgo-storage 
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | sqlrustgo-parser | cargo test --lib -p sqlrustgo-parser | 
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: | evidence-graph | cargo test --lib -p evidence-graph | 5 pa
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: | **Total** | | **552 passed; 0 failed** | ✅ PASS |
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: | WAL verification | cargo test -p wal-verification | 100% P
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: | evidence-graph build | cargo build -p graph-cli --release 
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: | graph-cli binary | ./target/release/gate status | 正常�
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: **Note**: Beta Gate 已在 local 环境验证通过。CI 环
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: [alpha] R4: cargo test --all-features ... PASS
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: [alpha] R4: Integration tests 28 files ... PASS
- ❌ Type A/B 违规：第 84 行声明无 CI/gate/commit 证据: [alpha] A1: SQL Corpus >=85% ... PASS
- ❌ Type A/B 违规：第 105 行声明无 CI/gate/commit 证据: | TPC-H SF=0.1 baseline | 有 baseline 文件即 PASS | ⚠�
- ❌ Type A/B 违规：第 106 行声明无 CI/gate/commit 证据: | TPC-H SF=1 (22 queries) | 22/22 PASS | ⚠️ SKIP (需 my
- ❌ Type A/B 违规：第 108 行声明无 CI/gate/commit 证据: **Analysis**: TPC-H 测试需完整 mysql-server 环境，CI
- ❌ Type A/B 违规：第 119 行声明无 CI/gate/commit 证据: **Local Evidence**: Local 环境所有编译测试通过，�
- ❌ Type A/B 违规：第 129 行声明无 CI/gate/commit 证据: [PASS] VERSION_HISTORY.md current: v3.7.0
- ❌ Type A/B 违规：第 134 行声明无 CI/gate/commit 证据: [PASS] docs/releases/v3.7.0/CHANGELOG.md: includes v3.7.0
- ❌ Type A/B 违规：第 137 行声明无 CI/gate/commit 证据: [PASS] docs/releases/v3.5.0/CHANGELOG.md: no duplicates
- ❌ Type A/B 违规：第 138 行声明无 CI/gate/commit 证据: [PASS] docs/releases/v3.6.0/CHANGELOG.md: no duplicates
- ❌ Type A/B 违规：第 139 行声明无 CI/gate/commit 证据: [PASS] docs/releases/v3.7.0/CHANGELOG.md: no duplicates
- ❌ Type A/B 违规：第 141 行声明无 CI/gate/commit 证据: [PASS] docs/releases/v3.5.0/README.md: EXISTS
- ❌ Type A/B 违规：第 142 行声明无 CI/gate/commit 证据: [PASS] docs/releases/v3.6.0/README.md: EXISTS
- ❌ Type A/B 违规：第 144 行声明无 CI/gate/commit 证据: [PASS] docs/README.md: first current version is v3.7.0
- ❌ Type A/B 违规：第 151 行声明无 CI/gate/commit 证据: | Alpha | develop/v3.7.0 分支 | A1-A5 PASS (Coverage >= 75
- ❌ Type A/B 违规：第 152 行声明无 CI/gate/commit 证据: | Beta  | Alpha PASS + P0 issues | B1-B8 PASS | ❌ PENDING 
- ❌ Type A/B 违规：第 153 行声明无 CI/gate/commit 证据: | RC    | Beta PASS              | R1-R4 PASS | ❌ PENDING 
- ❌ Type A/B 违规：第 154 行声明无 CI/gate/commit 证据: | GA    | RC PASS                | 全部检查 PASS | ❌ P
- ❌ Type A/B 违规：第 191 行声明无 CI/gate/commit 证据: [9/10] Gate evaluate...           → {"result":"PASS","reas
- ❌ Type A/B 违规：第 194 行声明无 CI/gate/commit 证据: === Result === PASS
- ❌ Type A/B 违规：第 209 行声明无 CI/gate/commit 证据: | G-01: No evidence = UNVERIFIED | `./gate evaluate nonexist
- ❌ Type A/B 违规：第 210 行声明无 CI/gate/commit 证据: | G-02: Incomplete chain = UNVERIFIED | `./ingest task t1 "x
- ❌ Type A/B 违规：第 222 行声明无 CI/gate/commit 证据: **Evidence Graph Gate 结论**: ✅ PASS — Evidence chain 
- ❌ Type A/B 违规：第 241 行声明无 CI/gate/commit 证据: | evidence-graph-gate | completed | **failure** | Docker Car
- ❌ Type A/B 违规：第 253 行声明无 CI/gate/commit 证据: **Not a code issue**: Local build 成功，证明代码本�
- ❌ Type A/B 违规：第 292 行声明无 CI/gate/commit 证据: | Alpha (L1 Compile + L2 Unit Tests) | ✅ PASS | 无 |
- ❌ Type A/B 违规：第 293 行声明无 CI/gate/commit 证据: | Beta (Integration) | ✅ PASS | 无 |
- ❌ Type A/B 违规：第 294 行声明无 CI/gate/commit 证据: | RC (TPC-H + Legacy) | ⚠️ PARTIAL | CI 环境问题；L
- ❌ Type A/B 违规：第 296 行声明无 CI/gate/commit 证据: | Evidence Graph v4.1 | ✅ PASS | 无 |
- ❌ Type A/B 违规：第 310 行声明无 CI/gate/commit 证据: 1. **代码质量**: Alpha/Beta 所有测试 PASS（552 test
- ❌ Type A/B 违规：第 313 行声明无 CI/gate/commit 证据: 4. **CI 问题**: 是环境问题，非代码缺陷，Local 
- ❌ Type A/B 违规：第 318 行声明无 CI/gate/commit 证据: 3. 修复 evidence-graph-gate job 后确认 CI PASS
- ❌ Type A/B 违规：第 341 行声明无 CI/gate/commit 证据:   Note: Local evidence chain is complete and PASS
- ❌ Type A/B 违规：第 344 行声明无 CI/gate/commit 证据: Rule G-01: ✅ PASS — UNVERIFIED returned when no complete
- ❌ Type A/B 违规：第 345 行声明无 CI/gate/commit 证据: Rule G-02: ✅ PASS — CI node must have status=PASS for PA
- ❌ Type A/B 违规：第 346 行声明无 CI/gate/commit 证据: Rule G-03: ✅ PASS — No evidence = UNVERIFIED, blocks rel
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GA_GATE_REPORT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：GA_GATE_REPORT.md

---

### 警告项（需要人工复核）

- ⚠️ 文档可能缺少 provenance 元数据：TEST_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：CHANGELOG.md
- ⚠️ 文档可能缺少 provenance 元数据：INTEGRATION_STATUS.md
- ⚠️ 文档可能缺少 provenance 元数据：RELEASE_CANDIDATE_GATE.md
- ⚠️ 文档可能缺少 provenance 元数据：PHASE_1.1_PROPOSAL.md
- ⚠️ 警告：第 131 行 FAIL 声明可能无证据
- ⚠️ 警告：第 259 行 FAIL 声明可能无证据
- ⚠️ 警告：第 270 行 FAIL 声明可能无证据
- ⚠️ 警告：第 279 行 FAIL 声明可能无证据
- ⚠️ 警告：第 288 行 FAIL 声明可能无证据
- ⚠️ 警告：第 298 行 FAIL 声明可能无证据
- ⚠️ 警告：第 307 行 FAIL 声明可能无证据
- ⚠️ 警告：第 316 行 FAIL 声明可能无证据
- ⚠️ 警告：第 326 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：DEVELOPMENT_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：INTEGRATION_DEBT_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：ARCHITECTURE_VIOLATIONS.md
- ⚠️ 警告：第 79 行 FAIL 声明可能无证据
- ⚠️ 警告：第 81 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：RELEASE_NOTES.md
- ⚠️ 文档可能缺少 provenance 元数据：VERSION_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：PERFORMANCE_TARGETS.md
- ⚠️ 文档可能缺少 provenance 元数据：FEATURE_MATRIX.md
- ⚠️ 警告：第 249 行 FAIL 声明可能无证据
- ⚠️ 警告：第 251 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GA_GAP_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：RELEASE_GATE_CHECKLIST.md
- ⚠️ 文档可能缺少 provenance 元数据：INTEGRATION_READINESS_REPORT.md
- ⚠️ 警告：第 143 行 FAIL 声明可能无证据
- ⚠️ 警告：第 161 行 FAIL 声明可能无证据
- ⚠️ 警告：第 309 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：INTEGRATION_TEST_REPORT.md
- ⚠️ 警告：第 18 行 FAIL 声明可能无证据
- ⚠️ 警告：第 43 行 FAIL 声明可能无证据
- ⚠️ 警告：第 44 行 FAIL 声明可能无证据
- ⚠️ 警告：第 45 行 FAIL 声明可能无证据
- ⚠️ 警告：第 46 行 FAIL 声明可能无证据
- ⚠️ 警告：第 47 行 FAIL 声明可能无证据
- ⚠️ 警告：第 48 行 FAIL 声明可能无证据
- ⚠️ 警告：第 82 行 FAIL 声明可能无证据
- ⚠️ 警告：第 83 行 FAIL 声明可能无证据
- ⚠️ 警告：第 85 行 FAIL 声明可能无证据
- ⚠️ 警告：第 86 行 FAIL 声明可能无证据
- ⚠️ 警告：第 87 行 FAIL 声明可能无证据
- ⚠️ 警告：第 88 行 FAIL 声明可能无证据
- ⚠️ 警告：第 89 行 FAIL 声明可能无证据
- ⚠️ 警告：第 90 行 FAIL 声明可能无证据
- ⚠️ 警告：第 91 行 FAIL 声明可能无证据
- ⚠️ 警告：第 92 行 FAIL 声明可能无证据
- ⚠️ 警告：第 94 行 FAIL 声明可能无证据
- ⚠️ 警告：第 115 行 FAIL 声明可能无证据
- ⚠️ 警告：第 251 行 FAIL 声明可能无证据
- ⚠️ 警告：第 295 行 FAIL 声明可能无证据
- ⚠️ 警告：第 302 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GA_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：GATE_CHAOS_AUDIT.md
- ⚠️ 文档不存在（跳过检查）：ISSUE_AUDIT_AND_GAP_ANALYSIS_v3.7.0.md
- ⚠️ 警告：第 174 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：RELEASE_SUMMARY.md

---

### 通过项

- ✅ 无状态声明（无需证据检查）：RELEASE_CANDIDATE_GATE.md
- ✅ 状态声明有证据绑定：第 355 行
- ✅ 状态声明有证据绑定：第 364 行
- ✅ 状态声明有证据绑定：第 407 行
- ✅ 无状态声明（无需证据检查）：ARCHITECTURE_VIOLATIONS.md
- ✅ 无状态声明（无需证据检查）：PERFORMANCE_TARGETS.md
- ✅ 无状态声明（无需证据检查）：FEATURE_MATRIX.md
- ✅ 状态声明有证据绑定：第 71 行
- ✅ 无状态声明（无需证据检查）：INTEGRATION_READINESS_REPORT.md
- ✅ 状态声明有证据绑定：第 60 行
- ✅ 状态声明有证据绑定：第 211 行
- ✅ 状态声明有证据绑定：第 234 行

---

## 未验证声明（UNVERIFIED CLAIMS）

以下声明**无证据支撑**，不得用于门禁判断：

| 文档 | 行 | 声明内容 |
|------|-----|----------|
| - |  | 测试 | 命令 | 通过标准 | |
| - |  | WAL integration | 16/16 PASS | - | |
| - |  | SQL parser | 32/32 PASS | - | |
| - |  - Alpha Gate: 14/14 PASS |
| - |  - 所有 integration tests PASS |
| - |  | Unit Tests | 93/93 PASS | |
| - |  | E2E | 28/28 PASS | |
| - |  | TPC-H SF=1 | 22/22 PASS | |
| - |  | A1 | cargo build --release | ✅ PASS | |
| - |  | A2 | cargo test (mysql-server --lib) | ✅ 93/93 PASS | |
| - |  | B1 | E2E integration | ✅ 28/28 PASS | |
| - |  | B2 | TPC-H SF=1 | ✅ 22/22 PASS | |
| - |  | G1 | GA_GAP_REPORT exists | ✅ PASS | |
| - |          // 必须通过 WAL 进行持久化 |
| - |  - **成功定义**: 所有 Beta Gate B1-B8 全部 PASS，治理体系无漏洞 |
| - |  | P0-4 | Beta Gate B1-B8 全部 PASS | — | 治理改进 | |
| - |  | P1-2 | TPC-H SF=1 22/22 PASS | — | 测试补充 | |
| - |  步骤 5: 验证所有 DML 测试通过 |
| - |  | Alpha CONDITIONAL PASS 语义模糊 | 门禁形同虚设 | 🔴 CRITICAL | |
| - |  当 Alpha Gate 报告中出现 CONDITIONAL PASS 时，必须满足 |
| - |  1. **所有 A1-A4 硬性指标必须 PASS** |
| - |     - A1: Build (release) PASS |
| - |     - A2: Test (lib) PASS |
| - |     - A3: Clippy PASS |
| - |     - A4: Format PASS |
| - |  PASS_COUNT=0 |
| - |      echo "PASS" |
| - |      ((PASS_COUNT++)) |
| - |  PASS_RATE=$(echo "$RESULT" | grep -o '[0-9]\+%' | head -1 | tr -d '%') |
| - |  if [ "$PASS_RATE" -ge 90 ]; then |
| - |      echo "PASS ($PASS_RATE%)" |
| - |      ((PASS_COUNT++)) |
| - |      echo "FAIL ($PASS_RATE%)" |
| - |      echo "PASS" |
| - |      ((PASS_COUNT++)) |
| - |      echo "PASS" |
| - |      ((PASS_COUNT++)) |
| - |      echo "PASS ($COVERAGE%)" |
| - |      ((PASS_COUNT++)) |
| - |      echo "PASS" |
| - |      ((PASS_COUNT++)) |
| - |      echo "PASS" |
| - |      ((PASS_COUNT++)) |
| - |      echo "PASS ($SQL_RATE%)" |
| - |      ((PASS_COUNT++)) |
| - |  echo "PASS: $PASS_COUNT/$TOTAL_COUNT" |
| - |  if [ "$PASS_COUNT" -eq "$TOTAL_COUNT" ]; then |
| - |  | Alpha | develop/v3.7.0 分支 | A1-A5 PASS (Coverage >= 75%) | ❌ PENDING | |
| - |  | Beta | Alpha PASS + P0 issues | B1-B8 PASS | ❌ PENDING | |
| - |  | RC | Beta PASS | R1-R4 PASS | ❌ PENDING | |
| - |  | GA | RC PASS | 全部检查 PASS | ❌ PENDING | |
| - |  | 2026-06-15 | Alpha Gate | A1-A5 PASS | |
| - |  | 2026-07-01 | Beta Gate | B1-B8 PASS + mysql-server 修复 | |
| - |  | 2026-07-15 | RC Gate | R1-R4 PASS | |
| - |  | 2026-07-30 | GA Release | 全部 PASS | |
| - |  | Alpha Gate | 14/14 PASS | ✅ DONE | v3.7.0-RC1 tag | |
| - |  | Unit Tests | 93/93 PASS | |
| - |  | E2E Integration | 28/28 PASS | |
| - |  | TPC-H SF=1 | 22/22 PASS | |
| - |  - [ ] Alpha Gate 通过（文档 + 门禁检查） |
| - |  - Alpha Gate: 14/14 PASS |
| - |  | `cargo test --lib -p sqlrustgo-mysql-server` | ✅ 93/93 PASS | Yes | |
| - |  | `cargo test --lib -p sqlrustgo` | ✅ 12/12 PASS | Yes | |
| - |  | E2E: CREATE TABLE | ✅ PASS | Yes | |
| - |  | E2E: INSERT | ✅ PASS | Yes | |
| - |  | E2E: SELECT | ✅ PASS | Yes | |
| - |  | E2E: UPDATE | ✅ PASS | Yes | |
| - |  | E2E: DELETE | ✅ PASS | Yes | |
| - |  | E2E: BEGIN | ✅ PASS | Session-level persistence works | |
| - |  | E2E: COMMIT | ✅ PASS | Data persists after commit | |
| - |  | E2E: `mysql -u mysql -pmysql` | ✅ PASS | Auth working | |
| - |  > **标准**: v3.7.0 GA 必须通过以下全部门禁方可发布 |
| - |  | 代码质量 | ✅ | cargo test / clippy / fmt 全通过 | |
| - |  | A1 | cargo build --release | `cargo build --release -p sqlrustgo-mysql-server` | 0 errors | 构建成功 | ✅ PASS | |
| - |  | A2 | cargo test --all-features | `cargo test --all-features` | 0 failures | 93/93 PASS | ✅ PASS | |
| - |  | A3 | clippy all features | `cargo clippy --all-features -- -D warnings` | 0 errors | 0 errors | ✅ PASS | |
| - |  | A4 | cargo fmt | `cargo fmt -- --check` | 0 failures | 0 failures | ✅ PASS | |
| - |  | B1 | E2E integration tests | 28 files | 28/28 PASS | ✅ PASS | |
| - |  | B2 | TPC-H SF=1 | 22/22 queries | 22/22 PASS | ✅ PASS | |
| - |  | B4 | Transaction correctness | BEGIN/INSERT/COMMIT persistence | PASS | ✅ PASS | |
| - |  | RC1 | Security: SKIP_AUTH | `grep SKIP_AUTH lib.rs` | `false` | ✅ PASS | |
| - |  | RC2 | Performance baseline | TPC-H SF=1 Q1-Q22 | 22/22 PASS | ✅ PASS | |
| - |  | RC4 | E2E runtime | mysql-server 可用 | PASS | ✅ PASS | |
| - |  | G1 | GA_GAP_REPORT exists | `ls GA_GAP_REPORT.md` | 存在 | ✅ PASS | |
| - |  | G2 | GA Score >= 56/80 | Score calculation | 65/100 | ✅ PASS | |
| - |  | G3 | P0 blockers closed | 2/2 fixed | 2/2 | ✅ PASS | |
| - |  | G4 | LEGACY_ISSUES documented | v3.7.x + INT issues | 6 issues | ✅ PASS | |
| - |  | G5 | v3.8.0 plan exists | DEVELOPMENT_PLAN.md | 存在 | ✅ PASS | |
| - |  | G6 | Changelog updated | CHANGELOG.md | 存在 | ✅ PASS | |
| - |  | G7 | Release notes | RELEASE_NOTES.md | 存在 | ✅ PASS | |
| - |  | MySQL Protocol | ✅ PASS | Auth (mysql/mysql working), COM_QUERY, OK/Error packets | |
| - |  | CREATE TABLE | ✅ PASS | DDL executed successfully | |
| - |  | INSERT + COMMIT | ✅ PASS | Transaction state persists per session (P0-1 fixed) | |
| - |  | UPDATE + DELETE | ✅ PASS | WHERE clause filtering works | |
| - |  | SELECT | ✅ PASS | Result set returned with column headers | |
| - |  | BEGIN + COMMIT | ✅ PASS | Session-level engine cache fixed (P0-1) | |
| - |  | Auth | ✅ PASS | SKIP_AUTH=false (P0-2 fixed); mysql/mysql auth working | |
| - |  **Result**: ✅ PASS |
| - |  **Result**: ✅ PASS |
| - |  **Result**: ✅ PASS |
| - |  **Result**: ✅ PASS |
| - |  **Result**: ✅ PASS |
| - |  **Result**: ✅ PASS |
| - |  | 2026-05-30 | Initial integration test report — DDL/DML PASS, BEGIN/COMMIT FAIL, TransactionManager disconnected confirmed | |
| - |  | Alpha (L1 Compile + L2 Unit Tests) | ✅ PASS | 0 | |
| - |  | Beta (Integration) | ✅ PASS | 0 | |
| - |  | Evidence Graph Gate v4.1 | ✅ PASS | 0 | |
| - |  | **Overall** | ⚠️ **CONDITIONAL PASS** | 见 Section 6 | |
| - |  **Condition**: RC Gate CI 在 sqlrustgo-runner:v1 Docker 镜像内有 Cargo registry 权限问题，需修复后重新验证。Local 测试全部 PASS。 |
| - |  | Full workspace build | 0 errors | 构建成功 | ✅ PASS | |
| - |  | graph-cli build | 0 errors | 构建成功 | ✅ PASS | |
| - |  | sqlrustgo (root) | cargo test --lib | 12 passed; 0 failed | ✅ PASS | |
| - |  | sqlrustgo-executor | cargo test --lib -p sqlrustgo-executor | 256 passed; 0 failed | ✅ PASS | |
| - |  | sqlrustgo-storage | cargo test --lib -p sqlrustgo-storage | 181 passed; 0 failed | ✅ PASS | |
| - |  | sqlrustgo-parser | cargo test --lib -p sqlrustgo-parser | 98 passed; 0 failed | ✅ PASS | |
| - |  | evidence-graph | cargo test --lib -p evidence-graph | 5 passed; 0 failed | ✅ PASS | |
| - |  | **Total** | | **552 passed; 0 failed** | ✅ PASS | |
| - |  | WAL verification | cargo test -p wal-verification | 100% PASS | ✅ PASS | |
| - |  | evidence-graph build | cargo build -p graph-cli --release | 构建成功 | ✅ PASS | |
| - |  | graph-cli binary | ./target/release/gate status | 正常运行 | ✅ PASS | |
| - |  **Note**: Beta Gate 已在 local 环境验证通过。CI 环境因 Docker 镜像问题未能执行。 |
| - |  [alpha] R4: cargo test --all-features ... PASS |
| - |  [alpha] R4: Integration tests 28 files ... PASS |
| - |  [alpha] A1: SQL Corpus >=85% ... PASS |
| - |  | TPC-H SF=0.1 baseline | 有 baseline 文件即 PASS | ⚠️ SKIP (无 baseline 文件需先运行 cargo bench) | |
| - |  | TPC-H SF=1 (22 queries) | 22/22 PASS | ⚠️ SKIP (需 mysql-server 运行环境) | |
| - |  **Analysis**: TPC-H 测试需完整 mysql-server 环境，CI 环境缺失。Local 编译通过说明核心逻辑完整。 |
| - |  **Local Evidence**: Local 环境所有编译测试通过，证明代码本身无问题。 |
| - |  [PASS] VERSION_HISTORY.md current: v3.7.0 |
| - |  [PASS] docs/releases/v3.7.0/CHANGELOG.md: includes v3.7.0 |
| - |  [PASS] docs/releases/v3.5.0/CHANGELOG.md: no duplicates |
| - |  [PASS] docs/releases/v3.6.0/CHANGELOG.md: no duplicates |
| - |  [PASS] docs/releases/v3.7.0/CHANGELOG.md: no duplicates |
| - |  [PASS] docs/releases/v3.5.0/README.md: EXISTS |
| - |  [PASS] docs/releases/v3.6.0/README.md: EXISTS |
| - |  [PASS] docs/README.md: first current version is v3.7.0 |
| - |  | Alpha | develop/v3.7.0 分支 | A1-A5 PASS (Coverage >= 75%) | ❌ PENDING | |
| - |  | Beta  | Alpha PASS + P0 issues | B1-B8 PASS | ❌ PENDING | |
| - |  | RC    | Beta PASS              | R1-R4 PASS | ❌ PENDING | |
| - |  | GA    | RC PASS                | 全部检查 PASS | ❌ PENDING | |
| - |  [9/10] Gate evaluate...           → {"result":"PASS","reason":"reachability","missing":[]} |
| - |  === Result === PASS |
| - |  | G-01: No evidence = UNVERIFIED | `./gate evaluate nonexistent --db /tmp/eg.db` | UNVERIFIED | UNVERIFIED, reason: task_not_in_graph | ✅ PASS | |
| - |  | G-02: Incomplete chain = UNVERIFIED | `./ingest task t1 "x" --db /tmp/eg.db && ./gate evaluate t1 --db /tmp/eg.db` | UNVERIFIED | UNVERIFIED, reason: incomplete_evidence_chain | ✅ PASS | |
| - |  **Evidence Graph Gate 结论**: ✅ PASS — Evidence chain 完整，Rule G-01/02/03 强制执行，无 orphan 节点。 |
| - |  | evidence-graph-gate | completed | **failure** | Docker Cargo registry permission denied | |
| - |  **Not a code issue**: Local build 成功，证明代码本身无问题。是 Docker 镜像 sqlrustgo-runner:v1 的 `/.cargo/` 权限问题。 |
| - |  | Alpha (L1 Compile + L2 Unit Tests) | ✅ PASS | 无 | |
| - |  | Beta (Integration) | ✅ PASS | 无 | |
| - |  | RC (TPC-H + Legacy) | ⚠️ PARTIAL | CI 环境问题；Local 测试通过 | |
| - |  | Evidence Graph v4.1 | ✅ PASS | 无 | |
| - |  1. **代码质量**: Alpha/Beta 所有测试 PASS（552 tests，0 failures） |
| - |  4. **CI 问题**: 是环境问题，非代码缺陷，Local 验证已通过 |
| - |  3. 修复 evidence-graph-gate job 后确认 CI PASS |
| - |    Note: Local evidence chain is complete and PASS |
| - |  Rule G-01: ✅ PASS — UNVERIFIED returned when no complete CI chain |
| - |  Rule G-02: ✅ PASS — CI node must have status=PASS for PASS result |
| - |  Rule G-03: ✅ PASS — No evidence = UNVERIFIED, blocks release |

**规则**: UnverifiedDoc 不得用于门禁判断

---

## Anti-Fabrication Policy 合规状态

| 要求 | 状态 |
|------|------|
| PASS/FAIL 声明绑定 CI 证据 | ❌ 违规 |
| 门禁结果绑定 gate_policy_eval_id | ❌ 违规 |
| 计划文档无 GA Final 伪造 | ❌ 违规 |
| provenance 元数据存在 | ⚠️  部分缺失 |

---

## 后续行动

### 必须执行的修复

1. **立即停止**当前门禁流程
2. **回退**到上一个 VerifiedDoc 状态
3. **补充**真实证据（CI run ID + log hash）
4. **重新**执行门禁检查

### 问责记录

- 违规次数: 163
- 违规类型: Type A（虚构执行）/ Type B（伪门禁）/ Type D（伪任务完成）
- 处理方式: 触发 Anti-Fabrication Policy 问责机制


---

*报告生成时间: 2026-05-30 21:45:00*
*检查工具版本: check_evidence_binding.sh v1.0.0*
*依据政策: Anti-Fabrication Policy v1.0.0*

---

## 参考：违规类型定义

| 类型 | 定义 | 严重程度 |
|------|------|----------|
| **Type A** | 虚构执行：AI 声称测试通过但无 CI 日志支撑 | P0 |
| **Type B** | 伪门禁：AI 生成门禁通过但无 gate engine 输出 | P0 |
| **Type C** | 伪证据：AI 引用不存在的 CI run / log hash | P1 |
| **Type D** | 伪任务完成：AI 标记任务完成但代码未合并 | P1 |
