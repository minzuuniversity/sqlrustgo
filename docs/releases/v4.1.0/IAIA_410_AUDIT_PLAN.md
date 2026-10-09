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

最后一条尤其重要：**不能只凭代码阅读意见直接给项目判定**，也不能把「无法复现」当作
永久事实。本计划启动时，P0 issue #5167 报告的「`COUNT(*)` 恒为 0」在某一次构建下
无法复现；但第一阶段审计（见 §13）在真实 `mysql` 客户端下复现了该缺陷，且定位到
**全表扫描路径返回 0 列**这一更根本的根因。教训是：无法复现只说明「本次构建 + 本次
数据集没有触发」，不等于缺陷不存在；复现实验必须用真实入口而非进程内 API。

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

| # | 项目 | 优先级 | 归属 | 第一阶段状态 |
|---|---|---|---|---|
| 1 | 锁定 commit 与构建配置，`audit/` 纳入版本控制 | P0 | Auditor D | ✅ 已完成（`91ac4dcd48`） |
| 2 | 扫描 TODO / 空实现 / mock / 测试禁用 / 绕行代码 | P0 | Auditor A | ✅ 已完成（见 §13.1） |
| 3 | 复原 MySQL 客户端 → Storage 的生产调用图 | P0 | Auditor A | ✅ 已完成（见 §13.2） |
| 4 | 对 Join / EXISTS / GROUP BY / NULL 做 PostgreSQL 差分 | P0 | Auditor B | ⚠️ 受 #003 阻断：非 PK 路径返回 0 列 |
| 5 | 验证 DML → TxManager → WAL → Storage | P0 | Auditor B | ✅ 已完成（见 §13.5，证真） |
| 6 | 进程崩溃与恢复测试 | P0 | Auditor B | ✅ 已完成（见 §13.3，F9 confirmed） |
| 7 | 验证 B+Tree 索引是否真实参与查询与 DML | P0 | Auditor B | ⚠️ 受 #003 阻断 |
| 8 | 关键算子变异测试（含 11 条 REVIEW 债务定性） | P1 | Auditor C | ⏳ 待执行 |
| 9 | 对 v3.12 / v4.0 / v4.1 做功能与性能回归 | P1 | Auditor C | ⏳ 待执行 |
| 10 | 证据矩阵与独立 GA 审核报告 | P1 | Judge | ⏳ 待执行 |

第 4–7 项是主战场。其中第 4、7 项被 #003（非 PK 查询返回空结果集）阻断，须先修
复该缺陷才能继续做有意义的差分与索引验证。

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

---

## 13. 第一阶段执行结果摘要（2026-10-09）

> 本节是已执行的审计证据，不是计划。每条发现均经真实 `mysql` 客户端与磁盘文件验证，
> 状态为 `confirmed` 或 `refuted`。冻结提交 `91ac4dcd48a31354caa690cef4c65996cf4ea606`
> （`gitea252/develop/v4.1.0`）。审计在独立 worktree `/tmp/iaia410` 中进行，未修改主工作区。

### 13.1 静态扫描结果

在 `91ac4dcd48` 上执行：

| 指标 | 数值 | 说明 |
|---|---|---|
| `unimplemented!()` 在 `crates/*/src` | 6 处 | 全部集中在 `wal_transactional_facade.rs`（见 §13.2）与 `stats_provider.rs:77` |
| `todo!()` 在 `crates/*/src` | 0 处 | — |
| `unreachable!()` 在 `crates/*/src` | 多为 match 穷尽性 | 不属于占位实现 |
| `#[ignore]` 总数 | 168 处 | 其中 `crates/*/src` 仅 5 处，其余在测试目录 |

结论：生产代码不存在大面积 `todo!()` / `unimplemented!()`。v4.1.0 的问题不是"留了一堆
空函数"，而是下文 §13.2 / §13.3 / §13.4 描述的三类集成真实性缺陷。

### 13.2 AUTH-V410-002 — planner/executor 物理计划层整体未接入生产（F3 假集成）

**判定：`confirmed`。**

`crates/planner` 与 `crates/executor` 的物理计划层从未有过执行能力，且在生产路径上
零引用。证据：

1. **`PhysicalPlan` trait 没有 `execute()` 方法**（`crates/planner/src/physical_plan.rs:14-26`），
   只有 4 个元数据方法（`schema` / `children` / `name` / `as_any`）。
