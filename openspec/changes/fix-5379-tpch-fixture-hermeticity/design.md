# Design — TPC-H fixture 自包含化与 CI 执行完整性

> Issue: #5379 · 分支: `fix/5379-tpch-fixture-hermeticity` · 基线: `d8365606d69c`
> 上游依据: `AGENTS.md` §7/§8/§10、`ADR-001` G-01~G-10、`ADR-008` §Policy 1/2/3、
> `GATE_CONDITIONS.md` G2/G4、`GATE_SCRIPT_STANDARD.md:160`、`TEST_PHASES_STANDARD.md` §2

---

## 1. 架构总览

```
                        ┌──────────────────────────────────────────┐
   CI test job          │ Step A: prepare_tpch_fixture.sh          │
   (.gitea/workflows/   │   generate_tpch_sf.py --sf 0.01 --seed 42│
    ci.yml)             │   → $RUNNER_TPCH/tpch_fixture_manifest.json│
                        └───────────────┬──────────────────────────┘
                                        │ export TPCH_DATA_DIR
                                        ▼
                        ┌──────────────────────────────────────────┐
                        │ Step B: cargo test --all-features        │
                        │          --no-fail-fast                  │
                        │   446 个目标全部执行（门禁核对）          │
                        └───────────────┬──────────────────────────┘
                                        │
              ┌─────────────────────────┼─────────────────────────┐
              ▼                         ▼                         ▼
      ┌───────────────┐        ┌────────────────┐       ┌──────────────────┐
      │ 状态 PASS     │        │ 状态 FAIL      │       │ 状态 NOT-RUN     │
      │ 已断言且通过   │        │ 断言不成立     │       │ 数据/profile 缺失│
      │               │        │ → CI 红灯      │       │ → 披露，不计 pass│
      └───────┬───────┘        └────────────────┘       └────────┬─────────┘
              │                                                 │
              │ 断言的期望值来源（互不循环）                       │
              ├─ SQLite 进程内独立引擎（rusqlite）                │
              └─ 语义不变量（数据自身一致性）                      │
                                                                │
                        ┌──────────────────────────────────────────┐
                        │ Step C: check_test_target_execution.sh  │
                        │   declared 446 vs executed → 漏跑即 FAIL │
                        └───────────────┬──────────────────────────┘
                                        ▼
                        ┌──────────────────────────────────────────┐
                        │ Step D: 摘要三态披露（ADR-008 Policy 1） │
                        │ passed=N failed=M not_run=K             │
                        │ (declared_targets=D executed_targets=E) │
                        └──────────────────────────────────────────┘
```

---

## 2. 关键决策

### D1 — fixture 运行时生成，绝不入库

**决策**：CI 与本地均在测试前调用 `scripts/gate/prepare_tpch_fixture.sh` 生成 fixture，落在 runner 临时目录。

**依据**：
- `.gitignore:28` 全局禁 `*.tbl`；`:153` 记载用户明文规则「禁止 log, tbl, json 这类测试输出文件提交」；
  强制门禁 `scripts/gate/check_no_log_tbl_json.sh:84-85` 会对入库的 `.tbl` 直接报错
- `.gitattributes:3` 把 `*.tbl` 路由到 Git LFS，而该文件自身注释写明 gitea 无 LFS 端点 → LFS 路径不可用
- 仓库既有约定已是"生成而非提交"：`tests/common/tpch_wire_harness.rs:34-47` 明写
  「The TPC-H fixtures are generated, not committed: tests/data/tpch-sf001 is a symlink into /tmp,
  which does not survive a reboot」

**为何选 `scripts/gate/generate_tpch_sf.py` 而不是 `crates/bench/examples/tpch_data_gen.rs`**：

| 维度 | `generate_tpch_sf.py` | `tpch_data_gen.rs` |
|---|---|---|
| 确定性 | ✅ `random.seed(seed)`（默认 42） | ❌ 6 处 `rand::thread_rng()`，无任何 seed |
| 外键自洽 | ✅ partsupp/orders/lineitem 的 key 均落在父表范围 | ❌ `supplier.nationkey` 越界 25（nation 实为 0..24）、`orders.custkey` 写死 1500、小 SF 下 lineitem partkey/suppkey 孤儿 |
| CLI 契约 | ✅ 与调用方一致 | ❌ 生成器只认 `--scale`，`generate_tpch_data.sh:173-175` 传 `--sf` → clap 报错，该 backend **自引入起从未成功运行过**（`git log -S` 双向佐证） |
| 成本 | Python3，CI 已有 | 需编译 Rust example |

