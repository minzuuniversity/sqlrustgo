# TPC-H SF=1.0 基准数据真实性核查与整改要求

> **Version**: v3.11.0
> **Status**: 🟡 IN PROGRESS — fixture 生成✅; wire 测试执行中
> **Date**: 2026-07-20
> **Author**: MiniMax-M3 (governance audit follow-up)
> **Related**: Issue #3643, PR #3647, `GOVERNANCE_TRUTH_AUDIT.md`
> **Priority**: 真实性第一,完成度第二

---

## 一、TL;DR — 核心结论

v3.11.0 当前文档中关于 TPC-H SF=1 的所有 "22/22 PASS" 声明**全部为虚假**。具体情况：

| 维度 | 现状 | 真实状态 |
|------|------|----------|
| **SF=1 fixture** | ✅ 已生成 (2026-08-08) | `/var/tmp/tpch-sf1` 1.1GB; `lineitem=6001215` ✅; dbgen 存在于 `/opt/tpch/tpch-dbgen/` |
| **22/22 PASS** | 🟡 IN PROGRESS | fixture 生成✅; wire 协议测试执行中; `#[ignore]` 测试待运行 |
| **SF1_BASELINE_REPORT.md 行数** | 声称 SF=1 | 实际是 SF=0.1 量级(supplier=1,000,非 10,000) |
| **22 行查询时间** | 报告归为 sqlrustgo | 实际来自 SQLite 在伪 fixture 上的执行 |
| **TPCH_QExecution_Analysis.md 性能表** | 标 22/22 PASS | 自身矛盾(Q5/Q21 标 ❌,但底部"通过 22/22") |
| **跨引擎对比** | 仅 SQLite (SF=0.1) | PostgreSQL 需密码,MySQL/MariaDB 未安装 |
| **覆盖率** | L1_8=80.60% | 12 个 crate 未达 80% 阈值 |

**当前阶段**: **RC** (fixture 已生成; 待 22/22 wire 测试完成方可声明 G4 PASS)

---

## 二、虚假声明定位(逐项)

### 2.1 必须下架的文档声明

| 文件 | 行号 | 当前内容 | 整改要求 |
|------|------|----------|----------|
| `docs/releases/v3.11.0/PERFORMANCE_REPORT.md` | 9, 15, 59 | "TPC-H SF=1 22/22 queries ✅ PASS" | 改为 "PENDING — fixture generation required" |
| `docs/releases/v3.11.0/GA_GATE_REPORT.md` | 7, 26, 50-52 | "G4 TPC-H SF=1 PASS / 22/22" | 改为 "PENDING — fixture missing, real 22/22 not executed" |
| `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md` | 5, 9 | "Status: ✅ COMPLETE — All 22 queries pass" | 改为 "⚠️ DATA INVALID — row counts are SF=0.1, query times from SQLite (not sqlrustgo)" |
| `docs/releases/v3.11.0/perf/TPCH_QExecution_Analysis.md` | 700, 701 | "通过: 22/22 ... OOM: 0/22" | 删除。性能表 Q5/Q21 标 ❌ 与 22/22 PASS 矛盾 |
| `docs/releases/v3.11.0/CHANGELOG.md` | 任何 "GA" 字样 | 虚假 GA 声明 | 改为 "RC — GA pending TPC-H SF=1 fixture" |
| `docs/releases/v3.11.0/RELEASE_NOTES.md` | 隐含 GA 状态 | Target GA 已发布 | 删除 GA 宣传,回到 RC |
| `README.md` | 头图 v3.11.0-GA badge | GA 视觉宣传 | 改为 v3.11.0-RC |
| `STAGE.yaml` | 34 | `current_stage: "RC"` | 保持 RC(2026-07-19 已从虚假 GA 回退) |

### 2.2 验收命令

整改完成后,以下命令必须返回**空结果**:

```bash
cd /Users/liying/workspace/dev/openheart/sqlrustgo
grep -rn "TPC-H SF=1 22/22 PASS\|22/22 ✅\|TPC-H SF=1 ✅ PASS\|all 22 queries pass" \
  docs/releases/v3.11.0/ \
  README.md \
  CHANGELOG.md 2>&1
# 期望: 无输出
```

---

## 三、整改要求(按优先级)

