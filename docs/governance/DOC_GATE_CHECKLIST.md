# SQLRustGo 文档门禁检查清单 (DOC_GATE_CHECKLIST)

> **版本**: v2.0
> **创建日期**: 2026-05-29
> **更新日期**: 2026-05-29
> **规范来源**: `GOVERNANCE_INDEX.md` + `DOCUMENT_COMPLETENESS_CHECK.md` + `DOCUMENT_COMPLIANCE_AUDIT.md` (v3.4.0) + 历史版本 GOVERNANCE_AUDIT
> **用途**: 所有版本（v2.6.0+）发布前必须通过本文档检查

---

## 一、检查原则（Truthfulness）

1. **禁止 PENDING 占位符**: 门禁报告中的所有状态必须基于实测结果，禁止 `⏳ 进行中`/`PENDING` 等**未经验证的占位符**
   - **占位符 PENDING**: 数据未知，写了"待实测"/"PENDING"等填充格式（如覆盖率写 PENDING）→ VIOLATION
   - **进度追踪 PENDING**: 工作进行中，状态已知（如"设计进行中"/"待实现"作为工作流状态）→ **不违规**，常见于设计文档和工作追踪文档
   - **判断标准**: 如果文档声称某个 Gate/阶段"通过"但含有 PENDING 数据 → VIOLATION；如果 PENDING 表示工作进度而非占位符 → 合规
2. **禁止引用历史数据冒充本次执行**: 每次发布必须基于当次实测数据
3. **禁止缺失文档假装存在**: 如果某个文档不存在，必须标记为 `⏭️ SKIP` 或 `❌ MISSING`，不能假装存在
4. **禁止跨文档状态矛盾**: 同一版本不同文档中对同一门禁项的描述必须一致
5. **禁止标题与内容矛盾**: 报告标题写 PASS 但内容写 FAIL 的必须纠正

---

## 二、必须文档清单（所有版本）

### 2.1 版本文档（每版本独立目录 `docs/releases/vX.Y.Z/`）

| # | 文档 | 验证方法 | 缺失处理 |
|---|------|---------|---------|
| D1 | `README.md` | `[ -f README.md ]` | ❌ 阻塞 |
| D2 | `CHANGELOG.md` | `[ -f CHANGELOG.md ]` | ❌ 阻塞 |
| D3 | `DEV_PLAN.md` 或 `DEVELOPMENT_PLAN.md` | 两者之一存在即可 | ❌ 阻塞 |
| D4 | `TEST_PLAN.md` | `[ -f TEST_PLAN.md ]` | 🟡 建议（非阻塞） |
| D5 | `RELEASE_NOTES.md` | `[ -f RELEASE_NOTES.md ]` | ❌ 阻塞 |
| D6 | `ALPHA_GATE_REPORT.md`（如果执行了 Alpha） | `[ -f ALPHA_GATE_REPORT.md ]` | ⏭️ SKIP（如无 Alpha） |
| D7 | `BETA_GATE_REPORT.md`（如果执行了 Beta） | `[ -f BETA_GATE_REPORT.md ]` | ⏭️ SKIP（如无 Beta） |
| D8 | `RC_GATE_REPORT.md`（如果执行了 RC） | `[ -f RC_GATE_REPORT.md ]` | ⏭️ SKIP（如无 RC） |
| D9 | `GA_GATE_REPORT.md`（GA 版本） | `[ -f GA_GATE_REPORT.md ]` | ❌ 阻塞（GA 版本） |
| D10 | `GATE_SUMMARY.md` | `[ -f GATE_SUMMARY.md ]` | ⏭️ SKIP（非强制） |

> **注意**: v2.8.0 之前的版本（如 v2.0.0/v2.1.0）使用 `DEVELOPMENT_PLAN.md`，v2.6.0+ 使用 `DEV_PLAN.md`。两个名字均可，接受任一。

### 2.2 SSOT 文档（治理规范，根目录 `docs/governance/`）