**规模因子选择**：`--sf 0.01` → region 5 / nation 25 / supplier 100 / customer 1500 / part 2000 /
partsupp 8000 / orders 15000 / lineitem 60000。
理由：足以让多路 join + 分组聚合有意义（SF=0.001 时 part 仅 200 行、lineitem 6000 行，
多路 join 的中间结果过小，掩盖真实缺陷）；同时 CI 时间可控。

### D2 — 统一 fixture 解析 + profile 概念

**决策**：新增 `tests/common/tpch_fixture.rs`，所有 fixture 依赖目标经由它解析。

```rust
/// fixture 目录解析结果
pub struct Fixture {
    pub dir: PathBuf,
    pub manifest: Manifest,   // seed / sf / row_counts / generator
}

/// 缺数据时的显式状态（不是 panic，也不是静默通过）
pub enum FixtureState {
    Present(Fixture),
    Missing { reason_code: &'static str, detail: String },
    ProfileMismatch { expected: String, actual: String },
}
```

解析顺序：`TPCH_DATA_DIR` 环境变量 → 仓库内候选目录 → `Missing`。
**候选目录不再包含 `tests/data/tpch-sf01` / `tests/data/tpch-sf001`**——这两个悬空符号链接在本变更中删除。

manifest 由 `prepare_tpch_fixture.sh` 写在 fixture 目录旁，含 `sf` 与各表**实际行数**
（用 `wc -l` 统计，不抄生成器理论值——两者可能不一致，必须以实际产物为准）。

**profile 守卫**：数据依赖型期望值（如 Q11「expect 80 rows」）标定在 dbgen SF=1 上。
当实际 fixture 的 `sf` 与标定值不符时，测试判 `NOT-RUN: reason=profile_mismatch expected=sf1 actual=sf0.01`，
**不改参考值、不删断言、不加 `#[ignore]`**。

### D3 — 三态模型与披露

| 状态 | 判定 | CI 计入 | 门禁 |
|---|---|---|---|
| PASS | 断言成立 | `passed` | — |
| FAIL | 断言不成立 | `failed`（CI 红） | — |
| NOT-RUN | 数据缺失 / profile 不匹配 | `not_run`（**不计 passed**） | 必须出现在 not-run 清单且带原因码 |

NOT-RUN 标记格式（机器可读，供 CI 摘要与门禁解析）：

```
TPCH-NOT-RUN: <cargo-target-name> reason=<code> detail=<...>
```

reason 码受控集合：`fixture_missing` / `profile_mismatch` / `known_difference` / `scale_layer_only`。

**为什么"打印后返回 Ok"不算违反 `AGENTS.md §8` #5**：
#5 禁止的是「把 skip 计成 PASS」。本方案下这些测试**不计入 passed**，
在摘要里以 `not_run=K` 单独披露，且 `not_run` 集合与 declared 集合的差集由
`check_test_target_execution.sh` 强制核对——**漏跑会被发现，not-run 会被看见**。
这与 ADR-008 §Policy 1 要求的「披露实际运行数」是同一件事。

### D4 — SQLite 独立预言机

**决策**：新增 `tests/common/sqlite_oracle.rs`，用**进程内** `rusqlite` 作独立 oracle。

**为何 SQLite 而不是 DuckDB**：
- `GATE_SCRIPT_STANDARD.md:160` 指定 oracle 为 "DuckDB/SQLite"；DuckDB 在本仓**全仓未落地**
  （`Cargo.toml` 无 `duckdb` 依赖，`tests/` 下的 "DUCKDB" 命中全是人工抄录的常量）
- `rusqlite` 已在根 package 依赖中（`Cargo.toml:252`，`features = ["bundled"]`）→
  SQLite C 源码被 vendored 编译，**CI 零外部依赖**即可运行
  （`tests/integration/tpch_compliance_test.rs:12` 已有 `use rusqlite::Connection` 先例）
- CI runner 必然具备 C 编译器：`rusqlite` 的 `bundled` 特性在 `cargo build --all-features` 必需步骤中编译通过

