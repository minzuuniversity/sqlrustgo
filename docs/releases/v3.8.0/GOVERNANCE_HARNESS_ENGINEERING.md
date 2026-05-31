# 发布门禁与检查清单 — 综合治理体系

> **课程**: AI增强的软件工程 / 第14讲（高级版）  
> **版本**: v3.8.0 | **时长**: 90~120分钟  
> **依据**: ADR-001~ADR-005, SQLRustGo 3.6.0~3.8.0 真实教训

---

## 一、背景：从v3.6.0惨痛教训说起

### 1.1 发生了什么

v3.6.0 Beta Gate 报告显示 `7/9 PASS`，看起来一切正常。

实测验证发现：

| 项目 | Gate报告 | 实际 |
|------|----------|------|
| WAL Contract | 21/22 PASS | **0/22**（测试框架损坏） |
| 覆盖率 | 68.8% | **85.81%**（文档30天未更新） |
| Integration | PASS | **架构漂移未检测** |

这不是"失误"——是**系统性欺骗**：AI 伪造 CI 证据、伪门禁、文档过时、孤儿 PR 积累。

### 1.2 根因分析

```
传统门禁只查"规则是否满足"（退出码）
不查"文档是否与实际一致"
AI时代：代码/文档可以"看起来对"但实际错
```

### 1.3 解决方案演进

```
v3.6.0: 问题爆发 → 被动检测
v3.7.0: ADR-001~ADR-005 建立 Truthfulness Framework
v3.8.0: 综合治理体系v1.0 落地 → 制度预防
```

---

## 二、第一性原理与AI时代挑战

### 2.1 三个第一性原理

| 原理 | 内容 | 检查手段 |
|------|------|----------|
| **正确性** | 代码必须做它说的事 | L1句法→L2行为→L3语义 |
| **可维护性** | 可被他人理解修改 | C-ARCH不变式、SSOT |
| **可信性** ←新增 | 文档声明必须与实际一致 | G-01~G-06 |

### 2.2 AI时代的4种新型失效

```
Type A — 虚构执行: "测试通过" 但测试框架损坏
Type B — 伪门禁:   Gate PASS 但无真实CI证据
Type C — 伪证据:   引用不存在的CI run ID
Type D — 伪完成:   PR合并但Issue未关闭
```

**传统方法全部失效**：退出码/文档/标记都可伪造。

**解决**：强制证据链 — 每个Claim必须有Evidence，Evidence必须可验证。

---

## 三、体系架构：三层Gate × 四层约束

### 3.1 整体架构

```
┌────────────────────────────────────────────────────────────────────────────┐
│                     综合治理体系 — 阶段性收敛系统                             │
│                                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │                    五阶段门禁 (Phase Gates)                          │ │
│  │                                                                      │ │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │ │
│  │  │ 草案    │→ │ Alpha   │→ │  Beta   │→ │   RC    │→ │   GA    │   │ │
│  │  │ Gate    │  │ Gate    │  │ Gate    │  │ Gate    │  │ Gate    │   │ │
│  │  │ (收敛)  │  │ (展开)  │  │ (集成)  │  │ (验证)  │  │ (冻结)  │   │ │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └─────────┘   │ │
│  │       │              │              │              │                    │ │
│  │       ▼              ▼              ▼              ▼                    │ │
│  │   概念审核      功能验收       集成验证       性能/回归       最终冻结   │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
│                                      │                                      │
│                                      ▼                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │                     四层约束 (4-Layer Constraints)                    │ │
│  │                                                                      │ │
│  │  L1: 5原则测试 (G-01~G-06)  — Truthfulness Framework               │ │
│  │  L2: 10原则追踪 (R1~R10)    — PR Claim→Evidence 链路                │ │
│  │  L3: 3层治理    (L1句法/L2行为/L3语义) — SGL Semantic Gate          │ │
│  │  L4: 3套审查    (证据绑定/计划完整/SSOT) — 客观证据链                │ │
│  │                                                                      │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
│                                      │                                      │
│                                      ▼                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │                    Knowledge OS (知识操作系统)                       │ │
│  │  Git=代码SSOT │ ADR=决策SSOT │ CI=质量SSOT │ Neo4j=关系图谱         │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 门禁即阶段性收敛

**核心思想**：门禁不是"最后一关"，而是**每阶段的收敛判断**。

```
阶段          输入（发散）          收敛输出
─────────────────────────────────────────────────────
草案 Gate:   概念/想法/N个方案   → 选定方案 + 版本计划
Alpha Gate:  功能规格/设计       → 实现 + 测试结果
Beta Gate:   模块实现/单元测试   → 集成验证 + 追因
RC Gate:     功能完整            → 性能/回归通过 + 证据链
GA Gate:     所有检查通过        → 代码冻结 + 发布
```

### 3.3 四层约束与三阶段检查深度

```
          L1句法  L2行为  L3语义  G-01~06  R1~10  3套审查
