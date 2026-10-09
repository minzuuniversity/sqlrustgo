# SQLRustGo v4.1.0 实现真实性与系统集成审计计划（IAIA-410）

> **文档性质**: 审计计划，不是审计报告，也不是执行结果。
> **制定日期**: 2026-10-09
> **审计代号**: IAIA-410（Implementation Authenticity & Integration Audit）
> **当前阶段**: `ALPHA`，以 `docs/releases/v4.1.0/STAGE.yaml` 为 SSOT
> **source_agent**: mcode (MiniMax-M3.1-Flash-Preview)
> **source_run**: iaia-410-plan-20261009
> **assessed_commit**: 冻结时写入 `audit/v4.1.0/commit.txt`，不在本文预设

---

## 1. 为什么要做这次审计

代码量、模块数量、单元测试通过率、SQL-92 功能覆盖率，都**不能**回答「v4.1.0 到底
能不能用」。它们度量的是产出的多少，不是能力的真假。

一个能编译、能过测试、模块齐全的数据库，仍然可能在 Join 上返回硬编码结果、在索引查询
上静默退化成全表扫描、在 COMMIT 上绕过 WAL。这些在上述任何一项指标里都是绿的。

因此本次审计的核心命题不是「代码是否正确」，而是：

> **从真实客户端发起一次操作，能否证明它确实经过了设计要求的全部核心模块，得到正确
> 结果，并在故障与重启后保持所承诺的数据库语义。**

### 1.1 本审计不做什么

- 不重新评审代码风格、命名、文档质量
- 不以「未实现 SQL 标准某特性」本身作为缺陷 —— 教学型数据库允许明确记录的功能限制
- 第一阶段**只审计不修复**。修复在第二阶段，依据证据单独开 Issue 与 PR

### 1.2 与既有门禁体系的关系

本审计**不新建**一套与既有门禁竞争的机制。v4.1.0 已有 175 个门禁脚本
（`scripts/gate/*.sh`）、B2 per-binary 门禁、`ignore_registry`、变异探针。本次审计是
**在这些机制之上回答它们尚未回答的问题**：门禁全绿，是否等于实现是真实的。

已有的 `ALPHA_TO_BETA_GATE_PLAN.md` 定义 AB-01..AB-10「晋级必须全绿」。本计划回答的是
另一层问题：**那 10 条全绿时，绿的是否有意义**。

---

## 2. 本仓库已有的审计基础设施（先盘点，再动手）

审计计划最容易犯的错误是重复造轮子。以下设施**已存在**，本审计应当复用并在不足处补齐。

| 设施 | 位置 | 已核实状态（2026-10-09，`gitea252/develop/v4.1.0`） |
|---|---|---|
| 门禁脚本 | `scripts/gate/` | 175 个 `.sh` |
| B2 per-binary 门禁 | `scripts/gate/run_b2_per_binary.py` | 枚举 472 个测试目标 |
| B2 禁用清单 | 同上 + `check_beta_v3.12.0.sh`（同步守卫） | **89 个 binary 被禁用**，约 19% |
| 禁用原因登记 | `docs/releases/v3.12.0/b2-disabled-test-binary-registry.md` | 89 条，含归因 |
| `#[ignore]` 登记 | `tests/baseline/ignore_registry.json` | 树内真实 `#[ignore]` = 115，上限 150 |
| 变异测试 | `scripts/gate/mutation_runner.py` + `v4.1_mutation_specs.json` | 27 条 spec：16 KILLED / 11 REVIEW |
| 反伪造门禁 | `scripts/gate/check_anti_fabrication.sh` | 存在 |
| 证据绑定门禁 | `scripts/gate/check_evidence_binding.sh` | 存在 |
| 阶段 SSOT | `docs/releases/v4.1.0/STAGE.yaml` | 存在 |

### 2.1 盘点中发现的三个既有缺口

这三条是本审计的**起点**，不是结论，均需在审计中取得证据后定性。

