# PR-5379 SPEC — TPC-H fixture 自包含化与 CI 执行完整性

> Issue: #5379 · 分支: `fix/5379-tpch-fixture-hermeticity` · 基线: `d8365606d69c`
> OpenSpec: `openspec/changes/fix-5379-tpch-fixture-hermeticity/`（proposal / design / tasks）
> 证据: `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md`
> 教训: `docs/governance/lessons/LESSON_5379_TPCH_FIXTURE_2026-10-20.md`

---

## 1. 问题陈述

Issue 标题是「TPC-H fixture 悬空符号链接导致 CI `test` job 恒红」。实测发现**问题面是五层**，
每层单独看都不严重，合起来让 CI 长期显示"接近全绿"的假象：

| 层 | 症状 | 实测证据 |
|---|---|---|
| 1 | 数据不可达 | `tests/data/tpch-sf01` → `/tmp/tpch-sf01`、`tests/data/tpch-sf001` → `/tmp/tpch-sf001`，均为 mode 120000 悬空符号链接；`git ls-files \| grep -c '\.tbl$'` = **0** |
| 2 | 382 个声明目标从未执行 | `cargo test` 默认 fail-fast，446 个 `[[test]]` 目标只跑了 64 个（**85.7% 从未执行**）；CI 摘要打 `Tests: passed=70 failed=1`，而 70 是**二进制数**不是测试数 |
| 3 | 绝大多数测试零断言 | 13 个文件只有 `eprintln!` 打印期望值；`four_way_harness.rs:174-185` 的 `compare_row_counts()` 是死代码；`check_g1_tpch_baseline.sh:139-147` 的"独立验证"实为重跑同一个测试 |
| 4 | fixture 现实度不足 | fixture 接上后实测：22 条规范查询里 **9 条结构性空转**（Q7/Q8/Q9 = 0 行，Q13 塌成 1 组，Q17/Q19 = NULL 聚合，Q18/Q20/Q22 = 0 行） |
| 5 | 共享 harness 的 `orders` schema 少一列 | `tpch_wire_harness.rs` 的 `orders` 只声明 8 列（漏 `o_totalprice`），而 `.tbl` 是 9 字段 → `o_orderstatus` 之后**每列静默错位一格** |

## 2. 范围

### 2.1 本 PR 做

- **B 信号层**：CI 内用仓库既有确定性生成器 `scripts/gate/generate_tpch_sf.py` 造 SF=0.01 fixture
  并设 `TPCH_DATA_DIR`；fixture **运行时生成、零入库**。
- 用**进程内 SQLite（rusqlite）** 作独立预言机（`GATE_SCRIPT_STANDARD.md:160` 要求 oracle 为
  DuckDB/SQLite；DuckDB 全仓未落地）。
- 13 个零断言文件补真断言（期望值来自 SQLite 或数据无关不变量）。
- CI 接线：`--no-fail-fast` + 超时闸 + **三态披露** + **目标执行核对门禁** + G1 改显式 NOT RUN。
- 修复本次迭代实测暴露的 3 个既有缺陷：生成器现实度、harness schema 缺列、`eval_22_vs_sf01` 加载器掉行。

### 2.2 本 PR 不做

- **规模层（SF=1 / dbgen）不接入** —— 留独立 issue **#5387** + ADR-008 例外续期。
- 不修 `q13_subquery_repro` 等目标暴露出的**引擎真实缺陷**（本轮只登记与分流）—— 见 **#5388**。
- 不动 `tests/integration/oracle/` 剩余 19 个零断言文件（`#[ignore]` 子集诊断类）—— 见 **#5391**。

## 3. 关键决策