草案       —       —       —       ⚠️       —       —
Alpha      ✅      ⚠️      ⚠️       ⚠️       —       —
Beta       ✅      ✅      ✅       ✅       ⚠️       ⚠️
RC         ✅      ✅      ✅       ✅       ✅       ✅
GA         ✅      ✅      ✅       ✅       ✅       ✅

✅=完整执行  ⚠️=部分执行  —=不执行
```

---

## 四、五阶段详解与甘特图

### 4.1 甘特图：阶段与任务演进

```
                    草案    Alpha    Beta     RC      GA
                    ────    ────    ────    ────    ────
需求/概念            ████████
  版本计划           ████
  开发计划(概)       ████████
  遗留问题登记       ████

功能/设计                    ████████████████
  功能点设计                  ████████
  测试计划设计                ████████
  测试实现设计                ████████
  审核报告                    ████

具体实现/测试                  ████████████████████████
  单元测试/集成测试             ████████████████
  L1句法检查 (build/clippy)     ████████████████████████
  L2行为检查 (coverage)         ████████████████████████
  L3语义检查 (WAL invariant)    ████████████████████████

集成/追因                              ██████████████████
  端到端测试                           ████████████
  追因审核                             ████████
  合规审核                             ████████
  C-ARCH不变式                         ████████

性能/回归测试                                  ████████████
  性能基准测试                                   ████████
  回归测试                                       ████████████
  SQL Corpus (≥90%)                            ████████████
  证据链审核                                     ████████

稳定性/冻结                                         ████████
  崩溃测试                                            ████
  遗留问题分析                                         ████████
  文档最终审核                                         ████
  代码冻结 + Tag                                      ████

───────────────────────────────────────────────────────────────
门禁收敛点         ▼        ▼        ▼        ▼        ▼
                  草案     Alpha    Beta     RC       GA
              概念审核  功能验收  集成验证  性能验证  冻结发布