**缺口 A：89 / 472 个测试 binary 不参与 B2 门禁。**
门禁在报告 PASS，但这 472 个里有 89 个从未被执行。已登记原因，可登记原因本身也可能
是错的（见 §2.2）。

**缺口 B：变异测试证据无法从新 checkout 复现。**
`tests/baseline/v4.1_mutation_specs.json` 被仓库根的 `*.json` 忽略规则排除
（`.gitignore:161`），**不在版本控制内**。经核实：

```
$ git cat-file -e gitea252/develop/v4.1.0:tests/baseline/v4.1_mutation_specs.json
fatal: path ... exists on disk, but not in 'gitea252/develop/v4.1.0'
```

后果：27 条 spec 与 16/11 的判定只存在于单机。任何人 clone 仓库后都无法重跑变异审计，
也无法复核这些结论。这本身就是 F6（测试伪通过）的一个变体 —— 审计结论本身不可复现。

**缺口 C：禁用清单的归因缺少复核机制。**
清单条目是「禁用时写下的一句归因」，此后无人复验。见 §2.2 的实例。

---

## 3. 已有的真实证据：禁用清单会积累错误的归因

这不是假设，是本审计启动前已经发生过的事。

`mysqladmin_e2e_test` 被 B2 禁用，登记原因是：

> `mysqladmin CLI e2e; pre-existing failure` / **Investigate mysqladmin e2e flow**

该 binary 在禁用清单里躺了很久。但 2026-10-09 的复核发现，缺陷**不在 admin 的 e2e
流程里**，而在所有 e2e 流程共用的客户端行解析器里：解析器用行包首字节猜行协议，而文本
协议中长度为 0 的字符串同样编码成 `0x00`，于是首列为空字符串的普通文本行被当成二进制
行解析，整条查询失败（`Binary row: data too short for null bitmap`）。

同时该 binary 的第二个缺陷是一个独立的测试脆弱性：它假定 `server.port+1` 无人占用，
并行下会被别的测试占掉，实测 3 次全量跑挂 1 次。

修复后 `mysqladmin_e2e_test` 8/8 通过，已移出禁用清单（89 = 原 90 − 1）。

### 3.1 这件事对审计的意义

1. **禁用清单是「未验证的假设」集合，不是已知事实集合。** 每条都需要在审计中重新定性：
   归因是否正确、现在是否还失败、修复后是否真的通过。
2. **本次那个 bug 是被变异审计发现的，不是被常规测试发现的。** 常规测试当时是绿的
   （`wire_client_version_is_non_empty` 等 7 条），绿的原因与缺陷无关。
3. 因此「禁用原因」与「重新验证」必须成对出现，不能只增不减。

---

## 4. 缺陷分类（F1–F10）

不应把所有简化实现都判为造假。真正危险的是：对外宣称具备某能力，实际既未完成其语义，
也未在运行时明确报错。

| ID | 类型 | 典型表现 | 严重程度 |
|---|---|---|---|
| F1 | 空实现 | `Ok(())`、返回默认结果、`unimplemented!()` | 严重 |
| F2 | 伪实现 | 硬编码查询结果、只处理固定样例 | 严重 |
| F3 | 假集成 | 模块存在，但生产主路径从未调用 | 严重 |
| F4 | 静默退化 | 索引查询自动退化为全表扫描且未说明 | 中至严重 |
| F5 | 语义退化 | 复杂 SQL 静默忽略条件、或错误处理 NULL | 严重 |
| F6 | 测试伪通过 | 只检查成功状态、不检查计算结果 | 严重 |
| F7 | 弱化门禁 | `#[ignore]`、跳过测试、改断言阈值 | 高风险 |
| F8 | 性能退化 | 算法复杂度恶化，原本可运行的查询超时 | 中至严重 |
| F9 | 持久性假象 | COMMIT 成功，但重启或崩溃后数据丢失 | **阻断发布** |
| F10 | 绕行架构 | 新增直接执行路径绕过 Binder / TxManager | 严重 |

### 4.1 必须区分的三种 fallback