| # | 文档 | 验证方法 | 缺失处理 |
|---|------|---------|---------|
| D11 | `gate_spec_v*.md`（当前版本对应规范） | `[ -f gate_spec_v*.md ]` | ❌ 阻塞 |
| D12 | `GOVERNANCE_INDEX.md` | `[ -f GOVERNANCE_INDEX.md ]` | ❌ 阻塞 |
| D13 | `GATE_REPORT_TRUTH.md` | `[ -f GATE_REPORT_TRUTH.md ]` | ⏭️ SKIP（参考文档） |

### 2.3 版本索引文档（根目录 `docs/releases/`）

| # | 文档 | 验证方法 | 缺失处理 |
|---|------|---------|---------|
| D14 | `VERSION_HISTORY.md` | `[ -f VERSION_HISTORY.md ]` | ❌ 阻塞 |
| D15 | `VERSION_ROADMAP.md` | `[ -f VERSION_ROADMAP.md ]` | ⏭️ SKIP（参考文档） |
| D16 | `VERSION_TREND.md` | `[ -f VERSION_TREND.md ]` | ⏭️ SKIP（参考文档） |

---

## 三、Truthfulness 检查（每个门禁报告必须自检）

| # | 检查项 | 命令 | FAIL 条件 |
|---|--------|------|----------|
| T1 | **无 PENDING 占位符** | `grep -c "PENDING\|⏳\|进行中" *.md` | count > 0 |
| T2 | **无"待实测"声明** | `grep -c "待实测\|待验证\|TBD" *.md` | count > 0 |
| T3 | **覆盖率数据为实测值** | `grep -E "[0-9]+\.[0-9]+%|覆盖率.*[0-9]" *.md | grep -v "^#"` | 包含 "PENDING" 或 "N/A" |
| T4 | **门禁结论与详情一致** | 摘要写 PASS 但详情写 PENDING/FAIL → FAIL | 不一致 |
| T5 | **标题与正文一致** | GA_GATE_REPORT 标题 "✅ PASS" 但结论 "FAIL" → FAIL | 不一致 |

**自检命令**：
```bash
# 在版本目录执行（如 docs/releases/v3.5.0/）
grep -n "PENDING\|⏳\|进行中\|待实测\|TBD" *GATE*.md
# 预期：无输出（0 matches）
```

---

## 四、链接完整性检查

| # | 检查项 | 命令 | FAIL 条件 |
|---|--------|------|----------|
| L1 | **内部相对链接无死链** | `bash scripts/gate/check_docs_links.sh entry` | exit_code ≠ 0 |
| L2 | **关键文档引用存在** | 验证 GATE_SUMMARY.md 中引用的 ALPHA/BETA/RC_GATE_REPORT.md 存在 | 文件不存在 |
| L3 | **无悬空相对路径** | `grep -rn "(\.\./[^)]*\.md)" docs/releases/vX.Y.Z/` | 输出 > 0 |

---

## 五、版本状态准确性检查

| # | 检查项 | 验证方法 | FAIL 条件 |
|---|--------|---------|----------|
| V1 | **当前版本与实际一致** | `grep "当前版本" VERSION_HISTORY.md` → 版本号 ≤ 实际最高 GA 版本 | 版本号 < 实际 |
| V2 | **CHANGELOG.md 包含最新版本** | `head -20 CHANGELOG.md` 包含当前版本号 | 不包含 |
| V3 | **VERSION_ROADMAP.md 更新日期** | `更新日期` 字段 ≥ 版本 GA 日期 | 日期早于 GA |
| V4 | **README.md 版本状态** | 版本已 GA 但 README 显示 "进行中" | 不一致 |

---

## 六、跨文档一致性检查（历史教训总结）

> 本节从 v3.4.0 GOVERNANCE_AUDIT.md 教训中提取，**所有版本适用**

### 6.1 Alpha Gate 一致性检查

