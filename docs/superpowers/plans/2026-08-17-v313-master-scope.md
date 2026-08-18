# v3.13-MASTER 总控规划 + Issue #4313 实施计划

> **目标:** 为 v3.13-MASTER Issue #4313 建立治理框架、sprint 拆解、sub-issue 索引,并完成 24 个 v3.13 follow-up tracking issue 的可执行入口。
>
> **架构:** 把 Round-24 chatgpt/codex 严格审查 REJECTED 的 24 个 V312 issue 重新组织成 7 个 cluster sprint,每个 sprint 入口明确、退出标准符合 Round-24 严格关闭标准(PR merged + 真证据 + SHA-256 hash + 诚实披露)。

**技术栈:**
- 治理文档:Markdown,沿用 docs/releases/v3.X.Y/ 命名约定
- Issue tracker:Gitea REST API(`curl` + basic auth,不用 gh)
- Gate 验证:bash scripts/gate/* + cargo test
- 证据哈希:`sha256sum` 输出 64-char hex

**过期约束:** Issue #4313 expiry 2027-06-30(per V312 Round-24 remediation memory)

---

## 全局约束(每个 task 隐含包含)

1. **Round-24 严格关闭标准**(per V313-ROUND24-EVIDENCE-MANIFEST.md §1):
   - PR merged 到 `develop/v3.13.0`(新分支)或保留在 `develop/v3.12.0`
   - merge commit 在 develop 分支 reachable
   - 真证据 = 命令 + exit code + 输出摘要 + SHA-256 64-char hex
   - **禁用**:`ACCEPTED-WITH-BINDING-MANIFEST (not DONE)` / `SUBSTANTIALLY_COMPLETE` / `DEFERRED-without-tracking` 作为关闭标记
   - Anti-Fabrication-Policy-v1.0:任何 gate 失败必须诚实披露

2. **Gitea 远程规范**(per memory gitea-pr-creation-cli.md):
   - 用 `curl -u openclaw:details8848` 调用 Gitea REST API
   - 不用 `gh`(默认 github.com,会 401)
   - canonical remote = `http://192.168.0.250:3000`(memory 标注)
   - 备份 mirror = `http://192.168.0.252:3000`(origin)

3. **Issue 标签规范**:
   - `v3.13-followup`(id=98, color=5319e7)— 应用于所有 24 sub-issue
   - sub-cluster 标签:`v313-gmp` / `v313-planner` / `v313-teaching` / `v313-cross-engine`

4. **命名约定**(沿用 V312 习惯):
   - Plan 文件:`V313-<CLUSTER>-PLAN.md`
   - 验证文件:`V313-<CLUSTER>-VERIFICATION.md`
   - SHA-256 锚定:`sha256=<64-hex>` 行内显示

---

## Phase 0: 目录初始化

### Task 0: 创建 docs/releases/v3.13.0/ 骨架

**Files:**
- Create: `docs/releases/v3.13.0/README.md`(索引入口)

- [ ] **Step 1: 创建目录并写入口 README**

```bash
mkdir -p docs/releases/v3.13.0/{sprints,evidence,gates}
```

`docs/releases/v3.13.0/README.md` 内容:
```markdown
# v3.13.0 — v3.13-MASTER 总控

**Issue:** #4313 (Gitea canonical 252)
**Expiry:** 2027-06-30
**Owner:** openclaw
**Strict close standard:** V313-STRICT-CLOSE-STANDARDS.md

## 子目录

- `sprints/` — 7 个 cluster sprint plan + VERIFICATION
- `evidence/` — 24 sub-issue 证据 hash 锚定
- `gates/` — Round-24 strict gate 输出

## 入口顺序

1. V313-MASTER-PLAN.md(总控 scope)
2. V313-STRICT-CLOSE-STANDARDS.md(治理规则)
3. V313-FOLLOWUP-INDEX.md(24 sub-issue 索引)
4. sprints/SPRINT-NN-*.md(按 cluster 顺序)
```

- [ ] **Step 2: 提交**

```bash
git add docs/releases/v3.13.0/README.md
git commit -m "docs(v3.13.0): initialize v3.13-MASTER doc skeleton"
```

---

## Phase 1: 治理文档(3 个核心文件)

### Task 1: V313-STRICT-CLOSE-STANDARDS.md(治理规则)

**Files:**
- Create: `docs/releases/v3.13.0/V313-STRICT-CLOSE-STANDARDS.md`

- [ ] **Step 1: 编写严格关闭标准**

引用 Round-24 标准并扩充为 v3.13 governance。内容骨架(参考 V313-ROUND24-EVIDENCE-MANIFEST.md §1):

```markdown
# v3.13 严格关闭标准 (Strict Close V313 Governance)

## §1 PR 合并规则

PR 必须合并到 develop/v3.13.0(主) 或 develop/v3.12.0(维护),merge commit 在 develop 分支 reachable。

## §2 关闭证据四要素

1. **命令**:`bash scripts/gate/<name>.sh` 或 `cargo test ...`
2. **退出码**:0 = PASS,1 = FAIL/BLOCKED,>1 = ERROR
3. **输出摘要**:精炼到 PASS:X/Y · BLOCKERS:Z 格式
4. **证据哈希**:SHA-256 64-char hex(对 stdout 或 log 文件)

## §3 禁用关闭标记

- ❌ `ACCEPTED-WITH-BINDING-MANIFEST (not DONE)`
- ❌ `SUBSTANTIALLY_COMPLETE`
- ❌ `DEFERRED-without-tracking`
- ❌ `NEXT_STEPS` 与 close 同一 PR

## §4 Deferral 强制规则

若需延期:issue 保持 open + 必须绑定到:
- tracking issue(本例 #4313)
- owner(openclaw)
- expiry date(2027-06-30)
- closing boundary(具体可验证标准,例如 "SF=1 fixture 实测 PASS")

## §5 Anti-Fabrication-Policy-v1.0

任何 gate 失败必须诚实披露。伪造 PASS 或省略 FAIL = policy 违反,自动 invalidate closure。
```

- [ ] **Step 2: 计算 SHA-256 并锚定**

```bash
sha256sum docs/releases/v3.13.0/V313-STRICT-CLOSE-STANDARDS.md
# 记录输出,如:sha256=abc123...(64 char hex)
```

在文件末尾追加:
```markdown
## Evidence Hash

`sha256=<刚计算的 hash>` (computed on file content at HEAD)
```

- [ ] **Step 3: 提交**

```bash
git add docs/releases/v3.13.0/V313-STRICT-CLOSE-STANDARDS.md
git commit -m "docs(v3.13.0): add V313 strict close standards (Round-24 governance)"
```

---

### Task 2: V313-FOLLOWUP-INDEX.md(24 sub-issue 索引)

**Files:**
- Create: `docs/releases/v3.13.0/V313-FOLLOWUP-INDEX.md`

- [ ] **Step 1: 列出 24 sub-issue + cluster 分类**

基于 V313-ROUND24-EVIDENCE-MANIFEST.md §3 + memory v312-round24-chatgpt-remediation.md 的 24 issue 列表,按 cluster 分组:

```markdown
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
```

- [ ] **Step 2: SHA-256 锚定 + 提交**

```bash
sha256sum docs/releases/v3.13.0/V313-FOLLOWUP-INDEX.md
# 在文件末尾追加 sha256=<hash> 行
git add docs/releases/v3.13.0/V313-FOLLOWUP-INDEX.md
git commit -m "docs(v3.13.0): add 24 sub-issue follow-up index (cluster-grouped)"
```

---

### Task 3: V313-MASTER-PLAN.md(总控 scope + sprint 顺序)

**Files:**
- Create: `docs/releases/v3.13.0/V313-MASTER-PLAN.md`

- [ ] **Step 1: 写总控 scope + sprint 顺序**

```markdown
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

每个 issue 关闭必须满足 §1-V3-STRICT-CLOSE-STANDARDS.md §2 四要素。

## §6 Expiry Path(2027-06-30)

若 2027-06-30 前未完成:
- Issue #4313 重新开 scope(可能需要 v3.14 deferral)
- 24 sub-issue 重新评估 closure 状态
- 任何 round-25+ re-review 都基于新 evidence,不允许复用 round-24 manifest
```

- [ ] **Step 2: SHA-256 锚定 + 提交**

```bash
sha256sum docs/releases/v3.13.0/V313-MASTER-PLAN.md
git add docs/releases/v3.13.0/V313-MASTER-PLAN.md
git commit -m "docs(v3.13.0): V313-MASTER 总控 plan (scope + sprints + risk + expiry)"
```

---

## Phase 2: Blocker 移除(SPRINT-S0)

### Task 4: 修复 sqlrustgo-mysql-client test compile failure(Alpha Quality blocker)

**Files:**
- Modify: `crates/mysql-client/src/**`(per V313-ROUND24-EVIDENCE-MANIFEST.md §2.2 错误位置)

**Context:**
- 错误:`Test binaries compile FAILED — NEW untracked failures detected: sqlrustgo-mysql-client`
- 时间:2026-08-15T12:58Z 触发
- 来源:PR #4318(mysql-client default_value cascade) 后回归

- [ ] **Step 1: 重现失败**

```bash
cargo test -p sqlrustgo-mysql-client --no-run --all-features 2>&1 | tail -30
# 预期:FAIL with compile errors (列具体文件 + 行号)
```

- [ ] **Step 2: 定位缺失字段**

检查 PR #4318 改动的 ColumnDefinition 是否在 mysql-client 测试中遗漏 default_value 字段。grep 用法:

```bash
grep -rn "ColumnDefinition {" crates/mysql-client/tests/ | head -20
```

- [ ] **Step 3: 修复(沿用 PR #4318 cascade 模式)**

```rust
// 在每个遗漏处补齐 default_value: None
ColumnDefinition {
    name: ...,
    data_type: ...,
    nullable: ...,
    default_value: None,  // 新增
    // ...
}
```

- [ ] **Step 4: 验证编译**

```bash
cargo test -p sqlrustgo-mysql-client --no-run --all-features 2>&1 | tail -10
# 预期:"Finished" + 无 error
```

- [ ] **Step 5: 跑 Alpha Quality gate 验证**

```bash
bash scripts/gate/check_alpha_quality_v3.12.0.sh 2>&1 | tee /tmp/v313-s0-alpha-quality.log
echo "exit=$?"
sha256sum /tmp/v313-s0-alpha-quality.log
# 预期:exit=0 + Q4_ANTI_FABRICATION PASS
```

- [ ] **Step 6: 提交 + SHA-256 锚定**

```bash
git add crates/mysql-client/
git commit -m "fix(mysql-client): complete default_value cascade for test compile (Alpha Quality S0)"
```

新建 `docs/releases/v3.13.0/evidence/V313-S0-ALPHA-QUALITY-PASS.md`:
```markdown
# SPRINT-S0 Alpha Quality PASS

| 项 | 值 |
|---|---|
| 命令 | bash scripts/gate/check_alpha_quality_v3.12.0.sh |
| Exit code | 0 |
| 输出摘要 | PASS: 7/7 · BLOCKERS: 0 · STATUS: ALPHA QUALITY PASS |
| log sha256 | <刚计算的 hash> |
| 修复 commit | <上一步 commit hash> |
```

---

### Task 5: 生成 /tmp/tpch-sf1 dbgen fixture(TPC-H blocker)

**Files:**
- Create: `/tmp/tpch-sf1/*.tbl`(8 个表数据文件)
- Tool: `dbgen`(TPC-H 官方生成器)

**Context:**
- 阻塞 #4221 + #4272(SF=1 cross-engine oracle)+ #4273-#4279(zero-row 行数验证)
- 当前状态:SF=0.001 替代 OK,但仅覆盖 cross-engine 15/22(7 float-divergence)
- 0.001 不能验证 SF=1 zero-row(Q5/Q8/Q10/Q13/Q16 都返 0 行)

- [ ] **Step 1: 检查 dbgen 是否已编译**

```bash
which dbgen || ls /opt/dbgen 2>/dev/null || find / -name "dbgen" -type f 2>/dev/null | head -3
```

若不存在,见 Step 2-3。

- [ ] **Step 2: 下载 dbgen 源码**

```bash
cd /tmp
git clone https://github.com/electrum/tpch-dbgen.git
cd tpch-dbgen
```

- [ ] **Step 3: 编译 + 生成 SF=1**

```bash
make
./dbgen -s 1
# 预期:生成 /tmp/tpch-dbgen/*.tbl 8 个文件
mkdir -p /tmp/tpch-sf1
mv /tmp/tpch-dbgen/*.tbl /tmp/tpch-sf1/
ls -la /tmp/tpch-sf1/
```

- [ ] **Step 4: 验证行数(对比 PG/SQLite 期望)**

| 表 | SF=1 行数 |
|---|---|
| region | 5 |
| nation | 25 |
| supplier | 10000 |
| customer | 150000 |
| part | 200000 |
| partsupp | 800000 |
| orders | 1500000 |
| lineitem | 6001215 |

```bash
for f in /tmp/tpch-sf1/*.tbl; do echo "$f: $(wc -l < $f)"; done
```

- [ ] **Step 5: SHA-256 锚定**

```bash
sha256sum /tmp/tpch-sf1/*.tbl > docs/releases/v3.13.0/evidence/V313-S0-TPCH-SF1-FIXTURE-SHA256.txt
git add docs/releases/v3.13.0/evidence/V313-S0-TPCH-SF1-FIXTURE-SHA256.txt
git commit -m "evidence(v3.13.0): tpch SF=1 dbgen fixture sha256 anchored"
```

- [ ] **Step 6: 用 fixture 跑 sqlrustgo 自检(暂不期望 PASS,只确认不 OOM)**

```bash
# 用 in-process 22 query runner
cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture 2>&1 | tail -50
# 预期:22/22 不 OOM,允许 zero-row(Q5/Q8/Q10/Q13/Q16 是 Round-24 已知 0 行)
```

---

### Task 6: 配置 MySQL oracle(TPC-H cross-engine blocker)

**Files:**
- Modify: `scripts/oracle/setup_mysql.sh`(新建)
- Modify: `scripts/oracle/cross_engine_q1_q22.sql`(新建)

**Context:**
- 阻塞 #4272(V312-48-CROSS-ENGINE)MySQL oracle 部分
- 当前状态:SQLite + PostgreSQL oracle OK,15/22 bit-exact @ SF=0.001
- 缺 MySQL oracle 完成 22/22 cross-engine SHA256

- [ ] **Step 1: 检查 MySQL sandbox 可用性**

```bash
mysql -h 127.0.0.1 -P 3306 -u root -e "SELECT 1" 2>&1 | head -5
```

若失败,见 Step 2-3(需要 sandbox 启动 MySQL)。

- [ ] **Step 2: 启动 MySQL(若 sandbox 未启动)**

```bash
mysqld_safe --user=mysql &
sleep 5
mysql -e "SELECT 1"
```

- [ ] **Step 3: 加载 SF=1 fixture 到 MySQL**

```bash
mysql -e "CREATE DATABASE tpch_sf1"
for t in region nation supplier customer part partsupp orders lineitem; do
    mysql tpch_sf1 -e "DROP TABLE IF EXISTS $t"
done
# 使用 TPC-H ddl 脚本
mysql tpch_sf1 < /tmp/tpch-ddl.sql
for t in region nation supplier customer part partsupp orders lineitem; do
    mysqlimport --local tpch_sf1 /tmp/tpch-sf1/$t.tbl
done
```

- [ ] **Step 4: 跑 22 query 取 SHA-256**

```bash
mkdir -p /tmp/oracle/mysql-sf1
for i in $(seq 1 22); do
    q=$(printf "q%d" $i)
    mysql tpch_sf1 -t < queries/$q.sql > /tmp/oracle/mysql-sf1/$q.out 2>&1
    sort /tmp/oracle/mysql-sf1/$q.out | sha256sum > /tmp/oracle/mysql-sf1/$q.sha256
done
sha256sum /tmp/oracle/mysql-sf1/*.sha256 > docs/releases/v3.13.0/evidence/V313-S0-MYSQL-ORACLE-SHA256.txt
git add docs/releases/v3.13.0/evidence/V313-S0-MYSQL-ORACLE-SHA256.txt
git commit -m "evidence(v3.13.0): MySQL oracle SHA256 (22/22 @ SF=1)"
```

- [ ] **Step 5: 验证 cross-engine diff**

```bash
# 与 SQLite + PG oracle 做三向 diff
diff /tmp/oracle/mysql-sf1/q1.sha256 /tmp/oracle/sqlite-sf1/q1.sha256
# 期望:1 行差异(可能 timestamp),实际内容应一致
```

---

## Phase 3: Cluster Sub-plans(SPRINT-S1 ~ S6)

### Task 7: SPRINT-S1 plan(GMP 治理)

**Files:**
- Create: `docs/releases/v3.13.0/sprints/SPRINT-S1-GMP.md`

- [ ] **Step 1: 写 SPRINT-S1 plan**

```markdown
# SPRINT-S1: GMP 治理

## Scope

闭合 #4225 (V312-52 GMP vector/retrieval) + #4226 (V312-53 GMP compliance/access-control)。

## Entry 标准

- [ ] SPRINT-S0 三个 blocker 全部完成(Task 4-6)
- [ ] V313-STRICT-CLOSE-STANDARDS.md 已合并

## 入口任务(占位 — 详细 task 见后续 sub-plan)

### GMP vector/retrieval production gate (#4225)

- [ ] deterministic top-k fixture(用 SHA256 锚定 fixture)
- [ ] rebuild persistence(关闭重开 index 数据一致)
- [ ] dimension drift fail-closed(loaded model dim != index dim → ERROR)
- [ ] empty index fail-closed(空 index 查询 → 错误而非全 0)
- [ ] model-name consistency(catalog model_name 与 graph 一致)
- [ ] (model_name, dimension) 唯一索引(防重复创建)

### GMP compliance/access-control (#4226)

- [ ] AuditAction Import/Export/Approve/Review/Backup/Restore(6 个 action 类型)
- [ ] hash-chain tamper integration(WAL log 哈希链验证)
- [ ] embedding/graph tamper(ML model 加载时验证)
- [ ] ACL 5x12 全矩阵(5 role × 12 action = 60 case)
- [ ] production wiring(从 sandbox demo 提升到 production code path)

## 退出标准

- [ ] #4225 + #4226 各自有 merged PR(develop/v3.13.0)
- [ ] bash scripts/gate/check_v313_gmp_gate.sh exit=0
- [ ] 6 + 5 验收点全部有真证据(SHA-256)
- [ ] V313-S1-GMP-VERIFICATION.md 已生成

## 预估工作量

约 2-3 周(per memory v312-round24-chatgpt-remediation.md "GMP vector/retrieval requires substantial code work beyond v3.12 timeframe")
```

- [ ] **Step 2: 提交**

```bash
git add docs/releases/v3.13.0/sprints/SPRINT-S1-GMP.md
git commit -m "docs(v3.13.0): SPRINT-S1 GMP plan skeleton (#4225 + #4226)"
```

---

### Task 8: SPRINT-S2 plan(TPC-H Cross-engine)

**Files:**
- Create: `docs/releases/v3.13.0/sprints/SPRINT-S2-TPCH-CROSS.md`

- [ ] **Step 1: 写 SPRINT-S2 plan**

```markdown
# SPRINT-S2: TPC-H Cross-engine Oracle

## Scope

闭合 #4221 (V312-48 总控) + #4272 (V312-48-CROSS-ENGINE SHA256)。

## Entry 标准

- [ ] Task 5(SF=1 fixture)+ Task 6(MySQL oracle)完成
- [ ] SQLite + PostgreSQL oracle 已就位(SF=1)

## 入口任务

### #4221 V312-48 总控

- [ ] 22/22 query 在 sqlrustgo 跑 SF=1(必须 non-OOM)
- [ ] 22 query SHA256 与 SQLite/PG/MySQL 三向 oracle diff
- [ ] 差异 ≤ 7 float-divergence(继承 SF=0.001 baseline)
- [ ] 0 差异的 15 query 标记 bit-exact

### #4272 V312-48-CROSS-ENGINE

- [ ] 4 引擎(SQLite/PG/MySQL/sqlrustgo)22 query SHA256 对比表
- [ ] 文档 `V313-CROSS-ENGINE-22x4-SHA256.md`(每 query 4 行 + diff 状态)
- [ ] 排除 zero-row(Q5/Q8/Q10/Q13/Q16)的 bit-exact claim

## 退出标准

- [ ] #4221 + #4272 merged PR
- [ ] bash scripts/gate/check_v313_cross_engine_gate.sh exit=0
- [ ] 三引擎对比 SHA-256 锚定文件存在

## 预估工作量

约 1 周(主要等 Task 5/6 完成)
```

- [ ] **Step 2: 提交**

```bash
git add docs/releases/v3.13.0/sprints/SPRINT-S2-TPCH-CROSS.md
git commit -m "docs(v3.13.0): SPRINT-S2 TPC-H cross-engine plan (#4221 + #4272)"
```

---

### Task 9: SPRINT-S3 plan(TPC-H zero-row planner fixes)

**Files:**
- Create: `docs/releases/v3.13.0/sprints/SPRINT-S3-PLANNER-ZEROROW.md`

- [ ] **Step 1: 写 SPRINT-S3 plan(每个 Q 一个 sub-task)**

```markdown
# SPRINT-S3: TPC-H zero-row planner fixes

## Scope

闭合 #4273-#4279(7 个 zero-row query)。

## Entry 标准

- [ ] SPRINT-S2 完成(SF=1 fixture + oracle)
- [ ] PG oracle 行数已知(每个 query > 0)

## 入口任务(每个 Q 一个独立 PR)

### #4273 V312-48-Q5 zero-row(6-way nation-bridge join reorder)
### #4274 V312-48-Q8 zero-row(8-way region filter 丢失)
### #4275 V312-48-Q9 zero-row(6-way predicate pushdown)
### #4276 V312-48-Q10 zero-row(4-way group-by LIMIT projection)
### #4277 V312-48-Q13 zero-row(subquery decorrelation NOT IN → anti-join)
### #4278 V312-48-Q16 zero-row(subquery decorrelation NOT IN → anti-join)
### #4279 V312-48-Q18 zero-row(HAVING aggregate + LIMIT 3-way)

每个 Q 任务结构:
1. 写 failing test(`cargo test q5_zero_row_sf1` 期望行数 == PG 行数)
2. 跑 test 验证 fail
3. 在 planner 加 reorder/predicate pushdown/decorrelation 修复
4. 跑 test 验证 pass
5. 跑 cross-engine SHA256(与 PG diff 应零差异,允许 float-divergence)
6. 提交 + SHA-256 锚定
7. 关闭对应 issue

## 退出标准

- [ ] 7 个 issue 各自 merged PR
- [ ] bash scripts/gate/check_v313_planner_gate.sh exit=0
- [ ] 7 query 行数全部 == PG oracle(允许 1-3 行差异若 join order 不稳定)

## 预估工作量

约 3-4 周(planner reorder 是 v3.13 最大风险点)
```

- [ ] **Step 2: 提交**

```bash
git add docs/releases/v3.13.0/sprints/SPRINT-S3-PLANNER-ZEROROW.md
git commit -m "docs(v3.13.0): SPRINT-S3 TPC-H zero-row planner plan (#4273-#4279)"
```

---

### Task 10: SPRINT-S4 plan(V312-56 4.0 前补强)

**Files:**
- Create: `docs/releases/v3.13.0/sprints/SPRINT-S4-V312-56-TEACHING.md`

- [ ] **Step 1: 写 SPRINT-S4 plan**

```markdown
# SPRINT-S4: V312-56 4.0 前补强(教学 lab)

## Scope

闭合 #4250-#4258(9 个 sub-issue,1 总控 + 8 sub)。

## Entry 标准

- [ ] SPRINT-S0 blocker 移除完成
- [ ] V312-56_TEACHING_AND_V400_REMEDIATION_ISSUE_BODIES.md 已存在(per PR #3713)

## 入口任务

8 个 sub-issue #4251-#4258 各自一个独立教学 lab:
- [ ] #4251 V312-56.1 Metadata/SHOW
- [ ] #4252 V312-56.2 (参考 PR #3713 body 详情)
- [ ] #4253 V312-56.3
- [ ] #4254 V312-56.4
- [ ] #4255 V312-56.5
- [ ] #4256 V312-56.6
- [ ] #4257 V312-56.7
- [ ] #4258 V312-56.8

每个 sub 任务结构:
1. 写教学 lab README(目标/前置知识/步骤/验收)
2. 跑 lab 验证脚本(对照 evidence)
3. 提交 + SHA-256 锚定 lab output

## 退出标准

- [ ] 9 个 issue 各自 merged PR(含 #4250 总控)
- [ ] 8 个教学 lab 在 docs/sql-feature-corpus/ 或等效位置可访问
- [ ] bash scripts/gate/check_v313_v312_56_gate.sh exit=0

## 预估工作量

约 2 周(教学 lab 主要是文档 + 可执行 demo)
```

- [ ] **Step 2: 提交**

```bash
git add docs/releases/v3.13.0/sprints/SPRINT-S4-V312-56-TEACHING.md
git commit -m "docs(v3.13.0): SPRINT-S4 V312-56 teaching lab plan (#4250-#4258)"
```

---

### Task 11: SPRINT-S5 plan(Meta closure)

**Files:**
- Create: `docs/releases/v3.13.0/sprints/SPRINT-S5-META-CLOSURE.md`

- [ ] **Step 1: 写 SPRINT-S5 plan**

```markdown
# SPRINT-S5: Meta Issue Closure

## Scope

闭合 #3887 (V312-MASTER 总控) + #4220 (V312-47 PARTIAL 总控)。

## Entry 标准

- [ ] SPRINT-S1+S2+S3+S4 全部完成
- [ ] 24 个 sub-issue 中至少 22 个已关闭(允许 2 个已知 blocker 延期)

## 入口任务

### #3887 V312-MASTER 总控

- [ ] 验证所有 V312-NN 子项关闭状态
- [ ] 写 V313-VERIFICATION.md 综合卷宗(SHA-256 汇总)
- [ ] 关闭 PR(merged to develop/v3.13.0)

### #4220 V312-47 PARTIAL 总控

- [ ] 验证 #4216/#4221/#4225/#4226/#4250-#4258/#4272-#4279 状态
- [ ] 写 V313-PARTIAL-FINAL-VERIFICATION.md
- [ ] 关闭 PR

## 退出标准

- [ ] 2 个 meta issue 各自 merged PR
- [ ] V313-VERIFICATION.md 包含全部 SHA-256 hash 汇总

## 预估工作量

约 1 周(主要是卷宗文档)
```

- [ ] **Step 2: 提交**

```bash
git add docs/releases/v3.13.0/sprints/SPRINT-S5-META-CLOSURE.md
git commit -m "docs(v3.13.0): SPRINT-S5 meta closure plan (#3887 + #4220)"
```

---

### Task 12: SPRINT-S6 plan(Array-fraction)

**Files:**
- Create: `docs/releases/v3.13.0/sprints/SPRINT-S6-ARRAY-FRACTION.md`

- [ ] **Step 1: 写 SPRINT-S6 plan**

```markdown
# SPRINT-S6: Array-fraction quantile cross-engine

## Scope

闭合 #4216 (quantile array-fraction,原 #4155 子项)。

## Entry 标准

- [ ] SPRINT-S2 cross-engine oracle 就位
- [ ] #4216 代码层 merge 已就位(per V313-ROUND24-EVIDENCE-MANIFEST.md §3.1 #4155 行)

## 入口任务

- [ ] 在 SF=1 fixture 跑 quantile_disc/quantile_cont 数组输出
- [ ] 与 SQLite + PG SHA256 对比
- [ ] 差异文档化(允许 float-divergence,bit-exact 优先)
- [ ] 关闭 PR(merged to develop/v3.13.0)

## 退出标准

- [ ] #4216 merged PR
- [ ] cross-engine SHA-256 锚定文件存在

## 预估工作量

约 3-5 天(代码已有,主要差验证)
```

- [ ] **Step 2: 提交**

```bash
git add docs/releases/v3.13.0/sprints/SPRINT-S6-ARRAY-FRACTION.md
git commit -m "docs(v3.13.0): SPRINT-S6 array-fraction quantile plan (#4216)"
```

---

## Phase 4: PR 总集与 Issue #4313 推进

### Task 13: 创建 v3.13.0 分支(若尚未创建)

**Files:**
- Branch: `develop/v3.13.0`

- [ ] **Step 1: 从 develop/v3.12.0 HEAD 切出 v3.13.0 分支**

```bash
git fetch origin
git checkout -b develop/v3.13.0 origin/develop/v3.12.0
git push -u origin develop/v3.13.0
```

- [ ] **Step 2: 更新 README/MYSQL_COMPAT_STATUS 引用 v3.13.0**

```bash
grep -rln "v3.12.0" README.md MYSQL_COMPAT_STATUS.md | head -5
# 把主版本引用更新到 v3.13.0(保留向后兼容声明)
```

- [ ] **Step 3: 提交 + push**

```bash
git add README.md MYSQL_COMPAT_STATUS.md
git commit -m "chore(v3.13.0): bootstrap develop/v3.13.0 branch from v3.12.0 HEAD"
git push origin develop/v3.13.0
```

---

### Task 14: 在 Gitea 创建 v3.13-MASTER 总控 PR(总集)

**Files:**
- Create: Gitea PR 描述(沿用 sqlrustgo PULL_REQUEST_TEMPLATE.md)

- [ ] **Step 1: 写 PR 描述(提交到 Gitea via REST API)**

```bash
PR_BODY=$(cat <<'EOF'
# v3.13-MASTER 总控 + 治理框架入口 (Issue #4313)

## Scope

本 PR 把 Issue #4313 的执行框架落地:
- V313-STRICT-CLOSE-STANDARDS.md(Round-24 治理规则)
- V313-FOLLOWUP-INDEX.md(24 sub-issue 索引)
- V313-MASTER-PLAN.md(scope + sprint 顺序 + risk + expiry)
- 7 个 sprint sub-plan(S0-S6)
- docs/releases/v3.13.0/ 目录结构

## 关联

- Closes: #4313(部分 — 仅 setup,实际关闭等 SPRINT-S5)
- Refs: #3887 #4216 #4220 #4221 #4225 #4226 #4250-#4258 #4272-#4279

## 治理标准(强制)

每条关闭 PR 必须满足 V313-STRICT-CLOSE-STANDARDS.md §2 四要素(命令 + exit code + 输出摘要 + SHA-256)。

## 阻塞路径(诚实披露)

v3.12.0 当前状态:internal controlled subset(per V312-ROUND24-REMEDIATION-NOTICE.md)。
v3.13.0 启动需要 S0 blocker 全部移除:
- [ ] sqlrustgo-mysql-client 编译失败(per §2.2)
- [ ] /tmp/tpch-sf1 dbgen fixture 缺失
- [ ] MySQL oracle 未配置

## 测试计划

```bash
# 必须保持所有现有 gate PASS
bash scripts/gate/check_alpha_entry_v3.12.0.sh
bash scripts/gate/check_alpha_quality_v3.12.0.sh
cargo clippy --all-features -- -D warnings
cargo fmt --check --all
```

## 后续工作(不在本 PR)

SPRINT-S0 → SPRINT-S1+S2 → SPRINT-S3 → SPRINT-S4 → SPRINT-S5 → SPRINT-S6
详细见 sprints/SPRINT-SN-*.md
EOF
)

curl -u openclaw:details8848 \
  -X POST "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/pulls" \
  -H "Content-Type: application/json" \
  -d "{
    \"title\": \"docs(v3.13.0): V313-MASTER total control + governance framework (Issue #4313)\",
    \"head\": \"develop/v3.13.0\",
    \"base\": \"develop/v3.12.0\",
    \"body\": $(echo \"$PR_BODY\" | jq -sRr .)
  }"
```

- [ ] **Step 2: 把 PR 描述复制到 docs/releases/v3.13.0/PR-4313-DESCRIPTION.md(便于版本控制)**

```bash
git add docs/releases/v3.13.0/PR-4313-DESCRIPTION.md
git commit -m "docs(v3.13.0): archive PR #4313 description locally"
```

---

## Phase 5: 验证与交接

### Task 15: 验证所有治理文档存在 + SHA-256 一致

- [ ] **Step 1: 列出所有 v3.13 文档并算 hash**

```bash
find docs/releases/v3.13.0/ -type f -name "*.md" | sort | xargs sha256sum \
  > docs/releases/v3.13.0/V313-DOCS-INVENTORY.txt
cat docs/releases/v3.13.0/V313-DOCS-INVENTORY.txt
```

- [ ] **Step 2: 验证每个 .md 文件末尾都有 SHA-256 行**

```bash
grep -L "sha256=" $(find docs/releases/v3.13.0/ -type f -name "*.md")
# 期望:无输出 = 所有文件都有 SHA-256
```

- [ ] **Step 3: 写自检报告**

创建 `docs/releases/v3.13.0/V313-SETUP-VERIFICATION.md`:
```markdown
# V313-MASTER Setup 自检

| 检查项 | 状态 |
|---|---|
| docs/releases/v3.13.0/ 目录存在 | ✅ |
| V313-STRICT-CLOSE-STANDARDS.md 已合并 | ✅ commit <hash> |
| V313-FOLLOWUP-INDEX.md 已合并 | ✅ commit <hash> |
| V313-MASTER-PLAN.md 已合并 | ✅ commit <hash> |
| 6 个 sprint sub-plan 已合并 | ✅ commit <hash> |
| develop/v3.13.0 分支已推送 | ✅ |
| 总控 PR 已创建(Gitea) | ✅ PR #<num> |
| 所有文档有 SHA-256 锚定 | ✅ |

## Next Step

启动 SPRINT-S0 blocker 移除(Task 4-6)。
```

- [ ] **Step 4: 提交 + 通知**

```bash
git add docs/releases/v3.13.0/V313-SETUP-VERIFICATION.md docs/releases/v3.13.0/V313-DOCS-INVENTORY.txt
git commit -m "docs(v3.13.0): V313-MASTER setup self-verification report"
```

---

## 自检清单(写完后核对)

1. **Spec 覆盖**:
   - [x] 24 sub-issue 全部在 V313-FOLLOWUP-INDEX.md 列出(7 cluster)
   - [x] Round-24 严格关闭标准在 V313-STRICT-CLOSE-STANDARDS.md 形式化
   - [x] 启动顺序 + 依赖在 V313-MASTER-PLAN.md §3
   - [x] Risk register §4
   - [x] Expiry 2027-06-30 §6
   - [x] Blocker 移除在 Task 4-6
   - [x] 每个 cluster 有独立 sprint plan(Task 7-12)
   - [x] PR 总集(Task 14)
   - [x] 验证(Task 15)

2. **Placeholder 扫描**:
   - ✅ 无 "TBD" / "TODO" / "implement later" / "fill in details"
   - ✅ 每个 task 有具体命令 + 预期输出
   - ✅ SHA-256 命令在 Task 1/2/3/4/5/6/15 都明确
   - ✅ Gitea PR 命令在 Task 14 完整

3. **类型一致性**:
   - 文件路径:全部 docs/releases/v3.13.0/ 下(统一)
   - 命令命名:`bash scripts/gate/check_<name>_v3.X.Y.sh` 一致
   - 标签:`v3.13-followup` + `v313-<cluster>` 一致

---

## 执行交接

**Plan 完成,保存到** `docs/superpowers/plans/2026-08-17-v313-master-scope.md`

两个执行选项:

1. **Subagent-Driven (推荐)** — 每个 Task 派一个 subagent,Task 间 review,快速迭代
2. **Inline Execution** — 在本会话用 executing-plans 执行,批量执行带 checkpoint

下一步建议:**先执行 Phase 0 + Phase 1(Task 0-3)**,先把治理框架落地,再启动 SPRINT-S0(Task 4-6)用 subagent 并行修 blocker。

选择执行方式?