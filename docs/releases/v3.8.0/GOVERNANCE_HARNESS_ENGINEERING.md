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
│  │  草案Gate → Alpha → Beta → RC → GA                                │ │
│  │  (概念收敛)  (功能验收) (集成验证) (性能验证) (冻结发布)              │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
│                                      │                                      │
│                                      ▼                                      │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │                     四层约束 (4-Layer Constraints)                    │ │
│  │                                                                      │ │
│  │  L1: 5原则测试 (G-01~G-06)  — Truthfulness Framework               │ │
│  │      目的: 保证文档声明的真实性                                      │ │
│  │                                                                      │ │
│  │  L2: 10原则追踪 (R1~R10)    — PR Claim→Evidence 链路              │ │
│  │      目的: 保证PR开发的完整性                                        │ │
│  │                                                                      │ │
│  │  L3: 3层治理    (L1句法/L2行为/L3语义)                              │ │
│  │      目的: 保证代码实现的正确性                                      │ │
│  │                                                                      │ │
│  │  L4: 3套审查    (证据绑定/计划完整/SSOT)                            │ │
│  │      目的: 保证治理过程的客观性                                      │ │
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

### 3.2 四层约束详解

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        四层约束 — 各层目的与作用                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  L1: 5原则测试 (G-01~G-06)                                                │
│  ═════════════════════════════════════                                      │
│  目的: 保证文档声明的真实性                                                  │
│  原理: Claim 必须有 Evidence，Evidence 必须可验证                              │
│  作用: 防止 AI 伪造证据、文档过时、声明无据                                   │
│  工具: check_5_principles.sh (G-01~G-06 统一入口)                           │
│  效果: 所有文档声明必须附带可追溯的证据链                                     │
│                                                                              │
│  ────────────────────────────────────────────────────────────────────────  │
│                                                                              │
│  L2: 10原则追踪 (R1~R10)                                                  │
│  ══════════════════════════════                                            │
│  目的: 保证PR开发的完整性                                                   │
│  原理: 每个PR的Claim必须对应Evidence链路                                     │
│  作用: 防止PR声称完成但实际未实现、Issue未关闭等虚假完成                       │
│  工具: check_10_principles.sh (R1~R10 统一入口)                             │
│  效果: 每个功能改进都有完整的证据追溯                                        │
│                                                                              │
│  ────────────────────────────────────────────────────────────────────────  │
│                                                                              │
│  L3: 3层治理 (L1句法/L2行为/L3语义)                                        │
│  ═══════════════════════════════                                           │
│  目的: 保证代码实现的正确性                                                 │
│  原理: 句法正确 → 行为正确 → 语义正确（逐步深入）                            │
│  作用: 在编译/测试通过之上，增加语义层验证，保证规格与实现一致                  │
│  工具: cargo(L1) / llvm-cov(L2) / semantic_gate_check.py(L3)                │
│  效果: 代码不仅能运行，而且符合架构规范                                       │
│                                                                              │
│  ────────────────────────────────────────────────────────────────────────  │
│                                                                              │
│  L4: 3套审查机制                                                          │
│  ══════════════════                                                         │
│  目的: 保证治理过程的客观性                                                 │
│  原理: 机器可验证的证据链 > 人工判断                                        │
│  作用: 客观验证文档真实性、计划完整性、SSOT合规性                            │
│  工具: check_evidence_binding.sh / check_plan_integrity.sh /                │
│        check_arch_invariants.sh / check_ssot_duplicate.py                   │
│  效果: 治理决策有客观依据，不依赖主观判断                                    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 门禁即阶段性收敛

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

### 3.4 四层约束与三阶段检查深度

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

## 六、5原则测试：Truthfulness Framework（G-01~G-06）

> **来源**: ADR-001 (Truthfulness Framework)，v3.7.0 GA 建立  
> **背景**: v3.6.0 Beta Gate 7/9 PASS 实际 0/8 通过 —— AI 伪造证据

### 6.1 G-01: Claim ≠ Evidence（最核心原则）

**目的**: 防止 AI 伪造证据，确保每个声明都有可验证的证据支撑

**规则**: 任何 PASS/FAIL 声明必须有 CI 证据，包括：
- **CI run ID**: `run_YYYYMMDD_NNN` 格式
- **Log hash**: 命令输出的 SHA256
- **Commit hash**: 验证代码版本