| 检查项 | 验证文件 A | 验证文件 B | 一致性要求 |
|--------|-----------|-----------|----------|
| Alpha 通过数 | GOVERNANCE_AUDIT.md | ALPHA_GATE_REPORT.md | 数字必须一致 |
| Alpha 跳过项 | LEGACY_ISSUES.md 或 GOVERNANCE_AUDIT.md | ALPHA_GATE_CHECKLIST.md | SKIP ≠ PASS |

**验证命令**：
```bash
# 检查 GOVERNANCE_AUDIT 中 Alpha 数字与 ALPHA_GATE_REPORT 是否矛盾
alpha_gov=$(grep -A5 "Alpha" docs/releases/vX.Y.Z/GOVERNANCE_AUDIT.md | grep -E "[0-9]+/[0-9]+" | head -1)
alpha_rep=$(grep -E "[0-9]+/[0-9]+" docs/releases/vX.Y.Z/ALPHA_GATE_REPORT.md | head -1)
[ "$alpha_gov" != "$alpha_rep" ] && echo "❌ Alpha count mismatch: GOVERNANCE_AUDIT='$alpha_gov' vs ALPHA_GATE_REPORT='$alpha_rep'"
```

### 6.2 GA Gate Checklist vs Report 一致性

| 检查项 | 验证文件 A | 验证文件 B | 一致性要求 |
|--------|-----------|-----------|----------|
| GA 通过数 | GA_GATE_CHECKLIST.md | GA_GATE_REPORT.md | 数字必须一致 |
| GA 结论状态 | GA_GATE_REPORT.md 标题 | GA_GATE_REPORT.md 结论节 | 标题与结论必须一致 |

**验证命令**：
```bash
# 检查 GA 报告标题与结论是否矛盾
title=$(grep "^#.*GA.*PASS\|^#.*GA.*FAIL" docs/releases/vX.Y.Z/GA_GATE_REPORT.md | head -1)
conclusion=$(grep -A5 "^##.*结论\|^##.*总结" docs/releases/vX.Y.Z/GA_GATE_REPORT.md | grep "FAIL\|PASS" | head -1)
```

### 6.3 稳定性测试跳过合理性

| 检查项 | 验证文件 | 合理性要求 |
|--------|---------|----------|
| R-S1~S4 跳过 | RC_GATE_REPORT.md | 必须标注原因（scope 外/超时/环境限制） |

---

## 七、门禁文档命名规范（v3.2.0+ 强制）

> 从 DOCUMENT_COMPLIANCE_AUDIT.md §6.4 提取

| 文档类型 | 期望文件名 | 旧版/别名 |
|---------|-----------|---------|
| 开发计划 | `DEV_PLAN.md` | `DEVELOPMENT_PLAN.md`（v2.x 兼容） |
| Alpha 门禁清单 | `ALPHA_GATE_CHECKLIST.md` | — |
| Alpha 门禁报告 | `ALPHA_GATE_REPORT.md` | — |
| Beta 门禁清单 | `BETA_GATE_CHECKLIST.md` | — |
| Beta 门禁报告 | `BETA_GATE_REPORT.md` | — |
| RC 门禁清单 | `RC_GATE_CHECKLIST.md` | — |
| RC 门禁报告 | `RC_GATE_REPORT.md` | — |
| GA 门禁清单 | `GA_GATE_CHECKLIST.md` | — |
| GA 门禁报告 | `GA_GATE_REPORT.md` | — |
| 门禁总结 | `GATE_SUMMARY.md` | — |
| 治理审计 | `GOVERNANCE_AUDIT.md` | — |
| 文档合规审计 | `DOCUMENT_COMPLIANCE_AUDIT.md` | — |

**命名检查命令**：
```bash
# 检查是否使用了旧命名
for f in docs/releases/vX.Y.Z/*GATE*.md; do
  basename "$f" | grep -E "^[A-Z]+_GATE_" || echo "❌ 命名不规范: $f"
done
```

---