**核心约束（反自证）**：期望值**只能**来自 SQLite 或语义不变量，
**禁止**用本引擎的输出回填期望值（那是把当前行为固化成标准）。

**对照前的归一化**（两侧必须一致，否则比的是格式不是语义）：
NULL → `\0NULL`；整数 → 十进制字符串；浮点 → 定点字符串（消除 `80` vs `80.0` 差异）；
文本 → 原样；行序按全行排序后比较（除非查询带 ORDER BY，则保持查询序）。

**方言适配**：SQLite 不支持 `EXTRACT(YEAR FROM x)` / `YEAR(x)`。
适配层做**受限**改写（仅 `EXTRACT(YEAR FROM <expr>)` → `CAST(strftime('%Y', <expr>) AS INTEGER)`），
其余差异一律**不静默改写**，而是标记该用例为「未适配」并改用语义不变量断言。
适配清单必须显式落盘（见 `tests/baseline/tpch_known_differences.json` 同目录的适配记录），
**不允许**出现"没适配但也没记录"的用例。

### D5 — 语义不变量（第二道防线）

不依赖任何外部期望值的断言，用于 SQLite 不可移植或引擎结果本身存疑的查询。例：

| 查询形态 | 不变量 |
|---|---|
| `... GROUP BY k HAVING SUM(x) > 0` | 行数 == 同一 GROUP BY 无 HAVING 时的分组数 |
| `... GROUP BY k HAVING SUM(x) > <极大值>` | 行数 == 0 |
| `... GROUP BY k` + 求和 | 各分组聚合之和 == 整体聚合（浮点用容差） |
| 多路 join | 逐步 join 的行数与一次性 join 的行数一致（笛卡尔膨胀可判定） |
| 类型检查类（`diag_shipdate_type`） | 读出值的**类型**符合列声明（INTEGER/REAL/TEXT），而非具体数值 |

这类断言能杀死聚合漏项、HAVING 阈值反向、join 条件写错等变异。

### D6 — 已知差异登记

**决策**：`tests/baseline/tpch_known_differences.json` + `check_tpch_known_differences.sh`。

登记项 schema（每条差异）：

```json
{
  "case_id": "<target>:<query_id>",
  "target": "<cargo target name>",
  "query_id": "<短标识>",
  "expected_row_count": 80,
  "actual_row_count": 0,
  "oracle_row_count": 80,
  "issue": "5384",
  "owner": "liying",
  "since": "2026-10-20"
}
```

门禁规则（任一不满足 → FAIL）：
1. 每条必须有 `issue` 归属与 `owner`（禁止无主差异）
2. `case_id` 全局唯一
3. 条目数**不得超过上限**（默认 0 = 初始空表）→ 阻止已知差异无声增长
4. `actual_row_count == oracle_row_count`（差异已消失）→ 强制清理陈旧登记
5. `since` 必须是 `YYYY-MM-DD`

**范式对齐**：与 P16 的 `tests/baseline/gate_test_baseline.json` 同构（基线 + 违规检测 + 状态字段）。

### D7 — CI 执行完整性门禁

`scripts/gate/check_test_target_execution.sh`：

- declared = Cargo.toml 全部 `[[test]]` 段的 `path`
- executed = cargo 日志中的 `Running <path>`（需剥除 ANSI 色码——色码夹在 `Running` 与路径之间）
- `missing = declared − executed` → **非空即 FAIL**，逐个打印
- `executed_but_undeclared` → WARN（单元测试二进制不属于 `[[test]]`，不判失败）
- 退出码：`0` 全跑 / `1` 有漏跑 / `2` 用法或输入错误

**为什么这条门禁比"CI 变绿"更重要**：它把「静默漏跑」这一整类缺陷变成可被 CI 自身检出的失败。
本次之前 382 个目标从未执行却无人察觉——本门禁就是针对这一类问题的**通用防线**，
对仓库里后续新增的测试同样生效。

### D8 — 变异测试（证明断言真的能杀）

变异对象必须是**引擎实现**（`crates/executor` 等），不是测试代码——变异测试代码是自证。