这是分类时最容易出错的地方。同一个 `if index_available { ... } else { scan }`：

| 形态 | 判定 | 依据 |
|---|---|---|
| **明确声明的 fallback** | 合理 | 索引不可用时用正确的全表扫描，结果语义不变 |
| **语义错误的 fallback** | 严重缺陷（F5） | 谓词解析失败时**不施加过滤** —— 结果集变大 |
| **隐藏 fallback** | 严重缺陷（F3/F10） | 宣称走 WAL/索引，实际走无持久化保障的替代路径 |

判据是**结果语义是否被保持**，不是代码形状。审计时必须用外部可观测结果回答，不能只读代码。

### 4.2 证据链要求

每项核心能力必须拿到四类证据，缺一不可：

```
功能声明（Requirement） → 真实实现（Implementation） → 运行时调用（Trace） → 外部可观测（Evidence）
```

**类存在、函数存在、调用语句存在，都不等于该能力真实集成。**

典型需要专门排查的反模式：

- Optimizer 构建了优化计划，Executor 用的却是原始计划
- Binder 做了类型检查，实际执行路径绕过 Binder
- Index Manager 实现了 B+Tree，查询直接走 Table Scan
- DML 更新了内存表，未经 TxManager 与 WAL
- 协议层返回 OK，执行器实际失败或结果未持久化

审计方法：对每个生产入口（MySQL Server、REPL、CLI）建立**反向调用图**，再把
`#[cfg(test)]` 专用入口单独标注，找出**只在测试中被调用**的路径。

---

## 5. 四轮审计

### 第一轮：静态代码审计（只读，不运行数据库）

在冻结提交上扫描，产出可疑点清单。**搜索结果的数量没有意义，重要的是每个可疑函数的真实
调用关系。**

```bash
git fetch origin --prune
git switch develop/v4.1.0
mkdir -p audit/v4.1.0

git rev-parse HEAD > audit/v4.1.0/commit.txt
git status --short      > audit/v4.1.0/status.txt

# 空实现 / 占位 / 模拟 / 绕行
rg -n -i 'todo!\s*\(|unimplemented!\s*\(|TODO|FIXME|HACK|STUB|PLACEHOLDER|\
mock|dummy|fake|not implemented|fallback|bypass|workaround' \
  --glob '*.rs' > audit/v4.1.0/suspicious.txt

# 可能掩盖失败的默认返回值
rg -n 'Ok\(\(\)\)|Ok\(None\)|Ok\(vec!\[\]\)|Ok\(Vec::new\(\)\)|\
unwrap_or_default\(\)' --glob '*.rs' > audit/v4.1.0/default_returns.txt

# 测试侧风险
rg -n '#\[ignore|should_panic|assert!\(true\)' \
  --glob '*.rs' > audit/v4.1.0/test_risks.txt
```

`audit/v4.1.0/` 必须入库，否则重蹈 §2.1 缺口 B 的覆辙。

#### 静态审计的判读纪律

单看

```rust
pub fn optimize(&self, plan: Plan) -> Result<Plan> { Ok(plan) }
```

可能是合法的恒等优化器。**只有**当 v4.1.0 声称实现了 Join Reorder / Predicate
Pushdown / Index Selection，而这些规则从未进入生产路径时，才构成 F3。

同理 `pub fn checkpoint(&self) -> Result<()> { Ok(()) }` 本身不算缺陷；只有当它被当作
已实现的 WAL Checkpoint 使用、且无任何截断逻辑时才是。

---

### 第二轮：动态真实性验证（最重要的一轮）

**不得只使用 SQLRustGo 自己的测试作为判据。** 必须由外部黑盒驱动，并以 PostgreSQL /
SQLite / MySQL 作为语义参考。