## 八、历史版本特殊要求

### 8.1 v3.3.0（文档最不完整版本）

**必须文档**（缺失清单来自 `DOCUMENT_COMPLIANCE_AUDIT.md` §3.6）：

| # | 文档 | 状态 |
|---|------|------|
| v330-1 | `ALPHA_GATE_CHECKLIST.md` | ❌ 缺失（历史遗留，无法补全） |
| v330-2 | `BETA_GATE_CHECKLIST.md` | ❌ 缺失（历史遗留，无法补全） |
| v330-3 | `RC_GATE_CHECKLIST.md` | ❌ 缺失（历史遗留，无法补全） |
| v330-4 | `GA_GATE_CHECKLIST.md` | ❌ 缺失（历史遗留，无法补全） |
| v330-5 | 所有 `GATE_REPORT` | ❌ 缺失（历史遗留） |
| v330-6 | `TEST_PLAN.md` | ❌ 缺失（历史遗留） |
| v330-7 | OO Design 文档 | 🟡 缺失（无设计文档） |

> v3.3.0 的文档缺失为历史遗留问题（v3.3.0 无任何门禁记录），无法补全。只能在 `GOVERNANCE_AUDIT.md` 中标注为 `❌ 历史遗留`。

### 8.2 v3.1.0（半途终止版本）

**问题**：Beta/RC/GA 从未执行，但 COMPREHENSIVE_STATUS_REPORT 自称 "Beta Gate ✅ PASS"

**检查要求**：
- 如果版本声称通过某阶段门禁，对应的 `*_GATE_REPORT.md` 必须存在
- 不能仅凭 `COMPREHENSIVE_STATUS_REPORT` 或 `STATUS_REPORT` 的声明就认定通过门禁

### 8.3 v2.8.0 ~ v2.9.0（文档最规范版本）

**检查要求**：这两个版本的 `DOCUMENT_AUDIT.md` 是最佳实践模板，后续版本应参考其结构。

---

## 九、缺失文档处理规范

| 状态 | 含义 | 在报告中的写法 |
|------|------|--------------|
| ❌ MISSING | 必须存在但缺失，阻塞发布 | `[ -f xxx.md ] || echo "❌ MISSING"` |
| ✅ EXISTS | 存在且验证通过 | `echo "✅ $doc_name"` |
| ⏭️ SKIP | 非强制/历史遗留 | `echo "⏭️ SKIP: $doc_name (历史遗留)"` |
| ⚠️ CONTRAVENTION | 文档存在但与其他文档矛盾 | 必须立即纠正 |

---

## 十、执行流程

### 10.1 发布前自检（开发者在本地执行）

```bash
VERSION=v3.5.0
cd docs/releases/$VERSION

# 1. 检查必须文档存在
for f in README.md CHANGELOG.md DEV_PLAN.md RELEASE_NOTES.md; do
  [ -f "$f" ] || echo "❌ MISSING: $f"
done

# 2. PENDING 扫描
pending=$(grep -c "PENDING\|⏳\|进行中\|待实测\|TBD" *_GATE*.md 2>/dev/null || echo 0)
[ "$pending" -gt 0 ] && echo "❌ Truthfulness FAIL: $pending PENDING found" || echo "✅ No PENDING"

# 3. 链接检查
bash ../../scripts/gate/check_docs_links.sh entry

# 4. 版本状态检查
grep "当前版本" ../../docs/releases/VERSION_HISTORY.md

# 5. Alpha 一致性（如果同时存在 GOVERNANCE_AUDIT 和 ALPHA_GATE_REPORT）
if [ -f GOVERNANCE_AUDIT.md ] && [ -f ALPHA_GATE_REPORT.md ]; then
  gov_alpha=$(grep -E "[0-9]+/[0-9]+.*PASS" GOVERNANCE_AUDIT.md | grep -i alpha | head -1)
  rep_alpha=$(grep -E "[0-9]+/[0-9]+" ALPHA_GATE_REPORT.md | grep -v "grep" | head -1)
  echo "Alpha: GOVERNANCE_AUDIT='$gov_alpha' vs ALPHA_GATE_REPORT='$rep_alpha'"
fi
```

