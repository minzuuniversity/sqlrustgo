# v4.1.0 测试计划

> **更新日期**: 2026-10-08
> **状态**: ACTIVE PLAN；本文定义测试范围和验收方法，不代表任何门禁已经通过。
> **阶段 SSOT**: `docs/releases/v4.1.0/STAGE.yaml`，当前为 `ALPHA`。
> **审计基线**: `gitea252/develop/v4.1.0@8bc4aac10c94ee9cd080faf644016c6c8ca126ca`

## 1. 目标

v4.1.0 的测试系统必须能够在进入 Beta 前发现 BustubX-EDU B 轨暴露的事务、
会话、多库隔离和 SQL 语义错误，同时证明门禁脚本自身不能静默放行失败。
测试结论只对同一冻结提交、同一环境和同一证据清单有效。

## 2. 分层测试模型

| 层级 | 目的 | 必须覆盖 | 主要产物 |
|---|---|---|---|
| L0 门禁自测 | 证明 gate 能发现失败 | 非零退出码、空结果、缺基线、忽略测试、损坏 workflow、mutation probe | gate 自测日志、退出码 |
| L1 单元/契约 | 局部算法和数据结构正确性 | parser/planner/executor/storage/transaction/catalog/optimizer/types | `cargo test` 日志、覆盖率 JSON |
| L2 确定性并发 | 事务和会话状态不串扰 | rollback、savepoint、MVCC、current database、多连接交错、8000 事务模型 | seed、调度、模型差异、复现统计 |
| L3 协议/E2E | 从客户端入口验证真实语义 | MySQL wire、BustubX-EDU B 轨、SQL corpus、结果 oracle | workload manifest、结果集 diff |
| L4 恢复/SOAK | 验证持续运行和故障恢复 | WAL、kill -9、重启、磁盘/网络异常、1h/24h/168h | heartbeat、恢复校验、manifest |
| L5 性能 | 建立可比较基线并阻止明显回退 | sysbench、Criterion、并发与事务批量场景 | 环境指纹、median/p95/p99、A/B 报告 |

任一层不得以 `ignored`、`filtered out`、空样本或 `|| true` 替代成功。

## 3. Alpha→Beta 必测矩阵

### 3.1 正确性与隔离

| 场景 | Alpha→Beta 验收 |
|---|---|
| 并发 rollback 隔离 | 30 次重复为一组，连续 10 组，失败率 `0/300` |
| 行数守恒 | 并发提交/回滚后结果与参考模型一致，连续 10 轮 |
| 8000 事务压力 | `0` 丢失、`0` 非预期 1062、最终 keyset 与模型一致 |
| savepoint | `integration_savepoint_test` 全部通过 |
| MVCC | `mvcc_transaction_test` 全部通过 |
| 多连接会话 | current database、事务所有权、prepared state 不跨连接泄漏 |
| 多库 DDL/DML | 建表、查询、UPDATE、DELETE、SHOW TABLES 均按目标库隔离 |
| wire 交错 | 两个以上真实连接的交错结果与顺序模型一致 |

上述场景须记录随机种子、并发度、循环次数和失败样本。一次偶然成功不能作为
稳定性证据。

### 3.2 BustubX-EDU 与 SQL 兼容性

1. 将 B 轨上机实验固化为版本化 workload manifest，记录 SQL、参数、前置数据和预期结果。
2. 对结果执行行数、排序后哈希和逐单元格差异检查；不得只判断“命令未报错”。
3. B 轨必测清单通过率必须为 100%，SQL corpus 通过率不得低于 80%。
4. 每个线上发现的缺陷必须补充 red-green 回归测试，并关联修复 PR。
5. 不支持的 SQL 必须明确分类为 parser、planner、executor、storage 或协议能力边界。

### 3.3 覆盖率

- Alpha→Beta 执行 `GATE_CONDITIONS.md` G17 的较严格阈值：L1_8 平均覆盖率 `>= 80%`。
- v4.1.0 的 GA 本地目标保持 `>= 85%`；parser 与 mysql-server 的既有证据必须在冻结提交复测。
- 覆盖率只接受真实执行成功的测试二进制；测试失败、零样本或解析失败均判 FAIL。
- `STAGE_CONFIG.yaml` 的 Beta 75% 与 G17 的 80% 存在漂移，整改 Issue #5113 负责统一。

### 3.4 SOAK 与故障恢复

| 阶段 | 最低要求 |
|---|---|
| Alpha→Beta | 1h OLTP SOAK 成功；进程、heartbeat、数据校验和退出原因齐全 |
| Beta | 24h 混合负载，包含多库、多连接和至少一次受控重启 |
| RC/GA | 168h 多模型 SOAK，并完成故障注入和恢复一致性校验 |

进程退出、heartbeat 中断、证据缺失或恢复后数据不一致均为 FAIL。历史报告或“文件存在”
不能替代本次冻结提交的实跑证据。

### 3.5 性能

- 固定硬件、工具链、features、数据规模、预热和运行次数。
- sysbench 至少覆盖 point-select、read-only、write-only、read-write；分别测试 autocommit
  和 1000 行显式事务。
- 每个场景至少 5 次，报告 median、p95、p99、离散度和错误数。
- 超过噪声区间后，吞吐下降 `>10%` 或 p99 延迟上升 `>20%` 时阻断晋级。
- 性能测试必须同时验证结果正确性；不能用错误但更快的实现建立基线。

## 4. 门禁与 CI 合同

Alpha→Beta 的详细门禁见 `ALPHA_TO_BETA_GATE_PLAN.md`。最低 CI 合同如下：

```bash
cargo fmt --check --all
cargo clippy --all-features -- -D warnings
cargo build --all-features
cargo test --all-features
bash scripts/gate/check_coverage_v312.sh
bash scripts/gate/check_sql_corpus_gate.sh
bash scripts/gate/check_anti_ignore_gate.sh
bash scripts/gate/check_gate_test_integrity.sh
```

CI 必须保留管道中测试命令的真实退出码。workflow 无法解析、required check 缺失、
被跳过或超时均不得记为 PASS。

## 5. 证据要求

每次晋级运行至少保存：

- `source_agent`、`source_run`、UTC/本地时间、冻结 commit、dirty 状态；
- 操作系统、CPU、内存、Rust 工具链、feature 集和配置哈希；
- 完整命令、开始/结束时间、退出码、stdout/stderr 路径；
- 测试数、失败数、ignored/filtered 数、覆盖率与性能统计；
- 每个产物的 SHA-256 和统一 `manifest.json`。

证据不得跨 commit 拼接。修复后必须重跑受影响门禁和完整 Alpha→Beta 套件。

## 6. Issue 映射

| 工作包 | Issue |
|---|---|
| 总控与晋级审核 | #5117 |
| 事务/会话正确性 | #5112；依赖 #5099、#5057、#5025 |
| CI 与门禁可信度 | #5113 |
| BustubX-EDU 回归 | #5114；依赖 #5103 |
| SOAK 与恢复 | #5115；依赖 #5102 |
| 性能基线 | #5116 |

## 7. 引用

- `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md`
- `docs/releases/v4.1.0/STAGE.yaml`
- `docs/governance/GATE_CONDITIONS.md`
- `docs/governance/STAGE_CONFIG.yaml`
- `docs/governance/adr/ADR-008-test-claim-transparency.md`
- `docs/governance/CI_GATE_CONTRACT.md`