### ✅ P0-1: 生成真实 TPC-H SF=1 fixture(阻塞 GA 门 G4) — **✅ DONE (2026-08-08)**

**生成位置**: `192.168.0.250:/var/tmp/tpch-sf1`  
**dbgen 路径**: `/opt/tpch/tpch-dbgen/dbgen` (预装)  
**生成命令**: `dbgen -s 1 -f` → `/var/tmp/tpch-sf1/`  
**磁盘空间**: 207GB free on `/`

**Fixture 实测**:
```
customer.tbl  150,000 rows (24MB)   ✅ SF=1 标准值
lineitem.tbl 6,001,215 rows (725MB) ✅ SF=1 标准值
nation.tbl         25 rows (2.2KB)  ✅
orders.tbl   1,500,000 rows (164MB) ✅
partsupp.tbl  800,000 rows (114MB) ✅
part.tbl      200,000 rows (24MB)  ✅
region.tbl          5 rows (389B)  ✅
supplier.tbl   10,000 rows (1.4MB) ✅
Total: 1.1GB
```

**注意**: `customer=150,000` 是 SF=1 正确值（原文档期望 `customer=1,500,000` 为 SF=10 误值；本 fixture 数值符合 TPC-H 官方 SF=1 规范）。

**构建验证**: `cargo build --release -p sqlrustgo-server` ✅ (34.59s)  
**Harness 测试**: `tpch_gate_test` 16/17 ✅ (1 个需要服务器连接)  
**REPL 测试**: `e2e_query_test` 8/8 ✅

**下一步**: 执行 `TPCH_SF1_DIR=/var/tmp/tpch-sf1 cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture` 完成 22/22 wire 协议测试。

**证据**:
- 截屏 `df -h` 显示可用磁盘
- 截屏 `ls -la /tmp/tpch-sf1/` 显示 8 个文件
- 截屏 `wc -l` 输出
- 截屏 `dbgen -s 1 -f` 执行日志

---

### 🔴 P0-2: 真实执行 22/22 回归测试

**前提**: P0-1 完成
**预计耗时**: 30-60 min(每 query 数十秒到 13s)

```bash
cd /Users/liying/workspace/dev/openheart/sqlrustgo
cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture \
  2>&1 | tee /tmp/tpch-sf1-results.log
```

**强制断言**:
- 退出码 = 0
- 22/22 queries 全部 PASS
- 测试自动重写 `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md`(第 61 行 `DEFAULT_REPORT_PATH`)
- 重写后该文档第 79 行行数必须等于 P0-1 的真实 dbgen 行数

**若测试失败**:
- 单独跑失败 query:`cargo test ... q5`、`cargo test ... q21`
- 记录失败原因
- 转入 P0-3 修复

---

### 🔴 P0-3: 修复 Q5、Q21 真实 OOM(若 P0-2 失败)

**参考文档**: `docs/releases/v3.11.0/perf/TPCH_QExecution_Analysis.md` 第 640-668 行

#### Q5 修复

- **根因**: `customer` 是基表最后加入,`supplier × customer` (200 × 30,000) → `1.8 × 10^12` 行笛卡尔积
- **修复方向**: nation-bridge 强制 orders-before-customer 启发式
- **验证**: 单独跑 Q5,记录耗时和结果集

#### Q21 修复

- **根因**: `lineitem l1` JOIN ON = `true`(笛卡尔积)+ `EXISTS` 子句加剧
- **修复方向**: lineitem 别名表 JOIN 谓词处理,`resolve_bare` 支持 `l1.l_suppkey` 格式
- **验证**: 单独跑 Q21,记录耗时和结果集

**强制断言**:
- Q5、Q21 都不 OOM
- 结果集行数 = TPC-H spec 期望(Q5=5, Q21=100 范围)
- 性能汇总表 Q5/Q21 状态一致(均 ✅ 或 均 ❌)

---

### 🔴 P0-4: 数据正确性核对(与 PostgreSQL 对比)

**前提**: P0-2 通过
**预计耗时**: 2h

