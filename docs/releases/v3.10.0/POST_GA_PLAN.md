# v3.10.0 Post-GA Plan

**Status**: Draft
**Last Updated**: 2026-07-13
**Maintainer**: openclaw

---

## Purpose

Define the post-GA maintenance, support, and development roadmap for v3.10.0, including hotfix criteria for v3.10.1+ and transition planning for v3.11.0.

---

## Phase 1: GA Stabilization Window (GA + 14 days)

### Hotfix Criteria (v3.10.1 triggers)

Any of the following would warrant a v3.10.1 hotfix release within 14 days of GA:

| Severity | Criteria | Example |
|----------|----------|---------|
| **P0** | Data corruption or data loss | Incorrect MVCC rollback, wrong query results |
| **P0** | Crash on any production query pattern | SIGSEGV, panic in normal execution path |
| **P1** | Security vulnerability | Unauthenticated access, SQL injection bypass |
| **P1** | Regression > 10% on critical path | TPC-H Q1/Q6 throughput regression |
| **P2** | Wire protocol incompatibility | MySQL client connection failure |

### Monitoring Period

- **Week 1**: Daily health check of production deployment
- **Week 2**: Reduce to every-other-day check
- **After week 2**: Weekly check until v3.11.0

---

## Phase 2: v3.11.0 Planning (GA + 7 days → GA + 30 days)

### v3.11.0 Strategic Direction

Continuing the **MySQL 5.7 alternative** roadmap, v3.11.0 will focus on:
1. **F-XX ISOLATED integration** (9 items, ~960 LOC)
2. **SEM-3 ALTER TABLE RENAME/MODIFY** completion
3. **SEM-4 coverage ≥ 80%** closure
4. **Extension crates evaluation** (product decision required)
5. **TPC-H 22/22 CI** (V310-11a/b completion)
6. **#3792 Q4 相关子查询优化** (Hash Semi Join + 去相关，~140h)

### Schedule Outline

| Phase | Target | Description |
|-------|--------|-------------|
| DRAFT | GA + 7d | v3.11.0 planning, branch from main |
| ALPHA | GA + 14d | Feature development |
| BETA | GA + 21d | Feature freeze, integration |
| RC | GA + 28d | Release candidate |
| GA | GA + 30d | v3.11.0 release |

### Inherited Debt Items (from v3.10.0 → v3.11.0)

| Item | Type | Est. Effort | Priority |
|------|------|-------------|----------|
| F-03 GIS | NOT_IMPLEMENTED | TBD | P2 |
| F-30 SEQUENCE | NOT_IMPLEMENTED | 20h | P2 |
| F-36 列级权限 | NOT_IMPLEMENTED | 40h | P1 |
| SEM-3 ALTER TABLE RENAME/MODIFY | IN_PROGRESS | 20h | P0 |
| SEM-4 Coverage ≥ 80% | IN_PROGRESS | 40h | P1 |
| F-23/24/25/26/27/29/31/32/35 ISOLATED | ISOLATED | ~960 LOC | P1 |
| #3136 Arch sem debt (CONTAINS macro) | IN_PROGRESS | 8h | P1 |
| 8 extension crates (SCOPE_DEFERRED) | DEFERRED | TBD | P2 |
| **#3792 Q4 相关子查询加速** (NEW) | **INHERITED** | **40h** | **P1** |
| **Hash Semi Join / Anti Join 算子** (NEW) | **NOT_IMPLEMENTED** | **60h** | **P1** |
| **子查询去相关（subquery decorrelation）** (NEW) | **NOT_IMPLEMENTED** | **40h** | **P2** |

> **Issue #3792 新增债务：** v3.10.0 实测显示 Q4 相关子查询占 96% 总执行时间（450K orders × 3M lineitem = 1.35 万亿次 naive 比较）。v3.11+ 需实现 Hash Semi Join / Anti Join + 子查询去相关优化以充分释放并行潜力。详见 `PARALLEL_EXECUTOR_OPTIMIZATION.md` §8.1。

### v3.11.0 GitHub Milestone Creation (GA + 1 day)

To avoid "promises that erode trust", all v3.11.0 commitments from this release's documentation must be tracked as GitHub Issues under a formal Milestone before GA is declared complete.

**Action**: Create Milestone `v3.11.0` on the primary Gitea instance (250) immediately after `v3.10.0` tag.

**Required Issues** (create all 8):

| Issue | Title | Labels | Linked Document |
|-------|-------|--------|----------------|
| v3.11.0-M1 | F-03 GIS geography support | `feature`, `high-priority` | COMPREHENSIVE_ASSESSMENT_REPORT.md §6 |
| v3.11.0-M2 | F-30 SEQUENCE support | `feature` | COMPREHENSIVE_ASSESSMENT_REPORT.md §6 |
| v3.11.0-M3 | SEM-3 ALTER TABLE RENAME/MODIFY | `debt`, `p0` | POST_GA_PLAN.md §Inherited Debt |
| v3.11.0-M4 | SEM-4 coverage ≥ 80% per crate | `debt`, `testing` | GA_GATE_REPORT.md D4.3 |
| v3.11.0-M5 | F-XX ISOLATED 9 items integration | `feature`, `refactoring` | ISOLATED_MODULES.md |
| v3.11.0-M6 | WITH RECURSIVE CTE support | `feature` | COMPREHENSIVE_ASSESSMENT_REPORT.md §6 |
| v3.11.0-M7 | Window functions (ROW_NUMBER, RANK, etc.) | `feature` | COMPREHENSIVE_ASSESSMENT_REPORT.md §6 |
| v3.11.0-M8 | TPC-H SF=1 full 22/22 queries | `testing`, `performance` | PERF_BASELINE.md R8 |