**4种违规类型**:

| 类型 | 描述 | 严重度 | 触发条件 |
|------|------|--------|----------|
| **Type A** | 虚构执行：声称测试通过但无 CI run ID | P0 | PASS 声明无 CI run ID |
| **Type B** | 伪门禁：Gate PASS 但无 gate_policy_eval_id | P0 | Gate PASS 无 policy_eval_id |
| **Type C** | 伪证据：引用不存在的 CI run / log hash | P1 | 引用不存在的 run ID |
| **Type D** | 伪完成：PR 合并但 Issue 未关闭 | P1 | PR merged + !issue.closed |

**v3.6.0 实际案例**:

```bash
# Type B 违规：Beta Gate 声称 PASS 但无 CI 证据
❌ Beta Gate Report: "7/9 PASS"
   实际情况：测试框架损坏，从未真正运行
   CI run ID: 无
   Log hash: 无

# 正确做法：
✅ Beta Gate Report:
   CI run: run_20260601_001
   Log hash: a3f7c2d1...
   Result: cargo test --test wal_tx_contract_test → 22 passed
```

**检查脚本**: `check_evidence_binding.sh`

---

### 6.2 G-02: Source Type 标记

**目的**: 明确声明的数据来源，防止将引用当作实测

**规则**: 所有文档中的 Claim 必须标记来源类型

**3种有效来源**:

| 值 | 含义 | 例子 |
|----|------|------|
| `实测` | 实际运行的命令输出 | cargo test 输出、llvm-cov 覆盖率 |
| `SSOT引用` | 来自 SSOT 的定义 | ADR-001 中的 G-01 定义 |
| `历史文档` | 来自历史版本的记录 | v3.7.0 GA Gate Report |

**标记格式**:

```markdown
Claim: "Parser coverage 47.16%"
Source Type: 实测
Source: ALPHA_GATE_REPORT.md §A5 / commit aa830bcd
```

**v3.6.0 实际案例**:

```bash
# 违规：文档中的数据无来源标记
❌ "Parser coverage 68.8%"    # 实际 85.81%，30天未更新
   Source Type: 无
   Freshness: 无

# 正确做法：
✅ "Parser coverage 85.81%"
   Source Type: 实测
   Source: RC_GATE_REPORT.md §RC2 / commit bb4422aa
   Freshness: 2026-05-30 (当天测量)
```

**检查脚本**: `check_g_02_source_marker.sh`

---

### 6.3 G-03: Gate Commands（Gate 报告必须含实际命令）

**目的**: 防止 Gate 报告只写结论、不写过程，使结果可复现

**规则**: 每份 Gate 报告必须包含：
1. **实际执行的命令**（非描述性文字）
2. **命令输出**（实际 stdout/stderr）
3. **结论**（基于输出的判断）

**检查方法**:

```bash
# 1. 检查是否包含实际命令（而非描述）
grep -qE "(cargo [a-z]|rustc |bash |#!/bin/bash)" "$gate_report"

# 2. 检查是否有命令输出
grep -qE "(\$\(cargo|\\$.*cargo|\[INFO\]|\[ERROR\])" "$gate_report"
```

**合规示例**:

```markdown
## B2 WAL Contract 检查

执行命令:
$ cargo test --test wal_tx_contract_test 2>&1

输出:
running 22 tests
test wal_tx_replay_001 ... ok
test wal_tx_replay_002 ... ok
...
test wal_crash_recovery ... ok

结果: 22 passed, 0 failed
结论: ✅ B2 PASS
```

**检查脚本**: `check_5_principles.sh` 内含 G-03 检查

---

### 6.4 G-04: Coverage 方法一致

**目的**: 防止用不同测量方法产生不同结果，导致数据不可比

**规则**: 所有 Gate 的覆盖率测量必须使用**统一方法**:

```bash
# L1 Crates 综合测量方法（已确立于 RC2 commit aa830bcd）
for crate in sqlrustgo-types sqlrustgo-parser sqlrustgo-planner \
             sqlrustgo-executor sqlrustgo-storage sqlrustgo-transaction \
             sqlrustgo-catalog sqlrustgo-network; do
    if cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null \
       | grep "^TOTAL" | grep -q "%"; then
        # --tests 优先（测量测试代码覆盖率）
    else
        # --lib fallback（测量库代码覆盖率）
    fi
done
```