2. `SeqScanExec::execute()` 硬编码返回 `Ok(vec![])`（`physical_plan.rs:59-61`）；
   `IndexScanExec` 连 `execute()` 方法都没有（`physical_plan.rs:122-138`）。
3. `IndexScanExec::new` 全仓库唯一调用点是 `planner.rs:116`，位于 test-only 的
   `DefaultPlanner::select_scan` 内。
4. `mysql-server` 在 `Cargo.toml:11-12` 声明了 planner/executor 依赖，但
   `crates/mysql-server/src/lib.rs:1-24` 的 import 里一个都没有。
5. `DefaultPlanner` 全部构造点位于 `planner.rs:369-801`，而 `#[cfg(test)] mod tests`
   起于 `planner.rs:358`——生产代码零构造。
6. 7 处 `SeqScanExec::new(String::new(), ...)`（`planner.rs:238,242,246,250,254,258,262`）
   是不可达占位符，注释自承 `"DDL statements - handled differently"`。

**真实执行者**是根 crate 内约 1.1 万行手写解释器（`src/engine_select.rs` +
`src/engine_dml.rs`）。生产链路上唯一运行的"优化"是 `sqlrustgo_optimizer::decorrelate`
（子查询去相关，`src/engine_select.rs:934,943,967`）。

**运行时佐证**：真实 `mysql` 客户端下 `EXPLAIN SELECT * FROM t WHERE id=3` 返回完全空
的结果集（0 行 0 列），服务端日志 `send_result_set: 0 cols, 0 rows`。

**影响**：`crates/planner` + `crates/executor` 的约 1 万行代码目前是纯负债；**所有
针对 planner/optimizer 的测试通过率都不构成生产正确性证据**（这些测试运行的是生产
不可达的代码路径）。

### 13.3 AUTH-V410-001 — COMMIT 成功后崩溃导致数据永久丢失（F9 持久性假象）

**判定：`confirmed`。属 GA 阻断项（§6 一票否决第 2 条）。**

复现命令（真实 `mysql` 客户端）：

```sql
CREATE DATABASE aud; USE aud;
CREATE TABLE t (id INT PRIMARY KEY, val INT, name VARCHAR(32));
INSERT INTO t VALUES (1,10,'a'),(2,20,'b'),(3,30,'c'),(4,40,'d'),(5,50,'e');
BEGIN; INSERT INTO t VALUES (6,60,'f'); COMMIT;
-- 崩溃前: SELECT * FROM t WHERE id=6 返回 (6,60,f) ✓
```

`kill -9` 进程后重启，服务端日志：

```
WARN recovery_engine: skipping entry (tx_id=2, type=Insert, table_id=116):
  table "t" named by this entry does not exist in the target
INFO WAL recovery: total=5 committed_txns=1 rows_inserted=0 skipped=2
```

**磁盘证据**：`/tmp/iaia410-data/aud/t.json` 的 `rows` 数组只含 id=1..5，**id=6 不
存在**。已提交事务的 WAL entry 被跳过，数据永久丢失，且仅 WARN 不 ERROR，服务照常
"Ready to accept connections"。

**根因**：`crates/storage/src/engine.rs:1423-1441` 的 `insert_in_db` 默认实现用
`let _ = db;` 丢弃库名参数后转发到 `insert`，WAL entry 不携带 database 标识，
`RecoveryEngine` 按 table_name 回放时无库上下文。

**附带缺陷**：`SHOW TABLES` 在 `USE default` 下仍列出表 `t`，但 `USE default;
SELECT * FROM t` 返回 `ERROR 1146 Table not found: t`，且
`/tmp/iaia410-data/default/` 目录根本不存在——目录与元数据不一致。

### 13.4 AUTH-V410-003 — 非主键等值查询返回空结果集（F5 语义退化）

**判定：`confirmed`。属 GA 阻断项（§6 一票否决第 3 条：复杂 SQL 静默返回错误结果）。**