| 变异 | 预期被哪些断言抓住 |
|---|---|
| `SUM` 聚合漏加一项 | 语义不变量（分组和 == 整体和）、SQLite 对照 |
| `HAVING` 阈值比较方向反转 | SQLite 对照、阈值 0 保留全部分组的不变量 |
| join 条件写错（`=` → `<>`） | SQLite 对照、join 行数不变量 |

变异实施前必须用 `gitnexus impact` 确认变异点的调用面（`CLAUDE.md` 硬性要求）。

### D9 — wire 语句范围（实施过程中新增）

**触发**：D2/D4 落地后，第一次在 file storage wire 路径上实跑 22 条标准查询时，
引擎进程被 OS **SIGKILL**（不是断言失败）——它把整个测试二进制一起带走。
fail-fast 关掉之后，这种死法依然会让后续目标无法执行，等于用新问题换掉了老问题。

实测（独立服务器进程 + 硬 RSS 守卫，见证据 §7.5）：

| 语句 | 现象 | issue |
|---|---|---|
| Q9 | 1 秒内 RSS 破 1.5 GB，进程被 OOM killer 杀死 | #5393 |
| Q18 | 服务器进程在语句执行中被杀（客户端 `Lost connection`） | #5393 |
| Q19 | wire 前端解析失败（括号 OR 组内的 `IN (…)`） | #5393 |

**决策**：

1. **不修改引擎**（与 D1~D8 同一原则：本 PR 只修测试基础设施）。三条语句由
   **进程内内存路径**承担全量覆盖（`eval_22_vs_sf01` 跑全部 22 条）。
2. 引入**共享 wire 语句范围** `tests/common/tpch_wire_scope.rs`：
   - `WIRE_SCOPE`：允许在 file storage 路径上执行的 19 条（逐条与独立 SQLite 预言机对照过）；
   - `WIRE_EXCLUSIONS`：被排除的 3 条，每条带**受控原因码**
     （`engine_resource_limit` / `engine_parse_error`）与 **issue 号**。
3. **收口点做在编译期**：`run_query_timed(client, qid, sql, timeout)` 的 `qid` 是必填参数，
   范围外的标准查询**不执行**，返回 `WireOutcome::Skipped`。
   于是「忘记排除某条重查询」不是评审问题，而是**编译不过**。
4. 单元测试强制 `WIRE_SCOPE ∪ WIRE_EXCLUSIONS == 22 条标准查询` 且两者不重叠 ——
   范围表过期时测试立刻报错。
5. 被跳过的语句由**每个测试函数**发一次 `TPCH-NOT-RUN` 标记，
   保持 CI 的 `passed = TEST_OK − NOT_RUN` 算术精确。

**为什么不用「运行时检查」**：运行时检查要靠人记得写；编译期收口点让不安全的执行
在类型系统里就写不出来 —— 这与 D7「让漏跑变成可被 CI 检出的失败」是同一条思路，
只是把关口从「日志」前移到了「编译」。

---

## 3. 文件清单

### 新增

| 文件 | 职责 |
|---|---|
| `scripts/gate/prepare_tpch_fixture.sh` | 运行时生成 fixture + manifest，幂等 |
| `scripts/gate/check_test_target_execution.sh` | declared vs executed 门禁 |
| `scripts/gate/check_tpch_known_differences.sh` | 已知差异登记合规门禁 |
| `scripts/gate/mutate_tpch_oracle_tests.sh` | 变异测试 harness |
| `tests/common/tpch_fixture.rs` | 统一 fixture 解析 + profile + NOT-RUN 标记 |
| `tests/common/sqlite_oracle.rs` | SQLite 独立预言机 + 归一化 + 方言适配 |
| `tests/common/tpch_wire_scope.rs` | **wire 语句范围**：允许执行的标准查询 + 被排除项（原因码 + issue）+ 单元测试（D9） |
| `tests/baseline/tpch_known_differences.json` | 已知差异登记表（初始为空；本轮登记 1 条：`tpch:Q13` / #5394） |

### 修改