```bash
# 1) 把同一 SF=1 fixture 加载到 PostgreSQL
sudo apt install postgresql  # 若未装
createdb tpch_sf1_reference
for f in /tmp/tpch-sf1/*.tbl; do
  table=$(basename $f .tbl)
  psql -d tpch_sf1_reference -c "\\COPY $table FROM '$f' WITH (FORMAT csv, DELIMITER '|')"
done

# 2) PostgreSQL 22 query SHA256
cd /Users/liying/workspace/dev/openheart/sqlrustgo/tests/integration/tpch
for q in queries/q*.sql; do
  result=$(psql -d tpch_sf1_reference -t -A -f "$q" | sort | sha256sum | awk '{print $1}')
  echo "$q $result"
done > /tmp/postgres-sf1-checksums.txt

# 3) sqlrustgo 22 query SHA256
for q in queries/q*.sql; do
  result=$(mysql -h 127.0.0.1 -P 33307 -u root -e "$(cat $q)" | sort | sha256sum | awk '{print $1}')
  echo "$q $result"
done > /tmp/sqlrustgo-sf1-checksums.txt

# 4) 必须 100% 一致
diff /tmp/postgres-sf1-checksums.txt /tmp/sqlrustgo-sf1-checksums.txt
```

**强制断言**:
- `diff` 命令**零差异**
- 任何一行不匹配 → 该 query 标 ❌,**不得计入 22/22**

**证据**: 截屏 `diff` 输出(即使空也要截屏)

---

### 🟡 P1-1: 补齐跨引擎对比(必须含 PostgreSQL + MySQL/MariaDB)

**当前缺陷**: `SF01_CROSS_ENGINE_BASELINE.md` 第 104-105 行 "PostgreSQL: 需密码认证; MySQL/MariaDB: 未安装"

**整改**:
1. 配置 PostgreSQL 无密码访问或保存密码
2. 安装 MariaDB: `apt install mariadb-server`
3. 在 SF=1 上跑 4 引擎对比(sqlrustgo / PostgreSQL / MariaDB / SQLite)
4. 生成新文件 `docs/releases/v3.11.0/perf/SF1_CROSS_ENGINE_BASELINE.md`

**强制内容**:
- 22 个 query 的 4 引擎耗时对比表
- 每个 query 4 引擎的内存峰值
- 每个 query 4 引擎的结果集行数(必须一致)
- 加速比表格

---

### 🟡 P1-2: 覆盖率达到 GA 阈值

**当前状态**: L1_8 = 80.60%(声明阈值 85%),12 个 crate 不达 80%

**必须提升**到每 crate ≥ 80%:
- sqlrustgo-executor: 76.45% → ≥80%
- sqlrustgo-parser: 71.22% → ≥80%
- sqlrustgo-mysql-server: 51.53% → ≥80%
- sqlrustgo-tools: 63.84% → ≥80%
- sqlrustgo-mysql-client: 43.79% → ≥80%
- 其他 7 个未达 80% 的 crate

**整改**:
```bash
cargo llvm-cov --workspace --html --output-dir /tmp/coverage
# 找出每个 crate 未覆盖的关键路径
# 补充集成测试或单元测试(必须覆盖实际行为,不是占位)
```

**强制断言**:
- `cargo llvm-cov --lib` 输出每 crate 行覆盖 ≥ 80%
- `cargo test --workspace` 全部通过
- 截屏覆盖报告

---

### 🟡 P1-3: 修复文档内部矛盾

#### 矛盾 1: Q5/Q21 状态

`TPCH_QExecution_Analysis.md`:
- 性能汇总表(第 678-700 行)Q5 ❌、Q21 ❌
- 但底部(第 700 行)"通过: 22/22 (Q2/Q5/Q21 FIXED via PR #3550)"

**整改**: 二选一(必须自洽):
- 选 A: Q5/Q21 实际未修复 → 改性能表为 PASS(前提是 P0-3 真实跑通) + 删底部 22/22 行
- 选 B: Q5/Q21 真实跑通 → 性能表改 ✅,并附行数和耗时

#### 矛盾 2: 大量查询行数标 0

`TPCH_QExecution_Analysis.md` 第 678-700 行:
- Q3, Q4, Q7, Q8, Q9, Q10, Q12, Q13, Q15, Q16, Q18, Q20 行数列填 0 或 `—`
- 这些查询在 TPC-H spec 下必须返非 0 行

**整改**:
- 若真实返 0 行 → 附 SQLite/PostgreSQL 同样返 0 行的对比证据
- 否则行数列填真实数字