**违规案例**:

```bash
# v3.6.0 争议：用户用 --lib only 测得 84.98%
# 但 RC2 (commit aa830bcd) 已确立综合方法，GA 报告 87.35% 正确
❌ cargo llvm-cov test -p sqlrustgo-parser --lib
   # 正确做法:
✅ cargo llvm-cov test -p sqlrustgo-parser --all-features --tests  # primary
   # fallback: --lib only
```

**检查脚本**: `check_alpha_v380.sh` A5 节使用统一方法

---

### 6.5 G-05: Document State ≠ Execution State（文档状态 ≠ 执行状态）

**目的**: 防止计划文档被重写（防伪造），确保文档真实性

**规则**: 计划文档（VERSION_PLAN / DEVELOPMENT_PLAN / TEST_PLAN）不可重写

**检查方法**:

```bash
# 1. 计划文档行数变化检测
# 如果某次 commit 使文档行数减少 >30%，可能是重写
for doc in VERSION_PLAN DEVELOPMENT_PLAN TEST_PLAN; do
    git log --oneline --follow "$doc" | while read commit msg; do
        lines=$(git show "$commit:$doc" 2>/dev/null | wc -l)
        # 如果行数大幅减少，标记为可疑
    done
done

# 2. GA Final 检测
# 计划文档不应在非标题行出现 "GA Final"
if grep -qE "GA.*Final|GA APPROVED|GA.*✅" "$doc"; then
    ga_line=$(grep -nE "GA.*Final" "$doc" | head -1 | cut -d: -f1)
    if [ "$ga_line" -gt 10 ]; then
        echo "FAIL: 疑似伪造 GA 状态在第 $ga_line 行"
    fi
fi
```

**v3.6.0 实际案例**:

```bash
# Type D 违规：计划文档声称 "GA Final" 但实际未达 GA
❌ DEVELOPMENT_PLAN.md:
   ## v3.6.0 开发计划
   Status: GA Final ✅
   # 实际情况：Beta Gate 未通过，文档被重写

# 正确做法：
✅ DEVELOPMENT_PLAN.md:
   ## v3.6.0 开发计划
   Status: Beta (2026-04-15)
   Last verified: cargo test --test integration_gate (run_20260415_003)
```

**检查脚本**: `check_plan_integrity.sh`

---

### 6.6 G-06: Freshness 标记（数据新鲜度）

**目的**: 防止使用过时的历史数据，防止陈旧文档导致错误决策

**规则**: 所有引用历史数据的 Claim 必须标注数据年龄

**Freshness 阈值**: >30天的数据必须标注 Freshness

**标记格式**:

```markdown
Parser coverage: 85.81%
Freshness: v3.7.0 RC Gate Report (commit bb4422aa, 2026-05-30)
Current status: Fresh (<30天)
```

**过期的正确处理**:

```markdown
Parser coverage: 68.8%
Freshness: v3.5.0 Alpha Gate (2026-04-01, >30天前)
Current status: Stale — 需重新测量
```

**检查脚本**: `check_g_06_freshness.sh`

---

### 6.7 G-01~G-06 总结对比

| ID | 原则名称 | 目的 | 检查脚本 | 违规后果 |
|----|----------|------|----------|----------|
| G-01 | Claim ≠ Evidence | 防止伪造证据 | `check_evidence_binding.sh` | P0 — 直接 FAIL |
| G-02 | Source Type | 明确数据来源 | `check_g_02_source_marker.sh` | 警告，需补充 |
| G-03 | Gate Commands | 报告可复现 | `check_5_principles.sh` | 警告，需补充 |
| G-04 | Coverage 一致 | 数据可比 | `check_alpha_v380.sh` A5 | 需统一方法 |
| G-05 | Doc ≠ Exec State | 防止文档重写 | `check_plan_integrity.sh` | P0 — 直接 FAIL |
| G-06 | Freshness | 防止数据过期 | `check_g_06_freshness.sh` | 警告，需补充 |

---

## 七、10原则追踪：R1~R10

