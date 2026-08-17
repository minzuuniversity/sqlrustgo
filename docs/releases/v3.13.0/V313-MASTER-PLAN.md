# v3.13-MASTER 总控 Plan

## §1 范围

为 Issue #4313 提供 24 sub-issue 的执行顺序、依赖关系、过期路径(2027-06-30)。

## §2 Sprint 拆解

| Sprint | Cluster | 入口标准 | 退出标准 | 依赖 |
|---|---|---|---|---|
| SPRINT-S0 | Blocker 移除 | 启动 v3.13.0 | SF=1 fixture 可用 + Alpha Quality PASS + MySQL oracle 通 | 无 |
| SPRINT-S1 | GMP 治理 | S0 完成 | #4225 + #4226 production wiring + gate PASS | S0 |
| SPRINT-S2 | TPC-H Cross-engine | S0 完成 | #4221 + #4272 MySQL+sqlrustgo oracle PASS | S0 |
| SPRINT-S3 | TPC-H zero-row planner | S0 完成 | #4273-#4279 7 query 行数 == PG 行数 | S0, S2 |
| SPRINT-S4 | V312-56 teaching | S0 完成 | #4250-#4258 9 sub 教学 lab 完成 | S0 |
| SPRINT-S5 | Meta closure | S1+S2+S3+S4 完成 | #3887 + #4220 关闭 | S1-S4 |
| SPRINT-S6 | Array-fraction | S2 完成 | #4216 cross-engine PASS | S2 |

## §3 启动顺序(推荐)

SPRINT-S0(blocker) → SPRINT-S1 + S2(并行) → SPRINT-S3(等 S2) → SPRINT-S4(并行 S1-S4) → SPRINT-S5(收尾) → SPRINT-S6(最后清理)

## §4 Risk Register

| Risk | 影响 | Mitigation |
|---|---|---|
| SF=1 fixture 不在 sandbox(/tmp/tpch-sf1) | Block S2/S3 | 找 SF=0.001 替代 + 诚实披露 OR 生成 dbgen fixture |
| Alpha Quality 失败(sqlrustgo-mysql-client 编译) | Block S0 | 先修 PR #4318 cascade 残留 |
| GMP production wiring 工作量 | S1 可能延期 | 拆 sub-issue 单 PR(每 PR gate PASS) |
| planner reorder 工作量 | S3 可能延期 | 每 Q 单 PR(S3-N 拆 7 PR) |

## §5 Round-24 严格关闭路径

每个 issue 关闭必须满足 V313-STRICT-CLOSE-STANDARDS.md §2 四要素。

## §6 Expiry Path(2027-06-30)

若 2027-06-30 前未完成:
- Issue #4313 重新开 scope(可能需要 v3.14 deferral)
- 24 sub-issue 重新评估 closure 状态
- 任何 round-25+ re-review 都基于新 evidence,不允许复用 round-24 manifest

## Evidence Hash

`sha256=8a6bf2edc53c51e197820c994671c68cb925cb3d4b969146897a0eb7fde76462`