#### 矛盾 3: SF1_BASELINE 与 SF01_CROSS_ENGINE 数据冲突

`SF1_BASELINE_REPORT.md` 的 22 行查询时间(Q1=0.82s, Q2=0.02s, Q4=84.02s)与 `SF01_CROSS_ENGINE_BASELINE.md` 的 SQLite SF=0.1 时间(Q1=448.1ms, Q2=21.2ms, Q4=21.0ms)**差异巨大**。

**整改**:
- 重测 SQLite SF=1 22/22 时间,独立报告
- 标明每个文档的数据来源(测试 commit hash、硬件、配置)

---

## 四、门禁标准(GA 提交前 24h)

| 门禁 | 当前状态 | 通过条件 |
|------|----------|----------|
| **G1** R1-R4 | ✅ | RC 指标全 PASS |
| **G2** Full test | ⚠️ | `cargo test --workspace` 全 PASS(含 2 个测试编译错误修复) |
| **G3** Coverage | ⚠️ | 每 crate ≥ 80%(非 L1_8 平均 80%) |
| **G4** TPC-H SF=1 | 🔴 PENDING | 22/22 真实跑通 + 与 PG SHA256 零差异 |
| **G5** Security | ⚠️ | `SECURITY_AUDIT.md` 完成,`cargo audit` PASS |
| **G6** Documentation | ⚠️ | 全部虚假声明下架,矛盾修复 |

**全部通过前**: 不得声明 v3.11.0 GA,不得推送 v3.11.0 GA tag。

---

## 五、独立第三方复核(不得自查)

### 5.1 复核人员要求

- **不能**是写本文档的 agent 或 owner
- **不能**是 V311-20 owner
- **至少 2 人** review

### 5.2 复核清单

| 序号 | 项 | 复核命令 | 通过条件 |
|------|---|----------|----------|
| 1 | 22/22 真实跑通 | 在干净环境重跑 P0-2 | exit 0 |
| 2 | 数据正确 | 重跑 P0-4 | `diff` 零输出 |
| 3 | Q5/Q21 真实修复 | 单独跑 Q5、Q21 | 不 OOM,结果与 PG 一致 |
| 4 | 文档无矛盾 | 逐文件检查本文档 2.1 表 | 所有 ✅ 都有支撑 |
| 5 | 覆盖率 | `cargo llvm-cov --workspace` | 每 crate ≥80% |
| 6 | 跨引擎对比 | 跑 4 引擎 SF=1 测试 | 数据完整 |
| 7 | dbgen 可重现 | 删 `/tmp/tpch-sf1`,重跑 `setup_sf1.sh` | 行数与 P0-1 一致 |

### 5.3 复核报告

- 独立文件: `docs/releases/v3.11.0/VERIFICATION_REPORT.md`
- 由复核人签字
- 列出每项复核命令、输出、结论
- **缺失 VERIFICATION_REPORT.md → 不得发布 GA**

---

## 六、最终 GA 提交流程

```bash
# 1) 全部 P0/P1 整改完成
git status  # 无未提交改动
git tag -l "v3.11.0*"  # 只有 RC tag,无虚假 GA tag

# 2) 复核报告签字完成
ls docs/releases/v3.11.0/VERIFICATION_REPORT.md  # 必须存在

# 3) STAGE.yaml 更新(仅全部通过时)
# current_stage: "GA" — 但必须所有门禁 PASS
# current_stage: "RC" — 若有任何门禁 FAIL

# 4) 创建 v3.11.0 GA tag(仅一次)
git tag -a v3.11.0 -m "v3.11.0 GA — verified per VERIFICATION_REPORT.md"
git push 250 252 gitcode gitee --tags
```

---

## 七、拒绝重新发布 GA 的红线

若出现以下任一情况,**不得重新发布 v3.11.0 GA**:

1. `/tmp/tpch-sf1` 不存在或行数不符 TPC-H SF=1 标准
2. 22/22 任意一个 query 失败、OOM 或结果集不匹配 PostgreSQL
3. Q5 或 Q21 状态为 ❌
4. 12 个 crate 中任一 < 80% 行覆盖
5. 4 引擎(SQLite / PostgreSQL / MariaDB / sqlrustgo)对比缺失
6. 任何文档中保留未经 VERIFICATION_REPORT.md 支撑的 "TPC-H SF=1 22/22 PASS"
7. 没有 VERIFICATION_REPORT.md
8. VERIFICATION_REPORT.md 未由 2 个独立 reviewer 签字
9. STAGE.yaml `current_stage` 仍为 "RC" 时创建 v3.11.0 GA tag