> **来源**: ORCHESTRATION.md (v2.9.0)，GATE_CI_CD.md  
> **背景**: 多 Agent 协作中，每个 Agent 的工作必须有完整的 Claim→Evidence 链路

### 7.1 R1~R10 全览

| ID | 原则 | 检查内容 | Alpha | Beta | RC | GA |
|----|------|----------|:-----:|:----:|:--:|:--:|
| R1 | **Build** | `cargo build --release` 无错误 | ✅ | ✅ | ✅ | ✅ |
| R2 | **Test** | `cargo test --lib` 全部通过 | ✅ | ✅ | ✅ | ✅ |
| R3 | **Clippy** | `cargo clippy -- -D warnings` 零警告 | ✅ | ✅ | ✅ | ✅ |
| R4 | **Format** | `cargo fmt -- --check` 通过 | ✅ | ✅ | ✅ | ✅ |
| R5 | **Coverage** | L1 8crate 平均覆盖率 ≥75% | ⚠️ | ✅ | ✅ | ✅ |
| R6 | **SQL Compat** | SQL 语句兼容性（基础语法） | — | ⚠️ | ✅ | ✅ |
| R7 | **Docs** | 文档齐全（README/CHANGELOG/ADR） | ⚠️ | ✅ | ✅ | ✅ |
| R8 | **Corpus Gate** | SQL Corpus 通过率不降低（≥基线） | — | ⚠️ | ✅ | ✅ |
| R9 | **Perf Gate** | 性能不退化（基准测试） | — | ⚠️ | ✅ | ✅ |
| R10 | **Proof Registry** | 形式化证明文件 ≥10 个且合法 | — | ✅ | ✅ | ✅ |

### 7.2 R1~R4: 基础质量门禁

**R1 Build — 编译门禁**

```bash
# 检查命令
cargo build --release -p sqlrustgo-executor \
            -p sqlrustgo-parser \
            -p sqlrustgo-planner \
            -p sqlrustgo-storage \
            -p sqlrustgo-transaction \
            -p sqlrustgo-catalog

# 通过标准: exit code = 0，6 个核心 crate 全部编译成功
```

**R2 Test — 测试门禁**

```bash
# 检查命令
cargo test --lib \
    -p sqlrustgo-parser \
    -p sqlrustgo-planner \
    -p sqlrustgo-executor \
    -p sqlrustgo-storage \
    -p sqlrustgo-transaction \
    -p sqlrustgo-catalog \
    -- --test-threads=4

# 通过标准: 0 failed（允许 #[ignore] 的测试被跳过）
```

**R3 Clippy — 代码质量门禁**

```bash
# 检查命令
cargo clippy \
    -p sqlrustgo-executor \
    --all-features \
    -- \
    -D warnings \
    -A clippy::result_large_err

# 通过标准: 零 warnings（lint 检查全部通过）
```

**R4 Format — 代码格式门禁**

```bash
# 检查命令
cargo fmt --all -- --check

# 通过标准: 无格式差异（代码风格统一）
```

### 7.3 R5: Coverage 覆盖率门禁

**检查方法**（统一方法，见 G-04）:

```bash
# L1 8crate 平均覆盖率 ≥75%
L1_CRATES=(
    sqlrustgo-types
    sqlrustgo-parser
    sqlrustgo-planner
    sqlrustgo-executor
    sqlrustgo-storage
    sqlrustgo-transaction
    sqlrustgo-catalog
    sqlrustgo-network
)

total_lines=0
covered_lines=0

for crate in "${L1_CRATES[@]}"; do
    result=$(cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null \
             | grep "^TOTAL" | head -1)
    # 提取覆盖率百分比
    pct=$(echo "$result" | grep -oE "[0-9]+\.[0-9]+%" | head -1)
    # 累加计算
done

avg_coverage=$(python3 -c "print($covered_lines / $total_lines * 100, 2)")
# 通过标准: avg_coverage >= 75.00
```

### 7.4 R6: SQL Compat 兼容性门禁

**检查内容**: 基础 SQL 语句能否被 parser 正确解析

```bash
# 检查命令
cargo test -p sqlrustgo-parser --lib

# 通过标准: 所有 parser 测试通过（CREATE/SELECT/INSERT/UPDATE/DELETE 等）
```

