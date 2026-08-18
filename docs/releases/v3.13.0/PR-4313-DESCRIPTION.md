# v3.13-MASTER 总控 + 治理框架入口 (Issue #4313)

## Scope

本 PR 把 Issue #4313 的执行框架落地:
- `V313-STRICT-CLOSE-STANDARDS.md` (Round-24 治理规则)
- `V313-FOLLOWUP-INDEX.md` (24 sub-issue 索引)
- `V313-MASTER-PLAN.md` (scope + sprint 顺序 + risk + expiry)
- 6 个 sprint sub-plan (SPRINT-S1 ~ SPRINT-S6)
- `docs/releases/v3.13.0/` 目录结构 (README.md + sprints/ + evidence/ + gates/)

SPRINT-S0 (blocker 移除) 已先合并到 develop/v3.13.0:
- Alpha Quality 7/7 PASS (`3a6f32f78a`)
- TPC-H SF=1 fixture SHA-256 锚定 (`e45f57007e`)
- MySQL oracle 22/22 SHA-256 (`0c752ebb38`)
- 治理入口完整 (`314cc35afe`)

## 关联

- Closes: #4313 (部分 — 仅 setup,实际关闭等 SPRINT-S5)
- Refs: #3887 #4216 #4220 #4221 #4225 #4226 #4250-#4258 #4272-#4279

## 治理标准(强制)

每条关闭 PR 必须满足 `V313-STRICT-CLOSE-STANDARDS.md` §2 四要素:
1. 命令 (`bash scripts/gate/<name>.sh` 或 `cargo test ...`)
2. 退出码 (0 = PASS,1 = FAIL/BLOCKED,>1 = ERROR)
3. 输出摘要 (精炼到 `PASS:X/Y · BLOCKERS:Z` 格式)
4. 证据哈希 (SHA-256 64-char hex)

## 启动顺序

SPRINT-S0 (blocker, ✅ DONE) → SPRINT-S1 + S2 (并行) → SPRINT-S3 (等 S2) → SPRINT-S4 (并行 S1-S4) → SPRINT-S5 (收尾) → SPRINT-S6 (最后清理)

详细见 `V313-MASTER-PLAN.md` §3。

## 阻塞路径(诚实披露)

v3.12.0 当前状态:internal controlled subset (per V312-ROUND24-REMEDIATION-NOTICE.md)。
v3.13.0 启动已完成 S0 blocker 全部移除:
- [x] sqlrustgo-mysql-client 编译失败 (per §2.2) — 修复 commit `0b9c01d142` cascade
- [x] `/tmp/tpch-sf1` dbgen fixture 缺失 — 已生成,8/8 表行数匹配
- [x] MySQL oracle 未配置 — 已配置,22/22 query SHA-256 锚定

后续 blocker (S1-S6 各自):
- S1: GMP production wiring 工作量 (deterministic fixture + ACL 60 case)
- S2: SQLite/PG oracle 升 SF=1 (当前 SF=0.001)
- S3: planner reorder + subquery decorrelation 工作量
- S4: 8 个教学 lab 文档 + gate 脚本
- S5: 收尾卷宗
- S6: array-fraction cross-engine 验证

## 提交列表

本 PR 包含 14 个 commit (从 `9554dee2bb` 到 `63d01aa9bb`):

| Commit | 描述 |
|---|---|
| `9554dee2bb` | chore(v3.13.0): bootstrap develop/v3.13.0 branch from v3.12.0 HEAD |
| `17d85de24a` | docs(v3.13.0): initialize v3.13-MASTER doc skeleton |
| `2cbd25451c` | docs(v3.13.0): add .gitkeep files so v3.13.0 subdirs are tracked |
| `a1940a9fdc` | docs(v3.13.0): add V313 strict close standards (Round-24 governance) |
| `0190762b71` | docs(v3.13.0): add 24 sub-issue follow-up index (cluster-grouped) |
| `b8ea6fc1d3` | docs(v3.13.0): fix title to 23 sub-issue + add audit note (T2 review fixes) |
| `c2d7d27239` | docs(v3.13.0): V313-MASTER 总控 plan (scope + sprints + risk + expiry) — *orphaned by T3 fix* |
| `53492f02f1` | docs(v3.13.0): use SHA-256 placeholder per T3 constraint |
| `1f66f22bc3` | docs(v3.13.0): T3 audit note documenting c2d7d27239 orphan + real SHA-256 |
| `3bdd85312f` | docs(v3.13.0): fix SHA-256 anchors in T1/T2/T3 governance docs |
| `3a6f32f78a` | docs(v3.13.0): add SPRINT-S0 Alpha Quality PASS evidence |
| `0c752ebb38` | evidence(v3.13.0): MySQL oracle SHA256 (22/22 @ SF=1) |
| `e45f57007e` | evidence(v3.13.0): tpch SF=1 dbgen fixture sha256 anchored |
| `314cc35afe` | docs(v3.13.0): SPRINT-S0 blockers complete |
| `436d08490f` | docs(v3.13.0): SPRINT-S1 GMP plan skeleton (#4225 + #4226) |
| `e7b623ff7f` | docs(v3.13.0): SPRINT-S2 TPC-H cross-engine plan (#4221 + #4272) |
| `7afa1970bd` | docs(v3.13.0): SPRINT-S3 TPC-H zero-row planner plan (#4273-#4279) |
| `5e389b4b68` | docs(v3.13.0): SPRINT-S4 V312-56 teaching lab plan (#4250-#4258) |
| `be0381deb4` | docs(v3.13.0): SPRINT-S5 meta closure plan (#3887 + #4220) |
| `63d01aa9bb` | docs(v3.13.0): SPRINT-S6 array-fraction quantile plan (#4216) |

## 测试计划

```bash
# 必须保持所有现有 gate PASS
bash scripts/gate/check_alpha_entry_v3.12.0.sh
bash scripts/gate/check_alpha_quality_v3.12.0.sh
cargo clippy --all-features -- -D warnings
cargo fmt --check --all
```

## 后续工作(不在本 PR)

SPRINT-S0 → SPRINT-S1 + S2 → SPRINT-S3 → SPRINT-S4 → SPRINT-S5 → SPRINT-S6
详细见 `sprints/SPRINT-SN-*.md`。

Issue #4313 expiry: 2027-06-30 (per V312 Round-24 remediation memory)。

## Anti-Fabrication-Policy-v1.0 声明

- 所有 SHA-256 hash 均为文件内容实测,无 placeholder (除 2 个已知 audit note 中提到的 orphan `c2d7d27239` 与 `486acbf1a2`)
- `V313-STRICT-CLOSE-STANDARDS.md` §3 禁用的所有关闭标记 (`ACCEPTED-WITH-BINDING-MANIFEST (not DONE)` / `SUBSTANTIALLY_COMPLETE` / `DEFERRED-without-tracking`) 在本 PR 中均未使用
- Deferral 项严格按 §4 绑定 (tracking issue + owner + expiry + closing boundary)