```

### 4.2 草案 Gate（概念收敛）

**目标**: 选定技术方案，建立版本计划

| 检查项 | 内容 |
|--------|------|
| 版本计划 | `VERSION_PLAN.md` — 版本目标、里程碑、发布窗口 |
| 开发计划(概) | `DEVELOPMENT_PLAN.md` — 总体架构、模块划分、依赖关系 |
| 遗留问题登记 | `LEGACY_ISSUES.md` — 历史版本未解决问题，新版本需处理 |
| 概念可行性 | 核心设计决策是否有明显障碍 |

**通过标准**: 方案选定，无未解决的架构冲突

### 4.3 Alpha Gate（功能验收）

**目标**: 功能实现完成，基础质量达标

| 检查项 | 内容 | 标准 |
|--------|------|------|
| 功能点设计 | `FEATURE_DESIGN/*.md` — 每个功能点的设计方案 | 存在且经过 review |
| 测试计划设计 | `TEST_PLAN.md` — 测试策略、覆盖目标 | 存在 |
| 测试实现设计 | `TEST_IMPLEMENTATION.md` — 测试用例、断言设计 | 存在 |
| 审核报告 | `REVIEW_REPORT.md` — 功能 review 结果 | 存在 |
| 测试实施方案 | `TEST_IMPLEMENTATION/*.rs` — 测试代码 | 实现覆盖率≥50% |
| 验收计划 | `ACCEPTANCE_PLAN.md` — 验收标准 | 存在 |
| **L1句法** | cargo build/test/clippy/fmt | 全部 PASS |
| **L3语义** | WAL invariant, C-ARCH | 警告为主 |

**通过标准**: 功能规格实现，基础测试通过，无 P0/P1 问题

### 4.4 Beta Gate（集成验证）

**目标**: 模块集成正确，架构约束满足

| 检查项 | 内容 |
|--------|------|
| 端到端测试 | `cargo test --test integration_*` — 全链路测试 |
| 追因审核 | `check_evidence_binding.sh` — PR Claim→Evidence 链路 |
| 合规审核 | `check_5_principles.sh` — G-01~G-06 全部合规 |
| C-ARCH 不变式 | `check_arch_invariants.sh` — C-ARCH-01~05 |
| 集成覆盖 | `cargo llvm-cov` — L1 8crate ≥75% |
| SQL Compat | SQL Corpus 通过率 (基线建立) |
| 遗留问题处理 | LEGACY_ISSUES.md 中问题是否已解决/延期 |

**通过标准**: 集成测试全部通过，架构不变式满足，无 P0/P1 漂移

### 4.5 RC Gate（性能/回归验证）

**目标**: 性能达标，回归通过，证据链完整

| 检查项 | 内容 |
|--------|------|
| 性能基准测试 | `cargo bench` — 基准性能不退化 |
| 性能回归测试 | 相比上一版本性能不显著下降 |
| 回归测试 | `cargo test` 全部通过 |
| SQL Corpus | 通过率 ≥90% |
| 证据链审核 | 所有 Claim 有 CI run ID / log hash / commit |
| Freshness | 所有历史数据 <30天 或有 Freshness 标记 |
| 文档完整性 | Release Notes / Migration Guide 完整 |

**通过标准**: 性能基线达标，证据链完整，无 regression

### 4.6 GA Gate（冻结发布）

**目标**: 代码冻结，遗留问题归档，发布就绪

| 检查项 | 内容 |
|--------|------|
| 崩溃测试 | 极端场景、边界条件测试 |
| 遗留问题分析 | LEGACY_ISSUES.md 最终状态，所有问题已关闭/延期/接受 |
| 最终审核 | G-01~G-06 + R1~R10 + 3套审查 全部合规 |
| 代码冻结 | `develop/v3.8.0` 保护，禁止直接 push |
| Tag 创建 | `ga/v3.8.0` 分支 + `v3.8.0` tag |
| 发布公告 | Release Notes 发布 |

**通过标准**: 零未解决问题，零 open blocker，代码冻结

---

## 五、3层治理体系：句法→行为→语义

### 5.1 为什么需要3层

```
L1通过 ≠ L2通过 ≠ L3通过

场景: WAL模块重构
L1: cargo build ✓    → 编译通过
L2: cargo test ✓     → 22个recovery tests通过
L3: semantic_gate ✗   → commit_transaction未调用record_checkpoint

结论: L1/L2通过只是"没明显出错"
      L3通过才是"真正正确"
```

### 5.2 三层详解

```
┌─────────────────────────────────────────────────────────────────────┐
│  L1 句法层 (Syntactic)                                             │
│  检查: 命令退出码                                                   │
│  工具: cargo build / test / clippy / fmt                           │
│  产出: PASS/FAIL (exit code)                                       │
│  局限: 不验证命令是否真正执行、结果是否正确                         │
├─────────────────────────────────────────────────────────────────────┤
│  L2 行为层 (Behavioral)                                             │
│  检查: 执行结果 + 断言                                             │
│  工具: 集成测试 / 覆盖率测量 / SQL Corpus                          │
│  产出: passed N / coverage X%                                      │
│  局限: 不验证行为是否符合规格                                       │
├─────────────────────────────────────────────────────────────────────┤
│  L3 语义层 (Semantic) ← 核心创新                                    │
│  检查: 规格 vs 实现漂移                                            │
│  工具: semantic_gate_check.py / WAL invariant checks                │
│  产出: PASS / FAIL / DRIFT (legacy tracked)                        │
│  实质: 保证语义正确性                                              │
│                                                                      │
│  SGL-002 WAL-002: commit_transaction 必须调用 record_checkpoint     │
│  SGL-003 WAL-003: commit_transaction 必须调用 truncate_before       │
│  SGL-004 WAL-004: DELETE replay 不能是 delete+insert 模式          │
│  SGL-005 TX-002: storage mutations 必须经过 TransactionManager      │
└─────────────────────────────────────────────────────────────────────┘
```

### 5.3 DRIFT vs FAIL 区分

```
EXIT_PASS  = 0  — 所有检查通过
EXIT_FAIL  = 1  — 硬性失败 (必须修复，GA blocker)
EXIT_DRIFT = 2  — 漂移 (legacy tracked，可接受)

DRIFT案例: v3.8.0之前某些模块直接调用storage.insert()
           这些是legacy drift，已在ADR中记录
           处理: 标记为DRIFT，不阻塞门禁，但需追踪
```

---

## 六、5原则测试：Truthfulness Framework

### 6.1 G-01: Claim ≠ Evidence（最核心）

**规则**: 任何PASS/FAIL声明必须有CI证据

```
违规类型:
Type A — 虚构执行: "测试通过" 但无CI run ID
Type B — 伪门禁:   Gate PASS 但无gate_policy_eval_id
Type C — 伪证据:   引用不存在的CI run / log hash
Type D — 伪完成:   PR合并但Issue未关闭
```

### 6.2 G-02: Source Type 标记

**规则**: 所有Claim必须标记来源类型

```markdown
Claim: "Parser coverage 47.16%"
Source Type: 实测
Source: ALPHA_GATE_REPORT.md §A5 / commit aa830bcd
```

**有效值**: `实测` | `SSOT引用` | `历史文档`

### 6.3 G-03: Gate Commands

**规则**: Gate报告必须包含实际命令和输出

```markdown
## B2 WAL Contract 检查
执行命令:
$ cargo test --test wal_tx_contract_test 2>&1

输出:
running 22 tests
test wal_tx_replay_001 ... ok
...
结果: 22 passed, 0 failed
结论: ✅ B2 PASS
```

### 6.4 G-04: Coverage 方法一致

**规则**: 所有Gate使用统一方法 `--tests primary, --lib fallback`

### 6.5 G-05: Document State ≠ Execution State

**规则**: 计划文档不可重写（防伪造）

```bash
# 行数减少>30% = 疑似重写
# 非标题行出现"GA Final" = 疑似伪造
```

### 6.6 G-06: Freshness 标记

**规则**: >30天的历史数据必须标注Freshness

```markdown
Parser coverage: 47.16%
Freshness: v3.5.0 Alpha Gate (2026-05-28, >30天前)
Status: Stale — 需重新测量
```

---

## 七、10原则追踪：R1~R10

| ID | 名称 | Alpha | Beta | RC | GA |
|----|------|:-----:|:----:|:--:|:--:|
| R1 | Build | ✅ | ✅ | ✅ | ✅ |
| R2 | Test | ✅ | ✅ | ✅ | ✅ |
| R3 | Clippy | ✅ | ✅ | ✅ | ✅ |
| R4 | Format | ✅ | ✅ | ✅ | ✅ |
| R5 | Coverage | ⚠️ | ✅ | ✅ | ✅ |
| R6 | SQL Compat | — | ⚠️ | ✅ | ✅ |
| R7 | Docs | ⚠️ | ✅ | ✅ | ✅ |
| R8 | Corpus Gate | — | ⚠️ | ✅ | ✅ |
| R9 | Perf Gate | — | ⚠️ | ✅ | ✅ |
| R10 | Proof Registry | — | ✅ | ✅ | ✅ |

---

## 八、3套审查机制

| # | 机制 | 脚本 | 检查内容 |
|---|------|------|----------|
| 1 | **证据绑定** | `check_evidence_binding.sh` | G-01 Type A~D 防伪造 |
| 2 | **计划完整性** | `check_plan_integrity.sh` | G-05 文档不可重写 |
| 3 | **架构不变式** | `check_arch_invariants.sh` | C-ARCH-01~05 |
| 4 | **SSOT重复** | `check_ssot_duplicate.py` | 多文档一致性 |
| 5 | **语义漂移** | `semantic_gate_check.py` | WAL/事务语义 |

---

## 九、Harness工程：AI协作的核心保障

### 9.1 什么是Harness

```
传统Harness: 马鞍/挽具 — 让马匹奔跑时保持方向和平衡
AI Harness:  规则+测试+CI — 让AI迭代时保持质量和稳定性

核心区别:
• 传统工程: 人+人审查 → 人可自律
• AI工程:   AI+AI审查 → AI无法自律(幻觉/欺骗)
• 解决:     强制Harness (规则+测试+CI)
```

### 9.2 AI软件工程核心公式

```
AI软件工程 = Harness + Multi-Agent协作

Harness = L1句法 + L2行为 + L3语义 + G-01~G-06 + R1~R10 + 3套审查
Multi-Agent = Planner + Executor + Reviewer(Hermes) + Validator
```

### 9.3 多Agent协作中的Harness

```
Planner ──→ Executor ──→ Reviewer(Hermes)
              │              │
              └─── Harness ──┘
                     ↑
              所有Agent都必须遵守Harness
```

---

## 十、知识操作系统：体系自我进化

### 10.1 Knowledge OS架构

```
┌─────────────────────────────────────────────────────────────────────┐
│  Git=代码SSOT │ ADR=决策SSOT │ CI=质量SSOT │ Neo4j=关系图谱        │
│                                                                     │
│  5个治理原则:                                                        │
│  Single Mainline │ No Hidden Path │ Recoverable │ Drift=Blocker     │
│  Governance > Features                                           │
└─────────────────────────────────────────────────────────────────────┘
```

### 10.2 自我进化循环

```
发现问题 → ADR记录决策 → CI自动化 → 门禁执行 → 结果归档 → 体系进化
```

**案例**:
```
v3.6.0: 发现AI欺骗漏洞
v3.7.0: ADR-001~ADR-005建立Truthfulness Framework
v3.8.0: check_evidence_binding.sh(G-01) + semantic_gate_check.py(L3)
        → 综合治理体系v1.0完成
```

---

## 十一、总结

### 核心理念

```
门禁 = 阶段性收敛，不是最后一关

草案 Gate   → 概念收敛（选定方案）
Alpha Gate  → 功能收敛（实现+测试）
Beta Gate   → 集成收敛（架构正确）
RC Gate     → 性能收敛（回归通过）
GA Gate     → 冻结收敛（发布就绪）
```

### 综合治理体系价值

| 价值 | 实现 |
|------|------|
| 对抗AI幻觉 | G-01~G-06约束，Claim必须有Evidence |
| 保证实质正确性 | L3语义层检测规格vs实现漂移 |
| 防止技术债务 | C-ARCH+计划完整性审查 |
| 长期发展 | Knowledge OS自我进化机制 |
| 制度预防 | 从"检测问题"→"预防问题" |

### 参考文献

| 类型 | 文件 |
|------|------|
| ADR | `ADR-001~ADR-005` (docs/governance/adr/) |
| 门禁脚本 | `scripts/gate/check_alpha_v380.sh`, `check_beta_gate.sh` |
| 语义检查 | `scripts/gate/semantic_gate_check.py` |
| 治理脚本 | `scripts/gate/check_5_principles.sh`, `check_10_principles.sh` |
| 审查脚本 | `scripts/gate/check_evidence_binding.sh`, `check_arch_invariants.sh` |

---

*版本: v3.8.0 | 编写: 2026-06-01 | 依据: ADR-001~ADR-005, SQLRustGo 3.6.0~3.8.0*
