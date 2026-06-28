# Tasks — tpch-sf1-cross-engine-baseline-via-in-process

> **▶️ RESUMED 2026-06-27** (previously DEFERRED 2026-06-19)
>
> 延期原因（已消除）：Mac mini 磁盘满（仅 1GB 可用）。
> 当前硬件状态：磁盘 706GB 可用（`df -h /`），dbgen 已就绪于 `/home/openclaw/tpch-dbgen-master/dbgen`。
> 详见 Gitea issue #3423 评论区。

## 0. Pre-flight

- [ ] 0.1 确认工作树干净（`git status` 输出 clean）
- [ ] 0.2 确认分支 `feature/issue-3423-tpch-sf1-baseline` 已 checkout
- [ ] 0.3 确认 `/home/openclaw/tpch-dbgen-master/dbgen` 可执行且版本正确
- [ ] 0.4 确认磁盘剩余空间 ≥ 12 GB（SF=1.0 产物约 1.1 GB + 工作空间预留）

## 1. SF=1.0 fixture 生成

- [ ] 1.1 跑 `bash scripts/generate_tpch_data.sh --sf 1 --backend dbgen --output /home/openclaw/tpch_baseline/sf1` 生成 SF=1 数据集
- [ ] 1.2 用 `wc -l` 校验每个 `.tbl` 行数匹配 TPC-H spec：
  - region: 5, nation: 25, supplier: 10,000, customer: 150,000
  - part: 200,000, partsupp: 800,000
  - orders: 1,500,000, lineitem: 6,000,000
- [ ] 1.3 拒绝使用 `tpch_data_gen` 内置示例（已知 100x 缩放 bug，见 `crates/bench/examples/tpch_data_gen.rs`）

## 2. 集成测试

- [ ] 2.1 确认 `tests/tpch_sf1_22_vs_3engines_test.rs` 已就位（无需新建）
- [ ] 2.2 测试通过 `tests/common/tpch_wire_harness` + `MySqlTestClient` 复用 SF=0.01 测试模式
- [ ] 2.3 当 SF=1.0 fixture 缺失时测试为 `#[ignore]`（graceful degrade）

## 3. Baseline 脚本

- [ ] 3.1 创建 `scripts/tpch_sf1_baseline.sh`（chmod +x），执行：
  1. 断言 `/home/openclaw/tpch_baseline/sf1/*.tbl` 存在且行数匹配预期
  2. 启动 sqlrustgo ephemeral（`target/release/sqlrustgo-mysql-server serve --port <P> --data-dir /home/openclaw/tpch_baseline/sf1`）
  3. 对 `queries/q{1..22}.sql` 跑 22 个查询，记录行数与 wall-clock
  4. （可选）同样 22 个查询在 MariaDB（`mysql -h 127.0.0.1 -P 3306`）上跑并记录
  5. （可选）同样 22 个查询在 SQLite（`sqlite3` + `.import`）上跑并记录
  6. 写报告到 `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`
  7. 关闭 ephemeral
- [ ] 3.2 退出码：0=全成功 / 1=任何步骤失败（带 "STEP FAIL: <step> <err>" 消息）
- [ ] 3.3 添加 `--dry-run` 标志（仅打印计划不执行）

## 4. Baseline 报告

- [ ] 4.1 脚本生成 `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`，含 22 条目（Q1..Q22），每条记录：
  - 行数
  - wall-clock（per-engine：sqlrustgo / MariaDB / SQLite）
- [ ] 4.2 报告必须明确标注对比路径为 in-process `MySqlTestClient`，并注明 external-client 后续工作跟踪于 issue #3474
- [ ] 4.3 报告头部 metadata：测试日期、Scale Factor、数据源路径、SQLRustGo 版本

## 5. 验证

- [ ] 5.1 `cargo build --test tpch_sf1_22_vs_3engines_test` exit 0
- [ ] 5.2 `cargo test --test tpch_sf1_22_vs_3engines_test -- --include-ignored` 在 10 分钟预算内 22/22 PASS
- [ ] 5.3 `bash scripts/tpch_sf1_baseline.sh` exit 0 且产出报告
- [ ] 5.4 `cargo clippy --all-features -- -D warnings` exit 0
- [ ] 5.5 `cargo fmt --check --all` exit 0

## 6. 提交与推送

- [ ] 6.1 `git add` 新增的 `scripts/tpch_sf1_baseline.sh`、`docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`、`openspec/changes/2026-06-18-tpch-sf1-baseline/`
- [ ] 6.2 提交信息：`test(tpch): resume SF=1.0 cross-engine baseline (#3423)`
- [ ] 6.3 推送 `feature/issue-3423-tpch-sf1-baseline` 到 gitea250
- [ ] 6.4 关闭 issue #3423（引用 commit SHA + 报告路径）

## Done criteria (DoD)

- 所有 checklist 全部勾选
- `bash scripts/tpch_sf1_baseline.sh` 在干净树 exit 0
- `cargo test --test tpch_sf1_22_vs_3engines_test -- --include-ignored` 22/22 PASS
- 报告已 commit 并包含完整 22 query 数据
- Issue #3423 state_reason: completed