| 编号 | 决策 | 理由 |
|---|---|---|
| D1 | fixture 运行时生成，绝不入库 | `.gitignore:28` 全局禁 `*.tbl`；`:153` 记载明文规则「禁止 log, tbl, json 入仓」；强制门禁 `check_no_log_tbl_json.sh:84-85` 对入库 `.tbl` 直接报错；`.gitattributes:3` 把 `*.tbl` 路由到 gitea 无端点的 LFS |
| D2 | 选 `generate_tpch_sf.py` 而非 `tpch_data_gen.rs` | 前者确定性（seed 42）+ FK 自洽；后者 6 处 `rand::thread_rng()` 无 seed、FK 孤儿、CLI 契约不匹配（只认 `--scale` 而调用方传 `--sf`）→ 该 backend **自引入起从未成功运行过** |
| D3 | SF=0.01 | region 5 / nation 25 / supplier 100 / customer 1500 / part 2000 / partsupp 8000 / orders 15000 / lineitem ~60000。SF=0.001 时 part 仅 200 行，会掩盖多路 join 缺陷 |
| D4 | 统一 fixture 解析 + profile 概念 | 缺数据是**显式状态**（NOT-RUN），不是 panic、不是静默通过；标定在别的 scale 的期望值走 `profile_mismatch` |
| D5 | 三态模型 | PASS / FAIL / **NOT-RUN**。NOT-RUN **不计入 passed**，必须带受控原因码 |
| D6 | 独立预言机 = 进程内 SQLite | rusqlite 已在根 package 依赖中（`Cargo.toml:252`，`bundled` 特性），CI 零外部依赖 |
| D7 | 跨引擎比对按**排序后多重集** | 无 ORDER BY 的查询行序是实现定义的；行序正确性由单独的不变量覆盖 |
| D8 | 生成器加 4 类生成时自检 | 键分布 / 谓词覆盖率 / 复合过滤覆盖率 / nation→region 映射。任一不过即 `exit 1`，不产出 fixture |
| D9 | manifest 记录生成器 SHA-256 | 只比 (sf, seed) 的话，生成器改了仍会 SKIP 并保留**陈旧 fixture** |
| D10 | 关 fail-fast 必须配超时闸 | 否则一个卡住的目标能吃掉整个 job（CI 容器上限 3h） |

## 4. 对外接口

| 接口 | 形态 | 兼容性 |
|---|---|---|
| `TPCH_DATA_DIR` | 环境变量，指向含 8 个 `.tbl` + manifest 的目录 | 新增，向后兼容（未设置 → NOT-RUN） |
| `TPCH_NOT_RUN_FILE` | 环境变量，NOT-RUN 标记的追加落盘位置 | 新增 |
| `tests/common/tpch_fixture.rs` | 共享模块 | 新增 |
| `tests/common/sqlite_oracle.rs` | 共享模块 | 新增 |
| `scripts/gate/*.sh`（4 个） | 门禁脚本，退出码语义与既有门禁一致 | 新增 |
| `tpch_wire_harness::{start_sf001, start_sf01}` | `Option<MySqlTestClient>`（原为直接返回） | **签名变更**（仅测试内部使用） |
| `tpch_wire_harness::SCHEMA_DDL` 的 `orders` | 8 列 → 9 列（补 `o_totalprice`） | **修正**，影响 12 个目标的列对齐 |

## 5. 验收不变量

1. 仓库内**不新增**任何 `.tbl` / fixture 数据文件（`check_no_log_tbl_json.sh` 仍 PASS）
2. `cargo test --all-features --no-fail-fast` 后，**declared 446 个目标全部执行**（执行核对门禁 PASS）
3. CI 摘要同时给出 `passed` / `failed` / `not_run` 三者，`not_run` **不计入** `passed`
4. 每个 NOT-RUN 都带受控原因码；不存在「无原因码的静默跳过」
5. 13 个零断言文件的 `assert!` 数 **> 0**，期望值来源为 SQLite 或语义不变量，**无一条来自本引擎输出回填**
6. 已知差异登记表每条都有 issue + owner，条目数 ≤ 上限，无陈旧条目
7. 变异测试：所有注入变异 **CAUGHT**（SURVIVED = 0）
8. `cargo clippy --all-features -- -D warnings` 0 警告；`cargo fmt --check --all` PASS
9. 规模层（SF=1）**未**在本变更中接入，且已立独立 issue 跟踪

## 6. 风险

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 首次让 382 个目标真跑，暴露与 TPC-H 无关的既有失败 | 高 | 高 | 本迭代内分类；真实缺陷分流为独立 issue（#5388），**不在本 PR 内修** |
| CI 运行时间显著增加（446 目标 + 60k lineitem fixture） | 高 | 中 | `timeout 7200` 兜底；G1 改 NOT RUN 后不再重复加载 fixture；wire 类目标实测耗时见 §7 |
| SQLite 与本引擎浮点聚合存在末位差异 | 中 | 中 | 归一化 + 相对容差 1e-9，容差在断言处显式声明 |
| 已知差异登记表被当成"缺陷永久豁免" | 中 | 高 | 条目数上限 + 强制 issue 归属 + 差异消失即强制清理 |
| `orders` schema 修正改变 12 个目标的列对齐，既有期望值失效 | **高** | 中 | 属"断言按旧（错位）数据标定"，分流为 #5388；本 PR 只修 schema，不改这些断言 |