**Verification command** (run post-milestone-creation):
```bash
# Verify all 8 issues exist and are in milestone
MILESTONE_ID=$(curl -s "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/milestones?state=open" | python3 -c "import sys,json; ms=[m for m in json.load(sys.stdin) if m['title']=='v3.11.0']; print(ms[0]['id'] if ms else 'NOT_FOUND')")
for i in M1 M2 M3 M4 M5 M6 M7 M8; do
  echo -n "Issue $i: "
  curl -s "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/issues?milestone=$MILESTONE_ID&labels=$i&state=all" | python3 -c "import sys,json; print('found' if json.load(sys.stdin) else 'MISSING')"
done
```

**Closing criteria**: Each issue must be referenced in the code change that resolves it. No issue may be closed without a corresponding code commit and test.
---

## Phase 3: Long-Term Maintenance (GA + 30 days onward)

### Support Commitment

| Version | Support Level | Duration |
|---------|--------------|----------|
| v3.10.0 | Full (critical + security fixes) | Until v3.11.0 GA |
| v3.10.x | Security-only | v3.11.0 GA + 90 days |
| v3.9.0 | Security-only (hotfix) | v3.11.0 GA |
| v3.8.x | End-of-life | — |

### Maintenance Tasks

- Binary signing key rotation (if applicable)
- Dependency vulnerability scanning (weekly `cargo audit`)
- CI/CD pipeline health monitoring
- Disk space management for TPC-H fixtures

---

## Phase 4: Infrastructure & Governance

### Branch Management Post-GA

| Branch | Action | Timing |
|--------|--------|--------|
| `main` | Active (GA releases only) | Post-GA |
| `develop/v3.10.0` | Archive (read-only) | Post-GA |
| `rc/v3.10.0` | Delete | Post-GA tag cut |
| `develop/v3.11.0` | Create from main | GA + 7d |

### Documentation Updates

- `docs/releases/v3.10.0/STAGE.yaml` → set `current_stage: "GA"`
- `docs/releases/v3.10.0/CHANGELOG.md` → add GA entry
- `docs/releases/v3.10.0/RELEASE_NOTES.md` → finalize GA version
- `docs/governance/STAGE_CONFIG.yaml` → update if framework changed
- Archive v3.10.0-specific plans to `docs/releases/v3.10.0/archive/`

---
### 社区采用风险

| 风险 | 可能性 | 影响 | 缓解策略 |
|------|--------|------|---------|
| 用户获取困难（新 DB 难以撼动 MySQL 习惯） | 高 | 中 | 强调零配置启动 + Docker 单行部署，对比 MySQL 10 分钟初始化 |
| ORM/工具兼容性出现 regression | 中 | 高 | 建立 CI 回归测试矩阵（ Sequelize, Prisma, mysql-connector-python）；每次发布前运行 |
| 文档不够导致用户流失 | 中 | 中 | 完善 README 和 MYSQL57_COMPARISON_REPORT.md；补充 5 分钟快速入门 |
| 贡献者招募困难 | 中 | 低 | 明确标注 good-first-issue；维护 CHANGELOG 可见度 |

### 市场定位风险

| 风险 | 可能性 | 影响 | 缓解策略 |
|------|--------|------|---------|
| MySQL 8.0 推出更轻量替代方案 | 低 | 高 | 差异化：v3.10.0 已针对嵌入式/IoT 优化；持续迭代避免被赶上 |
| 大厂推出"SQLite 替代"云数据库 | 中 | 中 | 强调本地部署能力（离线、低延迟）vs 云依赖 |
| 定位模糊（既不是 MySQL 替代也不够轻量） | 中 | 高 | 通过适用场景速查表（MYSQL57_COMPARISON_REPORT.md §8.3）明确边界，不夸大能力 |
| 功能承诺无法按时交付（侵蚀信任） | 中 | 高 | 所有 v3.11.0 承诺通过 GitHub Milestone 跟踪（见 §v3.11.0 GitHub Milestone Creation） |

## Resource Requirements

### For v3.10.0 GA Support

| Resource | Quantity | Purpose |
|----------|----------|---------|
| Z6G4 (Mac mini) | 1 | SOAK + E2E test runner |
| backup250 Gitea | 1 | Primary remote (until 252 recovers) |
| Disk (for TPC-H fixture) | 75GB+ | SF=1.0 baseline collection |

### For v3.11.0 Development

| Resource | Quantity | Purpose |
|----------|----------|---------|
| Z6G4 | 1 | Dev + test |
| CI runner | 1 | Automated gate checks |
| Disk | 150GB+ | Multiple SF fixtures + coverage data |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Gitea origin (252) stays down | High | Medium | backup250 as primary; migrate data when 252 recovers |
| Hotfix needed immediately post-GA | Low | High | v3.10.1 branch prepared ahead, CI warm |
| Insufficient coverage (SEM-4) delayed | Medium | Low | GA not blocked; tracked for v3.11.0 |
| F-XX integration scope creep | Medium | Medium | Strict scope control per ISOLATED_MODULES.md |
| Disk exhaustion on test runner | Medium | High | Monitor df, clean TPC-H fixtures after use |

---

## References

- `docs/releases/v3.10.0/EVIDENCE_STATUS.md` — current evidence tracking
- `docs/releases/v3.10.0/GA_RELEASE_TIMELINE.md` — GA timeline
- `docs/governance/STAGE_CONFIG.yaml` — stage framework
- `docs/governance/debt/debt-registry.yaml` — debt tracking
- `ISOLATED_MODULES.md` — isolated module catalog