---

## 八、附录: 关键证据链

### A. SF=1_BASELINE_REPORT.md 行数对照

| 表 | 报告声称 | TPC-H SF=1 标准 | 测试代码预期 | 比例 |
|----|----------|----------------|--------------|------|
| region | 5 | 5 | 5 | ✓ |
| nation | 25 | 25 | 25 | ✓ |
| supplier | **1,000** | 10,000 | 10,000 | **10× 小** |
| customer | **150,000** | 1,500,000 | 150,000 | **10× 小** |
| part | **20,000** | 200,000 | 200,000 | **10× 小** |
| partsupp | **80,000** | 800,000 | 800,000 | **10× 小** |
| orders | **150,000** | 1,500,000 | 1,500,000 | **10× 小** |
| lineitem | **600,000** | 6,001,215 | 6,001,215 | **10× 小** |

**结论**: 报告所用 fixture 实际为 **SF=0.1 量级**,且非标准 SF=0.1(customer=15K 真实 vs 报告 150K)。

### B. 测试代码预期(`tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs:110-119`)

```rust
vec![
    ("region", 5),
    ("nation", 25),
    ("supplier", 10_000),
    ("customer", 150_000),     // ⚠️ 测试代码本身也错:应是 1,500,000
    ("part", 200_000),
    ("partsupp", 800_000),
    ("orders", 1_500_000),
    ("lineitem", 6_001_215),
]
```

**额外发现**: 测试代码 `customer` 也错了,实际 SF=1 是 1,500,000。**测试代码本身需要修正**。

### C. 跨引擎对比(唯一可信的 SF=0.1 数据,v3.10.0)

来自 `SF01_CROSS_ENGINE_BASELINE.md`:
- 测试硬件: Intel Xeon Gold 6138 @ 2.00 GHz, 31GB RAM
- SQLite 版本: 3.45.1
- 只对比了 SQLite(PostgreSQL/MySQL/MariaDB 未跑)

| 类别 | 查询 | sqlrustgo vs SQLite (SF=0.1) |
|------|------|------------------------------|
| **sqlrustgo 优势** | Q1, Q3, Q6, Q7, Q8, Q9, Q10, Q12-Q15, Q17-Q19, Q21 | 0.03x–0.69x(更快) |
| | Q6 | **460x faster** |
| | Q12 | **347x faster** |
| **sqlrustgo 劣势** | Q2, Q4, Q5, Q11, Q16, Q20, Q22 | 2.0x–359x 慢 |
| | Q20 | **359x slower** |
| | Q2 | **108x slower** |
| | Q5 | **54x slower** |

**注意**: 这是 **SF=0.1** 数据,不能外推到 SF=1。SF=1 上 sqlrustgo 性能**无可靠数据**。

### D. 文档内部矛盾点

`TPCH_QExecution_Analysis.md`:
- 第 681 行(Q5 性能汇总):"状态 ❌, 内存 18.1 GB"
- 第 697 行(Q21 性能汇总):"状态 ❌, 内存 18.9 GB"
- 第 700 行(底部声明):"通过: 22/22 (Q2/Q5/Q21 FIXED via PR #3550)"

**结论**: 文档自相矛盾,真实性全面存疑。

---

## 九、引用

- Issue #3643: [CRITICAL] v3.11.0 GA 治理真实性修正
- PR #3647: docs: SF=1.0 truth audit — correct false 22/22 PASS claims
- `docs/releases/v3.11.0/GOVERNANCE_TRUTH_AUDIT.md` (前次审计)
- `docs/releases/v3.11.0/GA_GATE_REPORT.md`
- `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md`
- `docs/releases/v3.11.0/perf/TPCH_QExecution_Analysis.md`
- `docs/releases/v3.11.0/perf/SF01_CROSS_ENGINE_BASELINE.md`
- `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs`
- `scripts/tpch/setup_sf1.sh`

---

*Generated by MiniMax-M3 governance audit. All claims must be re-verified by independent reviewer per Section 5.*
