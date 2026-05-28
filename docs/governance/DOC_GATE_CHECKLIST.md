# SQLRustGo 文档门禁检查清单 (DOC_GATE_CHECKLIST)

> **版本**: v1.0
> **创建日期**: 2026-05-29
> **规范来源**: `docs/governance/GOVERNANCE_INDEX.md` + `gate_spec_v350.md` §六
> **用途**: 所有 GA 版本发布前必须通过本文档检查，确保文档真实性和完整性

---

## 一、检查原则（Truthfulness）

1. **禁止 PENDING 占位符**：门禁报告中的所有状态必须基于实测结果，禁止使用 `⏳ 进行中`/`PENDING` 等未经验证的占位符
2. **禁止引用历史数据冒充本次执行**：每次发布必须基于当次实测数据
3. **禁止缺失文档冒充存在**：如果某个文档不存在，必须标记为 `⏭️ SKIP` 或 `❌ MISSING`，不能假装存在
4. **禁止格式错误冒充合规**：链接错误就是错误，不能标记为 PASS

---

## 二、必须文档清单（GA 发布前必须存在）

### 2.1 版本文档（每版本独立目录 `docs/releases/vX.Y.Z/`）

| # | 文档 | 验证方法 | 缺失处理 |
|---|------|---------|---------|
| D1 | `README.md` | `[ -f README.md ]` | ❌ 阻塞 GA |
| D2 | `CHANGELOG.md` | `[ -f CHANGELOG.md ]` | ❌ 阻塞 GA |
| D3 | `DEV_PLAN.md` | `[ -f DEV_PLAN.md ]` | ❌ 阻塞 GA |
| D4 | `TEST_PLAN.md` | `[ -f TEST_PLAN.md ]` | ⏭️ SKIP（非强制） |
| D5 | `ALPHA_GATE_REPORT.md` | `[ -f ALPHA_GATE_REPORT.md ]` | ❌ 阻塞 GA |
| D6 | `BETA_GATE_REPORT.md` | `[ -f BETA_GATE_REPORT.md ]` | ⏭️ SKIP（如无 Beta） |
| D7 | `RC_GATE_REPORT.md` | `[ -f RC_GATE_REPORT.md ]` | ⏭️ SKIP（如无 RC） |
| D8 | `GA_GATE_REPORT.md` | `[ -f GA_GATE_REPORT.md ]` | ❌ 阻塞 GA |
| D9 | `GATE_SUMMARY.md` | `[ -f GATE_SUMMARY.md ]` | ❌ 阻塞 GA |

### 2.2 SSOT 文档（治理规范，根目录 `docs/governance/`）

| # | 文档 | 验证方法 | 缺失处理 |
|---|------|---------|---------|
| D10 | `gate_spec_vXYY.md`（当前版本对应规范） | `[ -f gate_spec_v*.md ]` | ❌ 阻塞 GA |
| D11 | `GOVERNANCE_INDEX.md` | `[ -f GOVERNANCE_INDEX.md ]` | ❌ 阻塞 GA |
| D12 | `GATE_REPORT_TRUTH.md` | `[ -f GATE_REPORT_TRUTH.md ]` | ⏭️ SKIP（参考文档） |

### 2.3 版本索引文档（根目录 `docs/releases/`）

| # | 文档 | 验证方法 | 缺失处理 |
|---|------|---------|---------|
| D13 | `VERSION_HISTORY.md` | `[ -f VERSION_HISTORY.md ]` | ❌ 阻塞 GA |
| D14 | `VERSION_ROADMAP.md` | `[ -f VERSION_ROADMAP.md ]` | ⏭️ SKIP（参考文档） |
| D15 | `VERSION_TREND.md` | `[ -f VERSION_TREND.md ]` | ⏭️ SKIP（参考文档） |

---

## 三、Truthfulness 检查（门禁报告自检）

每个门禁报告在提交前必须通过以下检查：

| # | 检查项 | 命令 | FAIL 条件 |
|---|--------|------|----------|
| T1 | **无 PENDING 占位符** | `grep -c "PENDING\|⏳\|进行中" *.md` | count > 0 |
| T2 | **无"待实测"声明** | `grep -c "待实测\|待验证\|TBD" *.md` | count > 0 |
| T3 | **覆盖率数据为实测值** | `grep -E "[0-9]+\.[0-9]+%|覆盖率.*[0-9]" *.md | grep -v "^#"` | 包含 "PENDING" 或 "N/A" |
| T4 | **门禁结论与详情一致** | 摘要写 PASS 但详情写 PENDING → FAIL | 不一致 |

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
| L1 | **内部相对链接无死链** | `bash scripts/gate/check_docs_links.sh` | exit_code ≠ 0 |
| L2 | **关键文档引用存在** | 验证 `GATE_SUMMARY.md` 中引用的 `ALPHA/BETA/RC_GATE_REPORT.md` 存在 | 文件不存在 |
| L3 | **无悬空引用** | `grep -rn "(\.\./[^)]*\.md)" docs/releases/vX.Y.Z/` | 输出 > 0 |