| 测试 | 目标 | 通过判据 |
|---|---|---|
| **A 真实执行链路** | 经 MySQL 协议跑 Join / Group By / HAVING / 子查询 / NULL / 类型转换 | 与参考 DBMS 比较**列名、类型、行数、逐值、NULL 位置、错误类别**。只比行数不算通过 |
| **B 优化器真实性** | 分别禁用与启用优化规则 | 规则声称有效时，必须出现**不同但语义等价**的执行计划，且实际结果相同 |
| **C 索引与存储真实性** | 大规模数据 + 建索引 | 索引查询与全表扫描结果一致；访问页数、执行计划可观测；再测 UPDATE / DELETE / 重复键 / 范围 / 重启。**索引文件存在不算通过** |
| **D 事务与 WAL 真实性** | 双连接并发、提交、回滚、kill -9、重启恢复 | 已确认提交必须保留；未提交必须消失；无脏读、无部分落盘。**必须区分进程崩溃持久性与断电持久性** |
| **E 故障注入** | 对 WAL append / 索引读 / 页读注入错误 | 观察错误是否被传播、是否触发预期回滚，**还是被 `unwrap_or_default()` 吞掉** |
| **F 版本差分** | v3.12 / v4.0 / v4.1 同环境同数据集 | 正确性、耗时、资源、崩溃、执行计划逐项对比，定位首次退化的提交 |

#### 模块禁用试验（F3 的判定利器）

比「读代码确认被调用」更强的方法：**破坏模块，看系统是否出现设计预期的可观测反应**。

以 WAL 为例，在隔离测试构建中注入 failpoint，使 WAL append 直接返回错误，然后用真实
MySQL 客户端执行：

```sql
BEGIN;
INSERT INTO t VALUES (9001, 100);
COMMIT;
```

| 观察结果 | 判定 |
|---|---|
| COMMIT 明确失败，事务按预期终止 | 该路径确实经过 WAL；仍需另验原子性 |
| COMMIT 成功，且全程无 WAL 调用 | **高度怀疑绕过 WAL（F3）** |
| COMMIT 成功，重启后数据丢失 | **F9 持久性假象 —— 阻断发布** |
| COMMIT 报错，但部分记录永久落盘 | **原子性缺陷 —— 阻断发布** |

前提：设计规范确实要求该提交必须写 WAL，且 failpoint 覆盖了全部合法 WAL 实现。否则不得
据此直接定性。

同一手法可用于 Binder、Index Manager、Transaction Manager、Buffer Pool。

---

### 第三轮：反向审计测试系统

**这是本项目当前最需要补强的一环。**

判据很朴素：如果一个数据库项目不断增加测试，但实际错误仍由外部教学实验或真实客户端发现，
那么问题不只是覆盖率不足，而是**测试本身没有证明真实性与集成完整性**。

| 检验 | 方法 | 判断标准 |
|---|---|---|
| 覆盖率 | `cargo llvm-cov` | **生产入口与关键分支**是否被执行，而非仅测试辅助代码 |
| 变异测试 | 已有 `mutation_runner.py` | 注入的错误能否被测试发现 |
| 差分测试 | PostgreSQL / SQLite / MySQL | 输出是否与参考系统语义一致 |
| 集成覆盖 | Trace + Call Graph | 是否覆盖生产环境完整调用链 |

#### 必须排除的虚假通过率

- 测试只调 `Parser::parse()`，不调执行器
- 测试直接实例化内存 Storage，绕过真实磁盘存储
- 测试直接调内部 Rust 函数，绕过 MySQL wire protocol
- 测试用模拟 Transaction Manager，不覆盖真实事务管理器
- 测试只断言 `is_ok()`，不检查返回内容与持久化状态
- 失败测试被设为 `#[ignore]`，且 CI 统计中未列为排除项

#### 变异测试的现状与补齐方向

现有 27 条 spec 的结果是 **16 KILLED / 11 REVIEW**。其中 11 条 REVIEW 是审计债务 ——
它们代表「已声称的修复，没有任何测试能证明它被钉住」。这 11 条必须逐条定性：是补测试、
是承认功能未实现、还是判定为不可变异。

§2.1 缺口 B 要求先把 spec 文件纳入版本控制，否则上述工作无法被第三方复核。

---

### 第四轮：真实性评分与发布判定