| 查询 | 服务端结果 | 客户端表现 |
|---|---|---|
| `SELECT * FROM t WHERE id=3`（主键） | 3 cols, 1 row ✓ | 正常显示 `3 30 c` |
| `SELECT id,val FROM t WHERE id=2` | 正常 ✓ | 正常显示 `2 20` |
| `SELECT SUM(val) FROM t WHERE id=1` | 1 col, 1 row ✓ | 正常显示 `10` |
| `SELECT COUNT(*) FROM t WHERE id=1` | 1 col, 1 row，值=1 ✓ | 正常显示 `1` |
| `SELECT * FROM t WHERE val=30`（二级索引） | **0 cols, 0 rows** ✗ | ERROR 2027 malformed packet |
| `SELECT * FROM t WHERE name='c'`（非索引） | **0 cols, 0 rows** ✗ | ERROR 2027 malformed packet |
| `SELECT * FROM t`（无条件全表扫描） | **0 cols, 0 rows** ✗ | ERROR 2027 malformed packet |
| `SELECT * FROM t WHERE id>2`（范围） | **0 cols, 0 rows** ✗ | ERROR 2027 malformed packet |
| `SELECT * FROM t WHERE id=1 OR id=2` | **0 cols, 0 rows** ✗ | ERROR 2027 malformed packet |
| `SELECT * FROM t WHERE 1=0`（应 0 行） | **0 cols, 0 rows** ✗ | ERROR 2027 malformed packet |
| `SELECT COUNT(*) FROM t` | 1 col, 1 row，**值=0** ✗ | 显示 `0`（表内 5 行） |

**判定**：只有 PK fast-path（`WHERE <pk_col> = <常量>`）能产出正确的列定义。任何需要
扫描路径的查询——无论有无 WHERE、无论条件真假、无论是否走索引——都返回 **0 列**。
`WHERE 1=0` 应返回 0 行但有 3 列，实际返回 0 列，说明**列定义构造本身**（而非行过滤）
失败。这是静默错误结果：不报错、不拒绝，而是返回错误的空结果集。

### 13.4.1 根因（第二阶段精确定位，Issue #5191）

初判「扫描路径缺陷」在第二阶段被证伪。**真实根因是握手选库未同步到 engine 的
`session_db`**，与扫描逻辑无关。

对照实验（同一连接内）：

```
USE d1; SELECT DATABASE()   -> ('d1',)     USE 正常
```

握手指定 `database='d1'`（`pymysql.connect(database=...)`）：

```
SELECT DATABASE()     -> ('default',)    ❌
SELECT * FROM t       -> 0 列 0 行        ❌
SELECT COUNT(*) FROM t -> 0              ❌
```

链条：握手选库只写 storage 的共享 `current_db`
（`crates/mysql-server/src/lib.rs:6541-6549` 非 TLS / `:6459-6467` TLS），
而 `ExecutionEngine::new` 把 `session_db` 初始化为 `DEFAULT_DATABASE` **常量**
（`src/execution_engine.rs:352-355`），且 engine 在握手选库**之后**才构造
（`:6557-6559` / `:6482-6484`）。`DATABASE()` 与
`substitute_current_database_in_statement` 都读 `self.session_db()`，于是整条连接按
`default` 解析，表查找落空 → 返回 0 列。

`USE` 之所以正常：`execute_use_database`（`src/execution_engine_methods.rs:733-746`）
**同时**写 storage 的 `current_db` 与 engine 的 `session_db`。**既有测试全部走 `USE`
这条唯一正常的路径**——全仓库除 `lib.rs` 内部外无任何测试设置握手 database 字段，
这是该缺陷长期未被发现的原因。

这也解释了 issue #5167「COUNT(*) 恒为 0」为何在不同库上下文下表现不同、曾「无法复现」。

**状态**：已修复并验证（PR #5195）。`COUNT(*)` 由 0 恢复为真实值，库隔离同时生效。

`SELECT COUNT(*) FROM t` 返回 0 而非 5，与 issue #5167 一致；本审计为该 Issue 提供了
运行时复现与根因方向。

### 13.5 AUTH-V410-004 — DML → TxManager → WAL → Storage 链路真实（证伪怀疑）

**判定：`refuted`。** 初版怀疑「`engine_dml.rs` 调 `*_in_db` 而 `WalStorage` 未
override，疑似绕过 WAL」不成立。

`StorageEngine` trait 默认实现（`engine.rs:1423-1441`）丢弃 `db` 后转发到已 override
的 `WalStorage::insert`（`wal_storage.rs:613` `log_insert`）、`delete`（`:662,668`）、
`update`（`:698-723`）。事务边界 `begin_implicit_dml_tx` / `commit_implicit_dml_tx`
（`src/execution_engine_methods.rs:1974,2028`）真实调用 `TransactionManager` 与
`commit_transaction_for`。崩溃前 COMMIT 的写入正确可见，WAL 文件生成。