---

## 五、版本状态准确性检查

| # | 检查项 | 验证方法 | FAIL 条件 |
|---|--------|---------|----------|
| V1 | **当前版本与实际一致** | `grep "当前版本" VERSION_HISTORY.md` → 版本号 ≤ 实际最高 GA 版本 | 版本号 < 实际 |
| V2 | **CHANGELOG.md 包含最新版本** | `head -20 CHANGELOG.md` 包含当前版本号 | 不包含 |
| V3 | **VERSION_ROADMAP.md 更新日期** | `更新日期` 字段 ≥ 版本 GA 日期 | 日期早于 GA |
| V4 | **README.md 版本状态** | `grep "当前版本\|v3\.X\.0" README.md` | 版本状态显示 "进行中" 但已 GA |

---

## 六、GA 报告内容完整性

每个 `GA_GATE_REPORT.md` 必须包含：

| # | 章节 | 要求 |
|---|------|------|
| G1 | **执行摘要** | 版本、日期、分支、SHA、整体结果 |
| G2 | **Gate 结果表格** | G1-G12 每项的状态、实测值、期望值 |
| G3 | **覆盖率数据** | 各 crate 的实测覆盖率（数字，非 PENDING） |
| G4 | **下一步** | 所有条目为 ✅ 或 ⏭️，无 ⏳ |
| G5 | **Truthfulness 声明** | "无 PENDING 占位，无引用历史数据冒充本次执行" |

---

## 七、执行流程

### 7.1 发布前自检（开发者在本地执行）

```bash
# 1. 检查必须文档存在
for f in README.md CHANGELOG.md DEV_PLAN.md ALPHA_GATE_REPORT.md GA_GATE_REPORT.md GATE_SUMMARY.md; do
  [ -f "$f" ] || echo "MISSING: $f"
done

# 2. PENDING 扫描
grep -rn "PENDING\|⏳\|进行中\|待实测\|TBD" *_GATE*.md && echo "FOUND PENDING!" || echo "CLEAN"

# 3. 链接检查
bash scripts/gate/check_docs_links.sh entry

# 4. 版本状态检查
grep "当前版本" docs/releases/VERSION_HISTORY.md
```

### 7.2 CI/CD 门禁（自动执行）

GA gate 脚本必须包含：

```bash
# D1-D9 必须文档存在性检查
check_ssot() {
  local name="$1"; local path="$2"
  [ -f "$path" ] || { echo "❌ $name MISSING"; DOC_FAIL=1; }
}
check_ssot "README.md" "docs/releases/v${VERSION}/README.md"
check_ssot "CHANGELOG.md" "docs/releases/v${VERSION}/CHANGELOG.md"
# ... 其他 D1-D9

# T1-T4 Truthfulness 检查
check_truthfulness() {
  local dir="$1"
  local pending=$(grep -c "PENDING\|⏳\|进行中" "$dir"/*GATE*.md 2>/dev/null || echo 0)
  [ "$pending" -gt 0 ] && { echo "❌ Truthfulness FAIL: PENDING found"; exit 1; }
}
```

---

## 八、缺失文档处理规范

| 状态 | 含义 | 在报告中的写法 |
|------|------|--------------|
| ❌ MISSING | 必须存在但缺失，阻塞发布 | `[ -f xxx.md ] || echo "❌ MISSING"` |
| ⏭️ SKIP | 规范中非强制，缺失可不处理 | `echo "⏭️ SKIP: $doc_name"` |
| ✅ EXISTS | 存在且验证通过 | `echo "✅ $doc_name"` |

---

## 九、常见错误及修复

| 错误类型 | 发现方法 | 修复方案 |
|---------|---------|---------|
| 相对路径写错（`../oo/` 写成 `oo/`） | `check_docs_links.sh` | 修正为正确相对路径 |
| 链接指向不存在的文件 | `check_docs_links.sh` | 删除链接或创建目标文件 |
| PENDING 占位符未清除 | `grep "PENDING" *_GATE*.md` | 替换为 ✅/❌/⏭️ |
| 版本号写错（如写成 v3.4.0 而非 v3.5.0） | 手动核对 VERSION 文件 | 修正版本号 |
| 覆盖率写 PENDING 而非实测值 | 手动检查 `cargo llvm-cov` 输出 | 填入实测值 |

---

## 十、相关文档

- [GOVERNANCE_INDEX.md](../governance/GOVERNANCE_INDEX.md) — 文档导航索引
- [GATE_REPORT_TRUTH.md](../governance/GATE_REPORT_TRUTH.md) — 门禁报告真实性规范
- [check_docs_links.sh](../../scripts/gate/check_docs_links.sh) — 链接检查脚本
- [gate_spec_v350.md](../governance/gate_spec_v350.md) — v3.5.0 门禁规范

---

*本清单为文档治理的最低标准，随着项目发展将持续补充完善*
*最后更新: 2026-05-29*