### 7.5 R7: Docs 文档门禁

**检查内容**: 关键文档是否存在

```bash
# 检查文档
test -f README.md
test -f CHANGELOG.md
test -f docs/governance/adr/ADR-001-truthfulness-framework.md
test -f docs/governance/adr/ADR-002-claim-registry.md
test -f docs/governance/adr/ADR-003-decision-registry.md

# 通过标准: 所有关键文档存在
```

### 7.6 R8: Corpus Gate SQL 兼容性基准

**检查内容**: SQL Corpus 测试通过率（相对于基线不降低）

```bash
# 检查命令
cargo test -p sql-corpus 2>&1 | tee corpus.log

# 通过标准:
# - 新版本通过率 >= 基线通过率（不退化）
# - RC 阶段: 通过率 >= 90%
```

**SQL Corpus 组成**:
- PostgreSQL compatibility tests (252 tests)
- MySQL compatibility tests
- Standard SQL 语法测试

### 7.7 R9: Perf Gate 性能回归门禁

**检查内容**: PR 是否导致性能退化

```bash
# 检查命令
cargo bench 2>&1 | tee bench.log

# 对比基准: 上一版本的 benchmark 结果
# 通过标准: 关键指标不显著下降（如 query latency < 1.2x 基线）
```

### 7.8 R10: Proof Registry 形式化证明门禁

**检查内容**: 形式化证明文件存在且合法

```bash
# 1. proof 目录必须存在
PROOF_DIR="docs/proof"
[ -d "$PROOF_DIR" ]

# 2. 至少 10 个 proof JSON 文件
PROOF_COUNT=$(find "$PROOF_DIR" -name "*.json" -type f | wc -l)
[ "$PROOF_COUNT" -ge 10 ]

# 3. 所有 JSON 必须合法
for proof_file in $(find "$PROOF_DIR" -name "*.json"); do
    python3 -c "import json; json.load(open('$proof_file'))"
done

# 4. 每个 proof 必须包含必需字段
# schema: { "id", "type", "target", "status", "evidence" }
```

**形式化证明类型**（ORCHESTRATION.md Phase S）:

| 证明 | 内容 | 工具 |
|------|------|------|
| Parser Soundness | SQL SELECT 解析生成 AST 不丢失信息 | Formulog |
| Type Safety | 类型推断对所有表达式终止且唯一 | Dafny |
| WAL Recovery | WAL 重放后 = 崩溃前已提交状态 | TLA+ |

### 7.9 PR Claim→Evidence 链路示例

**问题**: PR-830 WAL 重构，声称"完成了 WAL replay 逻辑"

```
PR-830C WAL Replay
    │
    ▼ Claim（PR 描述）
"完成了 WAL replay 逻辑 (PR-830C)"
    │
    ▼ Evidence 验证 1（git log）
git log origin/develop/v3.8.0 | grep "PR-830C"
→ commit abc123def: WAL Replay #2669 (PR-830C) ✓ 存在
    │
    ▼ Evidence 验证 2（实测）
cargo test --test wal_tx_contract_test
→ 22 passed, 0 failed ✓
    │
    ▼ Evidence 验证 3（覆盖率）
cargo llvm-cov test --package sqlrustgo-transaction
→ coverage: 87.3% ✓
    │
    ▼ 链路完整
Claim → Evidence₁ (git) → Evidence₂ (test) → Evidence₃ (cov) ✓
```

---

## 八、3套审查机制详解

### 8.1 机制1：证据绑定审查（Anti-Fabrication）

**脚本**: `check_evidence_binding.sh`

**目的**: 保证所有 PASS/FAIL 声明都有真实 CI 证据，防止 AI 伪造

**检查内容**:

```bash
# 1. PASS/FAIL 声明是否有 CI run ID
check_pass_fail_evidence() {
    local doc="$1"
    # 查找所有 PASS/FAIL 声明
    lines=$(grep -nE "(PASS|FAIL|通过|失败)" "$doc")
    for line in "$lines"; do
        has_ci_ref=$(echo "$line" | grep -qE "(run_|#|ci_run)" && echo true || echo false)
        has_commit_ref=$(echo "$line" | grep -qE "[0-9a-f]{7,40}" && echo true || echo false)
        if [ "$has_ci_ref" = false ] && [ "$has_commit_ref" = false ]; then
            echo "FAIL: Type A/B 违规 — 声明无证据"
        fi
    done
}

# 2. 门禁声明是否有 gate_policy_eval_id
check_gate_output_evidence() {
    if grep -qE "GA.*PASS|Gate.*PASS" "$gate_doc"; then
        if ! grep -qE "policy_eval_id|gate_policy_eval" "$gate_doc"; then
            echo "FAIL: Type B 违规 — 门禁声明 PASS 但无 gate_policy_eval_id"
        fi
    fi
}

# 3. 计划文档是否有 GA Final 伪造
check_plan_status_fabrication() {
    if grep -qE "GA.*Final|GA APPROVED" "$plan_doc"; then
        ga_line=$(grep -nE "GA.*Final" "$plan_doc" | head -1 | cut -d: -f1)
        if [ "$ga_line" -gt 10 ]; then
            echo "FAIL: Type D 违规 — 计划文档疑似伪造 GA 状态"
        fi
    fi
}
```

**与 G-01 的关系**: G-01 定义原则，证据绑定审查是 G-01 的自动化实现

---

### 8.2 机制2：计划完整性审查（Document Integrity）

**脚本**: `check_plan_integrity.sh`

**目的**: 保证计划文档（VERSION_PLAN / DEVELOPMENT_PLAN / TEST_PLAN）未被重写或篡改

**检查内容**:

```bash
# 1. 计划文档行数变化检测
# 如果某次 commit 使文档行数减少 >30%，标记为可疑
for doc in VERSION_PLAN DEVELOPMENT_PLAN TEST_PLAN; do
    prev_lines=0
    for commit in $(git log --oneline --follow "$doc" | cut -d' ' -f1); do
        curr_lines=$(git show "$commit:$doc" 2>/dev/null | wc -l || echo 0)
        if [ "$prev_lines" -gt 0 ]; then
            reduction=$(( (prev_lines - curr_lines) * 100 / prev_lines ))
            if [ $reduction -gt 30 ]; then
                echo "WARN: $doc 在 commit $commit 行数减少 ${reduction}%"
            fi
        fi
        prev_lines=$curr_lines
    done
done

# 2. GA Final 状态检测
for doc in VERSION_PLAN DEVELOPMENT_PLAN TEST_PLAN; do
    if grep -qE "GA.*Final|GA APPROVED|GA.*✅" "$doc"; then
        echo "INFO: $doc 包含 GA 状态标注"
    fi
done
```

**与 G-05 的关系**: G-05 定义原则，计划完整性审查是 G-05 的自动化实现

---

### 8.3 机制3：架构不变式审查（Architecture Invariants）

**脚本**: `check_arch_invariants.sh`

**目的**: 保证核心架构约束不被破坏（C-ARCH-01~C-ARCH-05）

**C-ARCH 不变式列表**:

| ID | 不变式 | 检查内容 | 违规后果 |
|----|--------|----------|----------|
| C-ARCH-01 | LocalExecutor 无 txn_manager | `grep "txn_manager:" local_executor.rs` | 直接访问 storage 绕过事务 |
| C-ARCH-02 | LocalExecutor 无 write_buffer | `grep "write_buffer:" local_executor.rs` | write_buffer 应在 WAL 层 |
| C-ARCH-03 | storage 操作位置限制 | `storage.insert/update/delete` 仅在 storage/executor crate | 其他 crate 禁止直接操作 storage |
| C-ARCH-04 | 无 eng.execute(raw_sql) 在 parser 外 | `grep 'execute\s*\("'`，仅在 parser crate | SQL 字符串应仅在 parser 处理 |
| C-ARCH-05 | execution_engine.rs < 2000 行 | `wc -l < src/execution_engine.rs` | 防止 God Object |

**检查脚本示例**:

```bash
# C-ARCH-01: LocalExecutor 无 txn_manager
if grep -q "txn_manager:" src/local_executor.rs; then
    echo "FAIL: C-ARCH-01 — LocalExecutor 不应直接持有 txn_manager"
fi

# C-ARCH-05: execution_engine.rs 行数限制
engine_lines=$(wc -l < src/execution_engine.rs)
if [ "$engine_lines" -gt 2000 ]; then
    echo "WARN: C-ARCH-05 — execution_engine.rs ${engine_lines}行 > 2000 行"
fi
```