100 分制 + 关键缺陷一票否决。**这是一套拟议的项目治理标准，不是行业统一标准**，
采用前需经 governance 批准。

| 维度 | 分值 | 审计内容 |
|---|---|---|
| 核心功能真实性 | 25 | Parser / Binder / Executor 与算子语义 |
| 系统端到端集成 | 20 | 真实入口、模块依赖、生产调用链 |
| 事务与数据可靠性 | 20 | ACID、WAL、故障恢复、索引一致性 |
| 测试与验证可信度 | 20 | 差分、变异、覆盖、CI 真实性 |
| 版本退化与性能 | 15 | 正确性回归、性能、算法复杂度 |

| 得分 | 工程结论 |
|---|---|
| 90–100 | 可作为稳定版候选，仍需通过发布阻断项 |
| 80–89 | 有条件 Release Candidate |
| 65–79 | 功能集成不足，暂不建议 GA |
| 50–64 | 需要专项架构与实现整改 |
| 0–49 | 存在重大真实性或可靠性风险 |

#### 一票否决项（不论总分）

1. 声称支持的核心事务语义无法实现
2. COMMIT 成功却发生不可接受的数据丢失
3. 复杂 SQL 静默返回错误结果，而不是拒绝执行
4. 生产执行链路绕过必要的权限、事务或持久化机制
5. 关键 CI 失败被隐藏，或发布报告声称通过但缺乏可复现实证

---

## 6. 角色分离

审计与开发必须分离。**原开发 Agent 不得同时负责认定自己的工作合格。**

| 角色 | 职责 |
|---|---|
| Auditor A | 静态分析、空实现、调用图 |
| Auditor B | 差分测试、事务、持久性 |
| Auditor C | CI、变异、版本退化 |
| Auditor D | 架构约束与需求追溯 |
| Independent Judge | 复核证据、重现失败、审查误报、**决定是否阻断发布** |
| Issue / PR / Gate | 保留 commit、命令、失败日志、修复后复测证据 |

### 6.1 执行纪律

- 所有 Agent 在**独立 worktree** 中工作
- **第一阶段只读**：不允许为了通过验证而修改源码、测试或断言
- 判定必须允许三种状态：`confirmed` / `refuted` / `unverified`

最后一条尤其重要：**不能只凭代码阅读意见直接给项目判定**。本计划启动时已经遇到一个
反例 —— P0 issue #5167 报告的「`COUNT(*)` 恒为 0」在当前 HEAD 无法复现（3 行与
10000 行、进程内与 wire 均正确）。若不实际复现就据此开工，会修一个不存在的缺陷。

---

## 7. 发现记录格式

每条发现统一格式，便于机械汇总与复核：

```yaml
id: AUTH-V410-001
module: transaction/wal
category: F3-fake-integration
severity: P0
status: suspected          # suspected | confirmed | refuted

commit: "<40-char-sha>"
production_entry: "<entrypoint>"
source_location: "<file:line>"

claim: "DML commit uses WAL"
evidence:
  static: "<call graph>"
  dynamic: "<runtime trace>"
  differential: "<observed result>"
  reproduction: "<test command + exit code>"

expected: "<expected behavior>"
actual: "<actual behavior>"
verdict: "unverified"
```

### 7.1 三个状态的区别

| 状态 | 含义 | 所需证据 |
|---|---|---|
| `confirmed` | 缺陷成立 | 静态 + 动态 + 可复现命令 |
| `refuted` | **经复核不成立** | 复现尝试的完整记录，包括为何不成立 |
| `unverified` | 证据不足 | 明确写出缺什么证据 |

`refuted` 必须保留记录。#5167 就是 `refuted` 的候选 —— 它不应当被静默丢弃，否则下一次
还会有人按同样的假设开工。

---

## 8. 优先执行的 10 项

