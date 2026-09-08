# SQLRustGo v4.0.0 ROADMAP

> **版本**: v4.0.0
> **状态**: draft (2026-09-08)
> **起点**: `develop/v4.0.0` @ `9febebb255` (from `develop/v3.12.0` GA)
> **目标 GA 日期**: 待定
> **产品目标**: 生产级 SQL + Vector + Graph + GMP 多模型数据库
> **路线图范围**: 2026-Q4 — 2027-Q2 (6 个月)

---

## 1. 战略定位

v4.0.0 = **一等公民多模型数据库**:

- 关系行 (SQL rows)
- 向量记录 (Vector records)
- 图节点/边 (Graph nodes/edges)
- GMP audit event

四类数据 **必须共享同一 transaction boundary、WAL、backup/restore、access control、observability**。

v4.0.0 不是 v3.12.0 的 incremental update,而是 **多模型融合的 GA 节点**:

```
v3.12.0 (GA: 2026-09-04)             v4.0.0 (target)
  SQL + internal vector/graph          SQL + first-class vector + first-class graph + GMP audit
       ↓                                       ↓
  internal capability                     first-class subsystem
       ↓                                       ↓
  GMP/RAG-only claim                  production multi-model claim
```

---

## 2. 当前状态 (2026-09-08)

| 项 | 状态 |
|---|---|
| `develop/v4.0.0` | **刚创建** at `9febebb255` (= v3.12.0 GA HEAD) |
| `main` / `release/v3.12.0` | `9febebb255` (3 remote sync) |
| v3.12.0 GA report | `docs/releases/v3.12.0/GA_GATE_REPORT.md` |
| v4.0.0 VERSION_PLAN.md | 已存在 (2026-08-08 起草) |
| v4.0.0 TEST_PLAN.md | 已存在 (2026-08-08 起草) |
| v4.0.0 README.md | 已存在 |
| v4.0.0 LEGACY_ISSUES.md | **新创建** (2026-09-08) |
| v4.0.0 DEV_PLAN.md | **新创建** (2026-09-08) |
| v4.0.0 ROADMAP.md | **本文档** (2026-09-08) |
| sqlrustgo-graph crate | M1-M5 已合并到 develop/v3.12.0 (`feature/gmp-graph-publish-v2` PR #4851) |
| GMP-Platform 集成 | PR #207 打开 (blocked by self-approval) |

---

## 3. 阶段里程碑

### Phase 0 — Draft (2026-09-08 → 2026-09-30)

**目标**: v4.0.0 进入 draft phase 的所有 governance 工作就绪。

**活动**:

1. ✅ `develop/v4.0.0` 分支创建 (继承 v3.12.0 GA)
2. ✅ LEGACY_ISSUES.md (21 必修 + 12 caveat + 8 重新评估)
3. ✅ DEV_PLAN.md (本文档姊妹文件,8 WP + 依赖关系)
4. ✅ ROADMAP.md (本文档)
5. ✅ CHANGELOG.md 更新: 进入 draft
6. ⏳ **Gitea branch protection**: develop/v4.0.0 enable_push=false / enable_force_push=false (草稿期禁止破坏性 push)
7. ⏳ **GMP-Platform PR #207 合并**: 解决 self-approval blocker (临时 admin override 或 split PR)
9. ⏳ **CI gate v4.0.0 starter**: `scripts/gate/check_no_log_tbl_json.sh` (file extension + size gate)
10. ⏳ **Vector SQL 语法 spec**: parser 增加 `VECTOR(N,)` + vector index DDL
11. ⏳ **Graph crate (sqlrustgo-graph) 升级**: 第一等公民暴露 (V400-04) 的 query surface
12. ⏳ **milestone 创建**: Gitea 上 `v4.0.0` milestone (open state)

**退出证据**:

- `docs/releases/v4.0.0/CHANGELOG.md` 标记 draft phase
- `develop/v4.0.0` branch protection 已 enable
- 至少 3 个 `v4.0.0` issues 创建
- `scripts/gate/check_no_log_tbl_json.sh` 写完 + dry-run 通过

---

### Phase 1 — Alpha (2026-10 → 2026-11)

**目标**: 证明 v4.0.0 多模型核心可工作 (但未 production-ready)。

**WP 范围**:

- **V400-01**: vector column + vector index SQL syntax ✅ parser/executor E2E
- **V400-02**: WAL-backed vector storage + rebuild ✅ crash/rebuild tests
- **V400-08**: multi-model optimizer + metadata filter skeleton (P1)
- WP-A / WP-B / WP-D (v3.12.0 遗留必修,见 LEGACY_ISSUES.md §3)

**Alpha Gate (`v4.0.0-alpha`)**:

| Gate | 领域 | 阈值 |
|---|---|---|
| A-G1 | workspace build/test/fmt/clippy | 退出 0, 0 warning |
| A-G2 | vector SQL insert/search/delete | 100% pass |
| A-G3 | vector WAL recovery | crash → WAL replay 恢复 |
| A-G4 | sqlrustgo-graph WAL recovery | crash → WAL replay 恢复 |
| A-G5 | WP-A/B/D issues closed | ≥ 12 of 17 |
| A-G6 | file-size + extension gate | 0 log/tbl/json > 1MB |

**退出证据**:

- `docs/releases/v4.0.0/evidence/alpha_gate.json`
- tag: `alpha/v4.0.0`
- release/v3.12.0 GA 路径未破坏 (回归测试)

---

### Phase 2 — Beta (2026-12 → 2027-01)

**目标**: 证明 v4.0.0 多模型在 cross-model 场景下可工作。

**WP 范围**:

- **V400-03**: graph storage 全面化 (从 sqlrustgo-graph + sqlcc 嫁接)
- **V400-04**: graph query surface (Cypher subset)
- **V400-05**: cross-model transaction (SQL + vector + graph + audit 同一 boundary)
- WP-C / WP-E (v3.12.0 遗留必修,DML/integrity)

**Beta Gate (`v4.0.0-beta`)**:

| Gate | 领域 | 阈值 |
|---|---|---|
| B-G1 | alpha gate 全部复测 | 100% pass |
| B-G2 | graph traversal correctness | bounded path query 100% pass |
| B-G3 | cross-model transaction atomicity | commit/rollback all-or-nothing |
| B-G4 | audit event ALCOA+ chain | 168h dry-run 0 chain break |
| B-G5 | ACL permission bypass | bypass attempts fail closed |
| B-G6 | 24h mixed SOAK | 0 crash, no consistency break |

**退出证据**:

- `docs/releases/v4.0.0/evidence/beta_gate.json`
- `docs/releases/v4.0.0/evidence/24h_soak_report.md`
- tag: `beta/v4.0.0`

---

### Phase 3 — RC (2027-01 → 2027-02)

**目标**: 证明 v4.0.0 在受控 production 场景下可发布。

**WP 范围**:

- **V400-06**: unified backup/restore (SQL + vector + graph + audit)
- **V400-07**: unified ACL + audit (覆盖 SQL/vector/graph 路径)
- **V400-08 完成**: multi-model optimizer + metadata filter
- 关闭 v3.12.0 剩余 caveat (按需)

**RC Gate (`rc/v4.0.0`)**:

| Gate | 领域 | 阈值 |
|---|---|---|
| RC-G1 | beta gate 全部复测 | 100% pass |
| RC-G2 | full multi-model backup/restore equality | counts/hashes/indexes equal |
| RC-G3 | ACL + audit path coverage | SQL + vector + graph paths enforced |
| RC-G4 | 168h mixed multi-model SOAK | 0 crash, no consistency break |
| RC-G5 | 所有 GA-claim-caveat 重新评估 | 进入 WP 或显式维持 caveat |

**退出证据**:

- `docs/releases/v4.0.0/evidence/rc_gate.json`
- `docs/releases/v4.0.0/evidence/168h_soak_report.md`
- tag: `rc/v4.0.0`

---

### Phase 4 — GA (2027-02 → 2027-03)

**目标**: v4.0.0 GA 发布。

**GA Gate (`ga/v4.0.0`)**:

| Gate | 领域 | 阈值 |
|---|---|---|
| GA-G1 | RC gate 全部复测 | 100% pass |
| GA-G2 | signed-off reviewer-1 + reviewer-2 | both sign-off |
| GA-G3 | Anti-Pattern 扫描 | 0 violations |
| GA-G4 | README + CLAIM_DOWNGRADE_MANIFEST 一致 | audit verified |
| GA-G5 | security scan | 0 critical |
| GA-G6 | docs sync (README, VERSION_PLAN, TEST_PLAN, ROADMAP, LEGACY_ISSUES, CHANGELOG) | all match HEAD |

**发布动作**:

1. merge `develop/v4.0.0` → `main`
2. tag `v4.0.0`
3. branch `release/v4.0.0` + `ga/v4.0.0`
4. push 到 4 remote (.252, .250, gitcode, GitHub*)
5. **GitHub push 受限于历史 251MB server.log;必须在 Phase 0 完成 git filter-branch 或 LFS migration**
6. GA release notes

**退出证据**:

- `docs/releases/v4.0.0/GA_GATE_REPORT.md` (8 promotion_to_GA_requires item PASS)
- `docs/releases/v4.0.0/REVIEWER-1-SIGNOFF.md` + `REVIEWER-2-SIGNOFF.md`
- tag: `v4.0.0`

---

## 4. 工作包与依赖

v4.0.0 工作包 (沿用 VERSION_PLAN §4 但加入 v3.12.0 遗留 + 治理项):

| ID | 工作包 | Phase | Owner | Exit evidence |
|---|---|---|---|---|
| **V400-00** | log/tbl/json gate + .gitignore | Phase 0 | devops | gate script + CI failure |
| **V400-01** | vector SQL syntax (parser + executor) | Phase 1 | parser | parser/executor E2E |
| **V400-02** | WAL-backed vector storage + rebuild | Phase 1 | storage | crash/rebuild tests |
| **V400-03** | sqlrustgo-graph 升级为 first-class storage | Phase 2 | graph | storage + traversal tests |
| **V400-04** | graph query surface (Cypher subset) | Phase 2 | graph | bounded path query tests |
| **V400-05** | cross-model transaction semantics | Phase 2 | transaction | rollback + crash tests |
| **V400-06** | unified backup/restore | Phase 3 | ops | restore SQL/vector/graph/audit equality |
| **V400-07** | unified ACL + audit | Phase 3 | security | permission bypass fails closed |
| **V400-08** | multi-model optimizer + metadata filters | Phase 3 | optimizer | query plan evidence |
| **V400-09** | 168h multi-model SOAK | Phase 3 | ops | 168h SOAK report |
| **WP-A** | v3.12.0 parser 必修 (#4708 #4696 #4710 etc) | Phase 1 | parser | 17 issues closed |
| **WP-B** | v3.12.0 type/function 必修 (#4721 #4674 etc) | Phase 1 | types | 7 issues closed |
| **WP-C** | v3.12.0 DDL/integrity 必修 (#4652 #4672 #4682) | Phase 2 | storage | 6 issues closed |
| **WP-D** | v3.12.0 join/subquery 必修 (#4668 #4656 #4649 #4636) | Phase 1 | executor | 4 issues closed |
| **WP-E** | v3.12.0 transaction 必修 (#4847 #4626) | Phase 2 | transaction | 2 issues closed |
| **WP-F** | schema migration 必修 (#4848) | Phase 1 | storage | 1 issue closed |
| **WP-G** | type/comparison 必修 (#4846) | Phase 1 | types | 1 issue closed |
| **WP-H** | v3.13/defer 重新评估 (#4707 #4699 #4688 #4671 等) | Phase 2/3 | mixed | 8 issues triage 完成 |

**总工作包**: 18 个

---

## 5. 依赖图

```
Phase 0: V400-00
            │
Phase 1:   WP-A → WP-D → WP-F → WP-G → V400-01 → V400-02 → WP-B
                                                      │
                                              ┌───────┴───────┐
Phase 2:                                       ▼               ▼
                                         V400-03 (graph)  WP-C, WP-E
                                              │               │
                                              ▼               ▼
                                          V400-04         V400-05
                                                       (cross-model txn)
                                                          │
Phase 3:                                                  ▼
                                                  V400-06 + V400-07
                                                          │
                                                          ▼
                                                  V400-08 + WP-H
                                                          │
                                                          ▼
                                              V400-09 (168h SOAK)
                                                  │
Phase 4:                                          ▼
                                              GA gate (8 items)
```

---

## 6. 风险与缓解

| 风险 | 影响 | 概率 | 缓解 |
|---|---|---|---|
| GitHub push 受限于 251MB log | GA 范围缩减 | 高 | Phase 0 用 git filter-branch 重写 + LFS 或 .gitignore 排除 |
| cross-model transaction 性能 | 168h SOAK fail | 中 | Phase 2 早期做 micro-bench;defer 复杂 case 到 v4.1 |
| GMP-Platform PR #207 self-approval | GMP 集成延后 | 中 | Phase 0 增加 admin PR merge whitelist (openclaw + hermes-agent) |
| sqlrustgo-graph 升级破坏现有 caller | regression | 中 | Phase 2 早期 regression suite + GMP-Platform consumer test |
| 168h SOAK 暴露一致性 bug | GA 推迟 | 中 | Phase 2 24h 试,问题 early-detection |
| 多模型 optimizer 复杂度 | V400-08 推迟 | 中 | Phase 1 skeleton,Phase 3 完整 |

---

## 7. 同步矩阵 (4 remote)

每个 Phase 完成时,所有 remote 必须 sync:

| Remote | URL | Token | Phase 0 |
|---|---|---|---|
| .252 Gitea | http://192.168.0.252:3000 | openclaw PAT | ✅ done |
| .250 Gitea | http://192.168.0.250:3000 | openclaw PAT | ✅ done |
| gitcode | https://gitcode.com/BreavHeart/sqlrustgo.git | LFS | ✅ done (develop/v4.0.0) |
| GitHub | github.com/yinglichina8848/sqlrustgo-public | ssh | ❌ blocked by 251MB log |

Phase 4 GA 时 GitHub 必须 unblock (Phase 0 完成 git filter-branch + git LFS migration)。

---

## 8. 治理与审计

v4.0.0 沿用 v3.12.0 治理:

- `docs/governance/ANTI_FABRICATION_POLICY.md` — 强制 evidence-based 声明
- `docs/governance/adr/ADR-014-multi-ai-coordination.md` — multi-AI 协作
- `docs/governance/GATE_CONDITIONS.md` — gate pass 判据
- `docs/governance/BRANCH_GOVERNANCE.md` — 分支清理规则

新增 v4.0.0 治理:

- **V4.0.0 / file-extension rule**: `*.log`, `*.tbl`, `*.json` 不入 commit
- **V4.0.0 / file-size rule**: 单文件 < 100MB (GitHub 兼容)
- **V4.0.0 / evidence archive**: 大 evidence 转 `docs/releases/v4.0.0/evidence/.../*.json.gz` 或 external object storage

---

## 9. 关键日期

| 日期 | 里程碑 |
|---|---|
| 2026-09-08 | Phase 0 start (develop/v4.0.0 created) |
| 2026-09-30 | Phase 0 退出 (draft complete) |
| 2026-11-30 | Phase 1 退出 (Alpha) |
| 2027-01-31 | Phase 2 退出 (Beta) |
| 2027-02-28 | Phase 3 退出 (RC) |
| 2027-03-31 | Phase 4 GA |

(具体日期按 Phase 0 退出后调整)

---

## 10. 附录

### 10.1 与 GMP-Platform 的关系

GMP-Platform 是 v4.0.0 的 first consumer:

- GMP-Platform PR #207 (sqlrustgo-graph 集成) — Phase 0 必合并
- GMP-Platform `gmp-storage` 使用 sqlrustgo CypherEngine — Phase 2 必须 regression test
- GMP-Platform `gmp-graph` writer — Phase 2 必须 regression test

### 10.2 与 llama.cpp @ .252:8001 的关系

- 当前 `/v1/embeddings` 返回 501 — v4.0.0 vector embedding 必须支持
- 替代方案:sqlrustgo 内部自部署 embedding 模型 (Phase 1 评估)
- 或 llama.cpp restart with `--embeddings` flag (Phase 0 ops 任务)

### 10.3 文档同步矩阵

| 文档 | Phase 0 | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|---|---|---|---|---|---|
| README.md | 初始 | 更新 | 更新 | 更新 | GA |
| VERSION_PLAN.md | 沿用 | 更新 | 更新 | 更新 | GA |
| TEST_PLAN.md | 沿用 | 更新 | 更新 | 更新 | GA |
| ROADMAP.md | 本文 | 更新 | 更新 | 更新 | GA |
| LEGACY_ISSUES.md | 初始 | 更新 | 更新 | 更新 | 关闭 |
| CHANGELOG.md | draft | alpha | beta | rc | ga |
| DEV_PLAN.md | 初始 | 更新 | 更新 | 更新 | GA |
| GA_GATE_REPORT.md | - | - | - | - | 写 |
| 168h_soak_report.md | - | - | - | 写 | - |