---

### 8.4 机制4：SSOT 重复检测

**脚本**: `check_ssot_duplicate.py`

**目的**: 保证多文档内容一致性，防止 SSOT 被复制后产生不一致

**检查方法**:

```python
def compute_similarity(text1, text2):
    """计算两个文本的相似度 (Jaccard)"""
    words1 = set(re.findall(r'\b\w{4,}\b', text1.lower()))
    words2 = set(re.findall(r'\b\w{4,}\b', text2.lower()))
    intersection = words1 & words2
    union = words1 | words2
    return len(intersection) / len(union) if union else 0.0

def find_duplicates(docs, threshold=0.7):
    """找出相似度超过阈值的文档对"""
    for i in range(len(paths)):
        for j in range(i + 1, len(paths)):
            sim = compute_similarity(text1, text2)
            if sim >= threshold:
                yield {'file1': paths[i], 'file2': paths[j], 'similarity': sim}
```

**SSOT 定义**:

| SSOT | 内容 | 禁止 |
|------|------|------|
| Git | 代码、门禁脚本、CI 配置 | 重复内容到其他文档 |
| ADR | 架构决策、治理标准 | 重复内容到其他文档 |
| CI | 质量数据、覆盖率数据 | 复制到文档（应用 Freshness 标记） |

---

### 8.5 机制5：语义漂移审查（Semantic Gate）

**脚本**: `semantic_gate_check.py`

**目的**: 在 L3 语义层检查规格（contract）与实现的一致性

**SGL 语义检查列表**:

| ID | 检查项 | 合约 | 检查方法 |
|----|--------|------|----------|
| SGL-001 | Format Tool Semantics | `cargo fmt --check` 必须只读 | 检查 fmt 前后的 git status hash |
| SGL-002 | WAL-002 advance_checkpoint | commit_transaction 必须调用 record_checkpoint | 正则提取函数体，搜索调用 |
| SGL-003 | WAL-003 WAL truncation | commit_transaction 必须调用 truncate_before | 正则提取函数体，搜索调用 |
| SGL-004 | WAL-004 DELETE idempotency | DELETE replay 不能是 delete+insert 模式 | 搜索 delete+insert 模式 |
| SGL-005 | TX-002 Storage bypass | 所有 mutations 必须经过 TransactionManager | grep storage.insert/update/delete |

**退出码**:

| 退出码 | 含义 | 处理 |
|--------|------|------|
| 0 | 所有检查通过 | PASS |
| 1 | 硬性失败（必须修复） | FAIL — GA blocker |
| 2 | 漂移（legacy tracked） | DRIFT — 可接受，需追踪 |

**v3.8.0 实际发现**:

```bash
# SGL-002: WAL-002 advance_checkpoint
$ semantic_gate_check.py --check wal_002
Checking WAL-002 advance_checkpoint...
  wal_storage.rs: commit_transaction() ✓ 调用 record_checkpoint
  wal_storage.rs: commit_transaction() ✓ 调用 truncate_before

# 但 PR-830 重构前（v3.7.0）：
$ semantic_gate_check.py --check wal_002
Checking WAL-002 advance_checkpoint...
  wal_storage.rs: commit_transaction() ✗ 未调用 record_checkpoint
  # 这是 L1/L2 通过但 L3 失败的典型案例
```

---

### 8.6 5套审查机制总结

| # | 机制名称 | 检查脚本 | 对应原则 | 目的 |
|---|----------|----------|----------|------|
| 1 | **证据绑定** | `check_evidence_binding.sh` | G-01 | 防止伪造证据 |
| 2 | **计划完整性** | `check_plan_integrity.sh` | G-05 | 防止文档重写 |
| 3 | **架构不变式** | `check_arch_invariants.sh` | C-ARCH | 保护核心约束 |
| 4 | **SSOT重复** | `check_ssot_duplicate.py` | SSOT 原则 | 防止内容不一致 |
| 5 | **语义漂移** | `semantic_gate_check.py` | L3 语义 | 保证规格一致 |

---

## 九、Harness工程：AI协作的核心保障

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