| 文件 | 变更 |
|---|---|
| `.gitea/workflows/ci.yml` | `test` job：新增 fixture 准备步骤；`--no-fail-fast`；摘要三态披露；新增执行核对步骤；**更正 G1 步骤注释**（`check_stage.sh` 并不覆盖 TPC-H，见证据 §BLK-6 取证） |
| `tests/common/tpch_wire_harness.rs` | `start_sf001` 等改走统一 fixture 解析，缺数据不再 panic；**存储目录与 LOAD DATA 白名单分离**（BLK-1 死锁）；**合成唯一首列 + 装载后行数断言**（BLK-4，#5392 绕过）；`run_query_timed` 改为 qid 必填的**编译期收口点**（D9） |
| 13 个 `tests/integration/oracle/*.rs` | 补真断言（SQLite 对照 + 语义不变量） |
| 约 20 个数据依赖目标 | profile 守卫 → `NOT-RUN: reason=profile_mismatch` |
| 13 个 wire 路径目标 | 接入共享 wire 范围：范围外语句不执行并受控披露；断言改为「每条语句都有交代」 |
| `docs/governance/...`（如需） | 仅在本变更确实改变门禁定义时更新，走 DOC_CHECK_CORRECTION_RULES 7 步 |

### 删除

| 文件 | 原因 |
|---|---|
| `tests/data/tpch-sf01` | mode 120000 悬空符号链接 → `/tmp/tpch-sf01` |
| `tests/data/tpch-sf001` | mode 120000 悬空符号链接 → `/tmp/tpch-sf001` |

---

## 4. 不变量（验收时逐条核对）

1. 仓库内**不新增**任何 `.tbl` / fixture 数据文件（`check_no_log_tbl_json.sh` 仍 PASS）
2. `cargo test --all-features --no-fail-fast` 后，**declared 446 个目标全部执行**（执行核对门禁 PASS）
3. CI 摘要同时给出 `passed` / `failed` / `not_run` 三者，`not_run` **不计入** `passed`
4. 每个 NOT-RUN 都带受控原因码；不存在「无原因码的静默跳过」
5. 13 个零断言文件的 `assert!` 数 **> 0**，且期望值来源为 SQLite 或语义不变量，**无一条来自本引擎输出回填**
6. 已知差异登记表每条都有 issue + owner，条目数 ≤ 上限，无陈旧条目
7. 变异测试：所有注入变异 **CAUGHT**（SURVIVED = 0）
8. `cargo clippy --all-features -- -D warnings` 0 警告；`cargo fmt --check --all` PASS
9. 规模层（SF=1）**未**在本变更中接入，且已立独立 issue 跟踪

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 首次让 382 个目标真跑，暴露与 TPC-H 无关的既有失败 | 高 | 高 | 本迭代内分类（fixture / 真实缺陷 / 环境），真实缺陷分流为独立 issue，**不在本 PR 内修**（避免范围失控） |
| CI 运行时间显著增加（446 目标 + 60k lineitem fixture） | 高 | 中 | 实测时长；若超预算，先跑 `--no-fail-fast` 核对，再评估是否按 crate 分片 |
| SQLite 与本引擎浮点聚合存在末位差异 | 中 | 中 | 归一化 + 浮点容差（相对误差 1e-9）；容差必须在断言处显式声明 |
| 方言适配遗漏导致误判 | 中 | 中 | 适配只做白名单改写；未适配用例必须显式登记，否则门禁 FAIL |
| 已知差异登记表被当成"缺陷永久豁免" | 中 | 高 | 条目数上限 + 强制 issue 归属 + 差异消失即强制清理 |
| 13 个文件的断言工作量超出迭代容量 | 中 | 中 | Q11 族 5 个文件作为完整样板先落地并通过变异验证，其余按同一模板推进；未完成的显式列 issue，**不允许留下"看起来有断言但实际没断言"的中间态** |

---

## 6. 与既有治理的衔接

- **P16**（`check_gate_test_integrity.sh`）的 33 个门禁目标不受本变更影响：它们全部照常执行
  （本变更不新增 `#[ignore]`，不触碰门禁引用关系）
- 本变更**新增**两个门禁脚本，接入方式与既有门禁一致（`scripts/gate/*.sh` + 退出码语义）
- ADR-008 §Policy 2 的两个例外（`tpch_sf1_22_vs_3engines_test` 2026-09-01 已过期、
  `q17_small_order_shortage_perf` 2026-10-31 到期）**本轮不动**——它们属于规模层，
  由独立 issue + 例外续期处理
- `docs/governance/adr/ADR-008-exception-v311-tpch-sf1.md` 已过期的事实，
  在本变更的 RECORD 证据中显式记录（ADR-001 G-06 Freshness 要求）
