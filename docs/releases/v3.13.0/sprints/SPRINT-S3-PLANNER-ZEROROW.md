# SPRINT-S3: TPC-H zero-row Planner Fixes (Cluster B 子项)

## §1 Scope

闭合 #4273-#4279 (7 个 zero-row TPC-H query),逐 query 修复 planner reorder / predicate pushdown / subquery decorrelation,直到 sqlrustgo 行数 == PG oracle 行数。

## §2 Entry 标准

- [x] SPRINT-S0 完成 (SF=1 fixture + MySQL oracle 就位)
- [ ] SPRINT-S2 完成 (SQLite/PG oracle SF=1 行数已知)
- [x] V313-STRICT-CLOSE-STANDARDS.md 已合并

## §3 入口任务 (每个 Q 一个独立 PR)

| Issue | Query | 行数现状 (SF=0.001) | 根因 | 修复方向 |
|---|---|---|---|---|
| #4273 | Q5 | zero | planner 6-way join reorder 失败 | nation-bridge reorder: customer → orders → lineitem → supplier → nation → region |
| #4274 | Q8 | zero | planner 8-way join filter 丢失 | region filter pushdown 到 lineitem 路径 |
| #4275 | Q9 | zero | planner 6-way predicate pushdown 缺 | part.p_type LIKE 推过 join |
| #4276 | Q10 | zero | planner 4-way group-by LIMIT projection 错 | customer + orders + lineitem + nation projection reorder |
| #4277 | Q13 | zero | subquery NOT IN 不 decorrelate | anti-join decorrelation (subq_flatten) |
| #4278 | Q16 | zero | subquery NOT IN 不 decorrelate | anti-join decorrelation (supplier/parts NOT IN) |
| #4279 | Q18 | zero | HAVING aggregate + LIMIT 3-way projection 错 | orders + lineitem + customer HAVING sum(...) > 315.. |

### §3.N 每个 Q 任务结构 (统一模板)

1. **写 failing test** — `cargo test q<N>_zero_row_sf1` 期望行数 == PG oracle 行数
2. **跑 test 验证 fail** — 记录实际行数(应为 0)
3. **实现 planner fix**:
   - #4273/#4274 → `crates/planner/src/join_reorder.rs` 加 bushy/linear reorder 策略
   - #4275 → `crates/planner/src/predicate_pushdown.rs` 跨 join 推 p_type LIKE
   - #4276 → `crates/planner/src/group_by_limit.rs` 处理 HAVING + LIMIT projection
   - #4277/#4278 → `crates/planner/src/subquery_decorrelate.rs` 加 anti-join (NOT IN → NOT EXISTS)
   - #4279 → `crates/planner/src/having_limit.rs` 处理 3-way aggregate + LIMIT
4. **跑 test 验证 pass** — 行数 == PG oracle
5. **跑 cross-engine SHA-256** — 与 PG diff 应零差异 (允许 float-divergence)
6. **提交 + SHA-256 锚定** — `V313-S3-Q<N>-EVIDENCE.md`
7. **关闭对应 issue** — 用 V313-STRICT-CLOSE-STANDARDS.md §2 四要素

## §4 退出标准

- [ ] 7 个 issue 各自 merged PR 到 `develop/v3.13.0`
- [ ] `bash scripts/gate/check_v313_planner_gate.sh` exit=0
- [ ] 7 query 行数全部 == PG oracle 行数 (允许 ±1-3 行若 join order 不稳定)
- [ ] `evidence/V313-S3-PLANNER-VERIFICATION.md` 汇总

## §5 风险与依赖

| Risk | 影响 | Mitigation |
|---|---|---|
| planner reorder 工作量超 3 周 | S3 延期 | 每 Q 单 PR,前 2 Q 先收尾(复杂度梯度),后 5 Q 视进展调整 |
| anti-join decorrelation 触发现有测试回归 | 影响其他模块 | 单独 feature flag `#[cfg(feature = "subquery_decorrelate")]` 灰度 |
| PG oracle 行数与 sqlrustgo 行数差 ±N | false-positive fail | 容忍 ±1-3 行,记录偏差原因 |
| bushy reorder 搜索空间爆炸 | planner 慢 | 用 linear + greedy baseline,后续优化 |

## §6 依赖

- SPRINT-S0 ✅ (SF=1 fixture)
- SPRINT-S2 (行数 baseline;S3 开始必须就位)
- 后续:无 (S3 完成后为 SPRINT-S5 收尾铺路)

## §7 预估工作量

约 3-4 周 (planner reorder 是 v3.13 最大风险点,per `V313-MASTER-PLAN.md` §4)。

## §8 References

- 上游 DEFERRED 提交: `1da964be77` + `3d3d98c3a8` (per memory `v312-48-sub-issues-analysis.md`)
- Cluster B 索引: `V313-FOLLOWUP-INDEX.md` Cluster B
- 严格关闭标准: `V313-STRICT-CLOSE-STANDARDS.md`

## Evidence Hash

`sha256=6b2f9049093acf55d69bf76c77de20dffc1b70df4074d729740004b7ab314c32` (computed on file content at HEAD)