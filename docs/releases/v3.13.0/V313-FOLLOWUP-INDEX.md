# v3.13 Follow-up Issue 索引

## Cluster A: GMP 治理 (#4225 + #4226)

| Issue | Title | Sprint | 当前 blocker |
|---|---|---|---|
| #4225 | V312-52 GMP vector/retrieval | SPRINT-S1 | production wiring 缺 |
| #4226 | V312-53 GMP compliance/access-control | SPRINT-S1 | ACL 5x12 全矩阵 + tamper 缺 |

## Cluster B: TPC-H 正确性 (#4221 + #4272 + #4273-#4279)

| Issue | Title | Sprint | 当前 blocker |
|---|---|---|---|
| #4221 | V312-48 TPC-H SF=1 总控 | SPRINT-S2 | /tmp/tpch-sf1 fixture 缺 |
| #4272 | V312-48-CROSS-ENGINE SHA256 | SPRINT-S2 | MySQL oracle + SF=1 |
| #4273 | V312-48-Q5 zero-row | SPRINT-S3 | planner reorder (nation-bridge 6-way) |
| #4274 | V312-48-Q8 zero-row | SPRINT-S3 | planner join order (8-way) |
| #4275 | V312-48-Q9 zero-row | SPRINT-S3 | planner predicate pushdown (6-way) |
| #4276 | V312-48-Q10 zero-row | SPRINT-S3 | group-by LIMIT (4-way) |
| #4277 | V312-48-Q13 zero-row | SPRINT-S3 | subquery decorrelation (NOT IN) |
| #4278 | V312-48-Q16 zero-row | SPRINT-S3 | subquery decorrelation (NOT IN) |
| #4279 | V312-48-Q18 zero-row | SPRINT-S3 | HAVING aggregate + LIMIT (3-way) |

## Cluster C: V312-56 4.0 前补强 (#4250-#4258)

| Issue | Title | Sprint | 当前 blocker |
|---|---|---|---|
| #4250 | V312-56 4.0 前补强总控 | SPRINT-S4 | meta,等 sub-issues |
| #4251 | V312-56.1 Metadata/SHOW | SPRINT-S4 | 教学 lab 缺 |
| #4252 | V312-56.2 | SPRINT-S4 | 教学 lab 缺 |
| #4253 | V312-56.3 | SPRINT-S4 | 教学 lab 缺 |
| #4254 | V312-56.4 | SPRINT-S4 | 教学 lab 缺 |
| #4255 | V312-56.5 | SPRINT-S4 | 教学 lab 缺 |
| #4256 | V312-56.6 | SPRINT-S4 | 教学 lab 缺 |
| #4257 | V312-56.7 | SPRINT-S4 | 教学 lab 缺 |
| #4258 | V312-56.8 | SPRINT-S4 | 教学 lab 缺 |

## Cluster D: Meta issues (#3887, #4220)

| Issue | Title | Sprint | 当前 blocker |
|---|---|---|---|
| #3887 | V312-MASTER 总控 | SPRINT-S5 | 等所有 sub-issue 关闭 |
| #4220 | V312-47 PARTIAL 总控 | SPRINT-S5 | 等所有 sub-issue 关闭 |

## Cluster E: Array-fraction (#4216)

| Issue | Title | Sprint | 当前 blocker |
|---|---|---|---|
| #4216 | quantile array-fraction | SPRINT-S6 | cross-engine SF=1 验证缺 |

## Cluster F: V312-19 stub(已 Round-24 reopen,部分嵌入 #4225)

见 V313-ROUND24-EVIDENCE-MANIFEST.md §3.1 提及 — 暂不在独立 sprint,合并到 Cluster A 处理。

sha256=<computed-at-commit-time>