| # | 项目 | 优先级 | 归属 |
|---|---|---|---|
| 1 | 锁定 commit 与构建配置，`audit/` 纳入版本控制 | P0 | Auditor D |
| 2 | 扫描 TODO / 空实现 / mock / 测试禁用 / 绕行代码 | P0 | Auditor A |
| 3 | 复原 MySQL 客户端 → Storage 的生产调用图 | P0 | Auditor A |
| 4 | 对 Join / EXISTS / GROUP BY / NULL 做 PostgreSQL 差分 | P0 | Auditor B |
| 5 | 验证 DML → TxManager → WAL → Storage | P0 | Auditor B |
| 6 | 进程崩溃与恢复测试 | P0 | Auditor B |
| 7 | 验证 B+Tree 索引是否真实参与查询与 DML | P0 | Auditor B |
| 8 | 关键算子变异测试（含 11 条 REVIEW 债务定性） | P1 | Auditor C |
| 9 | 对 v3.12 / v4.0 / v4.1 做功能与性能回归 | P1 | Auditor C |
| 10 | 证据矩阵与独立 GA 审核报告 | P1 | Judge |

第 4–7 项是主战场：这几条链路既有历史缺陷背景，又能通过外部测试得到相对明确的真伪判定。

---

## 9. 工具边界

GitNexus / AOCI-CODE / CodeGraph 等代码认知工具，在本审计中承担**辅助**角色：

- 哪些模块只有定义、没有生产调用者
- v4.0 → v4.1 之间哪些调用路径发生了变化
- 哪些事务边界与架构约束可能被新代码绕过
- 哪些关键函数没有对应测试覆盖

**它们不能独立证明数据库行为正确。** 行为正确性只能由外部差分测试、运行时跟踪、故障注入
和独立测试回答。调用图能证明「函数被调用」，不能证明「调用产生的结果正确」。

---

## 10. 交付物

第一阶段结束时交付四个可量化结果，而非一份笼统的「代码质量报告」：

1. **实现真实性缺陷清单** —— 按 F1–F10 分类，每条带证据与三态判定
2. **生产调用链完整性矩阵** —— 每项核心能力：声明 / 实现 / 调用 / 可观测证据
3. **测试有效性报告** —— 覆盖率、变异、差分、集成覆盖四项，含 89 个禁用 binary 的重新定性
4. **跨版本退化矩阵** —— v3.12 / v4.0 / v4.1 的正确性、性能、执行计划对比

第二阶段才依据证据创建 Issue 与修复 PR。

---

## 11. 与阶段晋级的衔接

本计划**不改变** `STAGE.yaml`，也**不替代** `ALPHA_TO_BETA_GATE_PLAN.md` 的 AB-01..AB-10。

关系是：

```
IAIA-410  →  回答「AB-01..AB-10 全绿时，绿的是否有意义」
AB-01..10 →  在 IAIA-410 结论为「有意义」的前提下，回答「是否满足晋级条件」
```

若 IAIA-410 发现一票否决项，`STAGE.yaml` 的晋级条件必须先被满足 —— 无论 AB 全绿与否。

审计结论与晋级证据一样，必须遵循 `docs/governance/` 下既有的证据与文档治理规则
（同一冻结提交、可复现命令、明确退出码、证据哈希）。

---

## 12. 引用

- `docs/releases/v4.1.0/STAGE.yaml` — 阶段 SSOT
- `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md` — AB-01..AB-10
- `docs/releases/v4.1.0/DEV_PLAN.md` / `ISSUES_PLAN.md` / `TEST_PLAN.md`
- `docs/releases/v3.12.0/b2-disabled-test-binary-registry.md` — 89 条禁用登记
- `docs/releases/v4.1.0/evidence/MUTATION_AUDIT_5015_STALE_BINARY_2026-10-09.md`
- `docs/releases/v4.1.0/evidence/TEXT_ROW_EMPTY_VALUE_5179_2026-10-09.md`
- `scripts/gate/run_b2_per_binary.py` / `mutation_runner.py` / `check_gate_test_integrity.sh`
- `tests/baseline/ignore_registry.json`
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md`
- `docs/governance/DOC_CHECK_CORRECTION_RULES.md`