### 10.2 GA CI/CD 门禁（自动执行）

GA gate 脚本必须包含以下检查：

```bash
# D1-D10 必须文档存在性检查
for doc in README.md CHANGELOG.md DEV_PLAN.md RELEASE_NOTES.md GA_GATE_REPORT.md; do
  [ -f "docs/releases/v${VERSION}/$doc" ] || { echo "❌ $doc MISSING"; DOC_FAIL=1; }
done

# T1-T5 Truthfulness 检查
pending=$(grep -c "PENDING\|⏳\|进行中\|待实测\|TBD" "docs/releases/v${VERSION}"/*GATE*.md 2>/dev/null || echo 0)
[ "$pending" -gt 0 ] && { echo "❌ Truthfulness FAIL: PENDING found"; exit 1; }

# L1 链接检查
bash scripts/gate/check_docs_links.sh entry

# V1-V4 版本状态
grep "当前版本" docs/releases/VERSION_HISTORY.md | grep -q "v${VERSION}" || echo "⚠️ VERSION_HISTORY not updated"
```

---

## 十一、常见错误及修复

| 错误类型 | 发现方法 | 修复方案 |
|---------|---------|---------|
| 相对路径写错（`../oo/` 写成 `oo/`） | `check_docs_links.sh` | 修正为正确相对路径 |
| 链接指向不存在的文件 | `check_docs_links.sh` | 删除链接或创建目标文件 |
| PENDING 占位符未清除 | `grep "PENDING" *_GATE*.md` | 替换为 ✅/❌/⏭️ |
| 版本号写错 | 手动核对 VERSION 文件 | 修正版本号 |
| 覆盖率写 PENDING 而非实测值 | 手动检查 `cargo llvm-cov` 输出 | 填入实测值 |
| GOVERNANCE_AUDIT 与 ALPHA_GATE_REPORT 数字矛盾 | 手动对比两个文件 | 以实测报告为准，纠正 GOVERNANCE_AUDIT |
| GA 报告标题与结论矛盾 | `grep "^#.*GA" GA_GATE_REPORT.md` vs 结论节 | 以结论节为准，纠正标题 |
| DEV_PLAN.md 与 DEVELOPMENT_PLAN.md 混用 | 目录中存在两个文件 | 删除旧名，保留新名（DEV_PLAN.md） |

---

## 十二、相关文档

- [GOVERNANCE_INDEX.md](../governance/GOVERNANCE_INDEX.md) — 文档导航索引
- [GATE_REPORT_TRUTH.md](../governance/GATE_REPORT_TRUTH.md) — 门禁报告真实性规范
- [DOCUMENT_COMPLETENESS_CHECK.md](../governance/DOCUMENT_COMPLETENESS_CHECK.md) — 文档完整性检查规范
- [DOCUMENT_COMPLIANCE_AUDIT.md](../releases/v3.4.0/DOCUMENT_COMPLIANCE_AUDIT.md) — v2.8.0~v3.4.0 综合审计报告
- [GOVERNANCE_AUDIT.md](../releases/v3.4.0/GOVERNANCE_AUDIT.md) — v3.4.0 治理审计
- [check_docs_links.sh](../../scripts/gate/check_docs_links.sh) — 链接检查脚本
- [gate_spec_v350.md](../governance/gate_spec_v350.md) — v3.5.0 门禁规范

---

*本清单为文档治理的最低标准，随着项目发展将持续补充完善*
*最后更新: 2026-05-29*
*编制依据: GOVERNANCE_INDEX + DOCUMENT_COMPLETENESS_CHECK + DOCUMENT_COMPLIANCE_AUDIT (v3.4.0) + 各版本 GOVERNANCE_AUDIT + 1207 链接扫描结果*