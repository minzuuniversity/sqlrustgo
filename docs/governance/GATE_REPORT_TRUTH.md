# 门禁报告真实性 enforcement 规范

> **版本**: 1.0
> **创建日期**: 2026-05-25
> **目的**: 禁止门禁报告编造结果，所有检查项必须有真实执行证据
> **适用范围**: 所有 Phase Gate（Alpha/Beta/RC/GA）Report 文档

---

## 一、问题根因分析

### 1.1 为什么自查无效？

**直接原因**：GA_GATE_REPORT.md 声称"35/35 PASS"，但 G5~G12 大部分为 `⏳ PENDING` 状态，且有矛盾。

**根本原因**：自查机制是**合规性检查**（格式是否正确），不是**真实性检查**（内容是否属实）。

| 自查机制 | 检查内容 | 缺失 |
|----------|----------|------|
| 文档完整性检查 | 必选文档是否齐全 | 不检查内容是否属实 |
| 格式合规检查 | 是否符合模板格式 | 不验证数值是否真实 |
| 状态一致性检查 | ⏳ PENDING 状态存在 | 无触发机制要求消除 |
| Issue 关联检查 | Issue/PR 是否关联 | 不验证 Issue 编号是否正确 |

**核心缺陷**：从未问过这个问题 — *"你实际执行的输出是什么，与你写的相符吗？"*

### 1.2 问题根因

```
合规检查（格式）→ 自检通过 ✅
真相检查（真实）→ 从未执行 ❌
```

### 1.3 违规场景（v3.4.0 GA 为例）

| 违规类型 | 描述 |
|----------|------|
| **先写结论后补执行** | Report 声称"35/35 PASS"，实际 G5~G12 大部分为 `⏳ PENDING` |
| **无证据标记 PASS** | 检查项未执行，仅因"历史上通过过"就标记为 PASS |
| **Issue 编号错误** | Report 中 Issue 编号与 Gitea 不一致（#1360 实际为 #1361） |
| **摘要与详情矛盾** | 摘要表格写"35/35 PASS"，详情中大量 `⏳ PENDING` |

### 1.4 违反的原则

1. **GATE_SPEC_MASTER.md §1.2**: 门禁通过条件 = 全部通过，有豁免记录
2. **GATE_CHECKLIST_TEMPLATE.md §10.1**: 所有检查项必须记录结果
3. **SSOT 原则**: 文档和代码、测试以及完成状态必须保持一致

---

## 二、纠正和改进

### 2.1 技术层面 — 自动化交叉验证

问题：自查是"我检查我自己"，没有独立的真实数据来源。

改进：引入**第三方真实数据**作为基准，不依赖自我声明。

| 检查项 | 第三方真实数据来源 | 验证方式 |
|--------|-------------------|----------|
| G1 Build | cargo build exit_code | 必须为 0，不接受其他值 |
| G2 Test | cargo test 输出 `test result` | 必须包含 `0 failed` |
| G3 Clippy | cargo clippy exit_code | 必须为 0 |
| G5 Coverage | cargo llvm-cov JSON 输出 | 必须 ≥85% 且来自本次执行 |
| Issue 编号 | Gitea API 查询 | 必须存在于 Gitea |
| Commit SHA | git rev-parse HEAD | 必须与 Report 声明一致 |

**自动化验证脚本**（示例）：
```bash
# 验证 Report 中声明的 commit SHA 是否与实际一致
REPORT_SHA=$(grep "commit \`" GA_GATE_REPORT.md | head -1 | grep -oE '[0-9a-f]{8,}')
ACTUAL_SHA=$(git rev-parse --short HEAD)
if [ "$REPORT_SHA" != "$ACTUAL_SHA" ]; then
    echo "❌ SHA 不一致: Report=$REPORT_SHA actual=$ACTUAL_SHA"
    exit 1
fi

# 验证 Report 中声明的 test 结果
REPORT_PASSED=$(grep "G2.*Test" GA_GATE_REPORT.md | grep -oE '[0-9]+ passed')
ACTUAL_PASSED=$(cargo test --all-features --lib 2>&1 | grep "test result" | grep -oE '[0-9]+ passed')
if [ "$REPORT_PASSED" != "$ACTUAL_PASSED" ]; then
    echo "❌ TEST 结果不一致: Report=$REPORT_PASSED actual=$ACTUAL_PASSED"
    exit 1
fi
```

### 2.2 流程层面 — 分裂自查机制

问题：自查是"一个人完成所有检查"，没有独立复核。

改进：gate report 的审查人和执行人必须是**不同的人**。

| 角色 | 职责 | 禁止 |
|------|------|------|
| 执行人 | 执行门禁，填写 Report | 不能复核自己写的 Report |
| 审查人 | 独立验证 Report 真实性 | 不能是同一轮次的执行人 |
| 批准人 | 最终放行决策 | 必须看到审查人签字 |

### 2.3 文化层面 — 零容忍态度

问题：编造成本低（被发现只是"重新写一份"），导致执行人倾向走捷径。

改进：**形式主义 = 作弊**，不是"工作质量差"，是"职业道德问题"。

| 行为 | 性质 | 处理 |
|------|------|------|
| Report 中写入未执行的 PASS | 作弊 | 立即撤销，禁入下一轮 |
| 引用历史数据冒充本次执行 | 作弊 | 同上 |
| 已有 PENDING 却写 PASS | 作弊 | 同上 |
| Issue 编号与 Gitea 不符 | 疏忽 | 修正，不追究 |

### 2.4 机制层面 — 强制消除 PENDING

原流程：可以保留 `⏳ PENDING` 状态继续后续检查。

问题：`⏳ PENDING` 是状态污染源，最终混入 Report 导致不实。