**但存在两处真实结构风险**（非虚假实现）：
1. CLUSTERED 表在 `src/engine_dml.rs:2175-2177` 直接写内存态 `ClusteredTable`，**完
   全绕过 WAL**。
2. `*_in_db` 丢弃 `db` 是 §13.3 数据丢失的直接成因。

### 13.6 附带发现

1. **AOCI-CODE 发现的 casefold 冲突**：`docs/releases/v2.9.0/OPencode_STARTUP.md`
   与 `OPENCODE_STARTUP.md` 内容完全相同，仅文件名大小写不同。在 macOS / Windows
   大小写不敏感文件系统上会互相覆盖。`aoci init` 因此 fail-closed。
2. **`--executor-parallelism=N` 静默降级**：help 文本自承
   `Requires --features parallel-executor at build time to take effect
   (otherwise capped to 1 at runtime)`。CLI 接受参数但运行时静默降级为串行。
3. **`mysql-server` 默认数据目录 `/tmp/sqlrustgo-data`**（`main.rs:208`），重启即丢
   数据，多实例互相污染。

### 13.7 第一阶段结论

v4.1.0 在 `91ac4dcd48` 上存在 **2 项 GA 阻断**（F9 数据丢失 + F5 静默错误结果）和
**1 项重大假集成**（F3 planner/executor 未接入）。按 §6 一票否决，**当前状态不得
晋级 GA**。

第二阶段需先修复 §13.4（F5）以解除对 Join / 索引 / 差分测试的阻断，再修复 §13.3
（F9），最后处理 §13.2 的 planner 接入或明确废弃。

---

## 14. 第二阶段：整改跟踪（2026-10-09 起）

### 14.1 Issue 清单

| Issue | 分类 | 严重度 | 状态 |
|---|---|---|---|
| [#5191](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5191) | F5 握手选库未同步 `session_db` | P0 | **已修复**（PR #5195） |
| [#5192](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5192) | F9 崩溃后已提交数据丢失 | P0 / GA 阻断 | 待整改 |
| [#5193](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5193) | F3 planner/executor 未接入生产 | P0 | 待架构决策 |

### 14.2 #5191 整改结果

**根因**：`src/execution_engine.rs:352-355` 用 `DEFAULT_DATABASE` **常量**初始化
`session_db`，而 engine 在握手选库之后才构造。详见 §13.4.1。

**修复**：从 storage 读取当前库，并在 `storage` 被 move 进 struct 之前取值：

```rust
let session_db = storage.read().current_db();
```

按最小修改原则，`src/engine_builder.rs` 的 6 处 builder 未改动——其构造函数在生产
代码中无调用者。

**验证**（真实 pymysql 客户端，`--all-features`）：

| 查询（握手 `database='d1'`） | 修复前 | 修复后 |
|---|---|---|
| `SELECT DATABASE()` | `default` ❌ | `d1` ✅ |
| `SELECT * FROM t` | 0 列 0 行 ❌ | 2 列 3 行 ✅ |
| `SELECT COUNT(*) FROM t` | `0` ❌ | `3` ✅ |
| `SELECT * FROM t WHERE id=2` | 0 列 0 行 ❌ | 1 行 ✅ |

库隔离同时生效：`d2` 连接访问 `d1` 的表 → 正确报 `ERROR 1146`。

**新增回归测试** 3 条（`src/execution_engine_tests.rs`）：直接锁定回归点、端到端
列数/行数/`COUNT(*)` 值、会话库不随其他连接切换而漂移。

### 14.3 本轮的方法论教训

1. **初判被证伪**：第一阶段把 §13.4 归因于「扫描路径」，第二阶段的对照实验
   （同连接 `USE` 正常 vs 握手选库异常）证明扫描逻辑无问题，根因在库上下文。
   若不做这个对照，就会去修错的地方。
2. **变异验证在本例中的价值有限**：我确认全仓库无测试设置握手 database 字段，
   因此变异测试无论通过与否都不提供信息量。**更有价值的是直接问「这个缺陷为何
   长期未被发现」**——答案是测试全部覆盖在另一条路径上。
3. **未完成项**：`cargo test -p sqlrustgo --lib` 未跑完（crate 测试配置编译超
   10 分钟），新增测试的断言结果尚未验证，合入前需补跑。端到端验证已完成。