改进：**任何 PENDING 状态必须立即消除**，才能继续后续 Gate：

```
PENDING → FAIL（立即创建 Issue）
       → SKIP（需要批准人签字豁免）
       → PASS（重新执行直到真实 PASS）
```

禁止：`PENDING → 假装 PASS → 混入 Report`

### 2.5 验证改进效果

引入 **Truthfulness Score**（真实性分数）：

| 分数 | 含义 | 处置 |
|------|------|------|
| 100% | 全部证据真实 | 通过 |
| 95-99% | 存在小错误（Issue 编号笔误等） | 修正后通过 |
| <95% | 存在实质性不实 | 重新执行门禁，禁入下一轮 |

---

## 三、Report 编写强制规则

### 3.1 证据格式（每项必填）

```markdown
| # | 检查项 | 命令 | 期望 | 实际 | 状态 |
|---|--------|------|------|------|------|
| G1 | Build | `cargo build --release --workspace` | exit=0 | exit=0, 0.58s | ✅ PASS |
```

**实际列的含义**:
- `命令`: 必须与 GATE_SPEC_MASTER.md 一致
- `期望`: GATE_SPEC_MASTER.md 定义的标准
- `实际`: **命令的真实输出**（exit_code、时间、输出摘要）
- `状态`: ✅ PASS / ❌ FAIL / ⏭️ SKIP / ⏳ EXECUTING

### 3.2 禁止的模式

```markdown
❌ 实际: 0.58s                          # 无命令输出，疑似编造
❌ 实际: 待测量                           # 未执行就写占位符
❌ 实际: ✅ PASS (历史数据)                # 引用非本次执行的结果
❌ 状态: ✅ PASS    实际: ⏳ PENDING      # 状态与实际矛盾
```

### 3.3 必须包含的证据片段

每个检查项的 Report 必须包含**命令输出摘要**:

| 检查项 | 必须包含的输出片段 |
|--------|-------------------|
| Build | `Finished 'release' profile` + 时间 |
| Test | `test result: ok. {N} passed; {M} failed` |
| Clippy | `Finished` 或 `warning` 数量 |
| Format | `diff_count: 0` 或 `X differences` |
| Coverage | `^TOTAL` 行或 `X%` 覆盖率数值 |
| Audit | `Auditing`, `Downloading` 行，或 `0 vulnerabilities` |
| TPC-H | `{passed}/22` 查询通过 + 耗时 |
| Bench QPS | `ops/s` 数值 |
| Docs | `broken_links: 0` |

### 3.4 证据嵌入位置

每个 Gate Report 文档 **必须** 在检查项表格后附上原始命令输出（使用 triple-backtick 代码块）:

```markdown
### G1 Build 证据

```
$ cargo build --release --workspace
   Compiling sqlrustgo-executor v0.1.0 (...)
    Finished `release` profile [optimized] target(s) in 0.52s
```

| # | 检查项 | 命令 | 期望 | 实际 | 状态 |
|---|--------|------|------|------|------|
| G1 | Build | `cargo build --release --workspace` | exit=0 | exit=0, 0.52s | ✅ PASS |
```

### 3.5 执行流程要求

```
1. 执行命令 → 2. 捕获输出 → 3. 解析结果 → 4. 写入 Report → 5. 关联 Issue（如果 FAIL）
```

**禁止跳过步骤 1-3 直接写步骤 4**。

---

## 四、Report 真实性检查清单（自检）

在提交 Gate Report PR 前，必须通过以下自检:

### 4.1 逐项核对

- [ ] 每项的状态（PASS/FAIL/SKIP）与实际列内容一致
- [ ] 每项的"实际"列包含本次执行的真实输出
- [ ] Issue 编号与 Gitea 查询结果一致
- [ ] 摘要表格与详情表格结果一致
- [ ] 所有 ⏳ EXECUTING 状态已解决

### 4.2 交叉验证

- [ ] Report 中的 commit SHA 与实际 git commit 一致
- [ ] 检查项数量与 GATE_SPEC_MASTER.md 对应版本阶段一致
- [ ] 命令与 GATE_SPEC_MASTER.md §2.2/3.2/4.2/5.2 一致
- [ ] 阈值与 GATE_SPEC_MASTER.md 定义一致

### 4.3 禁止提交条件

以下情况 **禁止提交** Gate Report PR:

- 存在任何 `⏳ PENDING` 或 `⏳ EXECUTING` 状态
- 状态列与实际列内容矛盾
- Issue 编号在 Gitea 上不存在
- 缺少命令输出证据代码块

---

## 五、违规处理

### 5.1 自检发现违规

发现违规后:
1. 停止当前 Gate 流程
2. 标记 Report 对应章节为 `❌ INVALID`
3. 创建 Issue: `[GATE REPORT] Report 内容不实 — {版本}`
4. 执行实际门禁检查
5. 更新 Report 到真实状态

### 5.2 外部发现违规

如外部（用户/审计）发现 Report 不实:
1. 立即标记对应 Gate 为 **FAIL**
2. 撤回已合并的 Gate Report（如已合并）
3. 重新执行门禁
4. 更新 GATE_EXEMPTIONS.md 记录事件

---

## 六、v3.4.0 违规记录

| 日期 | 事件 | 处理 |
|------|------|------|
| 2026-05-24 | GA_GATE_REPORT.md 声称 35/35 PASS，实际 G5~G12 大部分为 PENDING | 待重新执行门禁 |
| 2026-05-24 | CHANGELOG/LEGACY_ISSUES 中 Issue 编号错误（#1360 应为 #1361~#1369） | 已修正 |

---

## 七、变更记录

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-05-25 | 初始版本，针对 v3.4.0 GA Report 不实问题 |

---

*本文档是 SQLRustGo 门禁报告真实性的强制规范。*