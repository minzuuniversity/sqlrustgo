# P0 整改 Diff Plan — governance 落实 (Stage 3)

> **作者**: Hermes Agent (hermes-z6g4 @ Z6G4 192.168.0.252)
> **日期**: 2026-06-26
> **依据**: 上一轮"为什么 governance 落实不了"分析 + 李哥批 A = P0 全做 (1+2+3)
> **基线 SHA**: 3d817386d (origin/develop/v3.9.0)
> **Worktree**: .worktrees/doc-audit-2026-06-26
> **目的**: 列精确 diff, 等单字母批准后执行
> **状态**: 待批

---

## 0. P0 原则 (5 个不变量, 跟 Stage 2 一致)

1. **最小修改**: 只改"事实性错误" (gate 自身 PIPESTATUS bug + CI hard-block + AGENTS.md 强制读)
2. **证据优先**: 任何"PASS" 必须实跑验证 (改完跑同一 gate 看到 exit code 变化)
3. **可撤销**: 每个修改可 `git checkout -- <file>` 恢复, 每步有 git diff
4. **不引 doc claim 当 PASS**: 整改报告引用的"PASS" 必须来自 P11-P16 实跑
5. **不抢资源**: 不跑 `cargo test --all-features` (会抢 Claude Code / opencode CPU), 不跑 SOAK (撞 13306)

---

## 1. 改 5 个 gate 脚本 (P0 步骤 1: 修 PIPESTATUS bug)

### 1.1 共同修法 (5 处统一)

**Pattern** (当前坏):
```bash
PASSED=$(cargo test --test <test> 2>&1 | grep -E "test result.*ok" | head -1)
if echo "$PASSED" | grep -q "ok"; then
    echo "  PASS"
fi
```

**修后** (统一模板):
```bash
# Save cargo test output to variable, then capture exit code
CARGO_OUTPUT=$(cargo test --test <test> 2>&1)
CARGO_EXIT=$?
if [ $CARGO_EXIT -ne 0 ]; then
    echo "  ❌ FAIL: cargo test exit $CARGO_EXIT (PIPESTATUS check)"
    echo "$CARGO_OUTPUT" | tail -10
    exit 1
fi
# Now check PASSED only if exit 0
PASSED=$(echo "$CARGO_OUTPUT" | grep -E "test result.*ok" | head -1)
if [ -z "$PASSED" ]; then
    echo "  ❌ FAIL: cargo test exit 0 but no 'test result: ok' line"
    echo "$CARGO_OUTPUT" | tail -10
    exit 1
fi
echo "  PASS: $PASSED"
```

### 1.2 5 处具体 diff

**A. `scripts/gate/check_p12_crash_test.sh` line 89** (从单行 grep 改 5 行)

Before (line 89):
```bash
PASSED=$(cargo test --test crash_test_framework 2>&1 | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1)
```

After (替换 89-94):
```bash
# FIX 2026-06-26 (Hermes / P14 DRIFT): capture exit code before grep
CARGO_OUTPUT=$(cargo test --test crash_test_framework 2>&1)
CARGO_EXIT=$?
if [ $CARGO_EXIT -ne 0 ]; then
    echo "  ❌ FAIL: cargo test exit $CARGO_EXIT (drift check, P14)"
    echo "$CARGO_OUTPUT" | tail -10
    exit 1
fi
PASSED=$(echo "$CARGO_OUTPUT" | grep -E "test result.*ok" | grep -oE "[0-9]+ passed" | head -1)
```

**B. `scripts/gate/check_g14_real_crash.sh` line 74** (同样修法)

Before (line 74-75):
```bash
TPCH_PASSED=$(cargo test --test tpch_gate_test 2>&1 | grep -E "test result.*ok" | head -1 || true)
if echo "$TPCH_PASSED" | grep -q "ok"; then
```

After:
```bash
# FIX 2026-06-26 (Hermes / P14 DRIFT)
CARGO_OUTPUT=$(cargo test --test tpch_gate_test 2>&1)
CARGO_EXIT=$?
if [ $CARGO_EXIT -ne 0 ]; then
    echo "  ⚠️ WARN: TPC-H cargo test exit $CARGO_EXIT (drift check, P14)"
    CARGO_OUTPUT=""  # fall through to existing warn branch
else
    TPCH_PASSED=$(echo "$CARGO_OUTPUT" | grep -E "test result.*ok" | head -1)
fi
if [ -n "$TPCH_PASSED" ] && echo "$TPCH_PASSED" | grep -q "ok"; then
```

注: G14 这处原本就是 WARN-only, 不强 fail, 改完仍走 WARN 路径

**C. `scripts/gate/check_g13_stability.sh` line 81** (同 B, WARN-only)

**D. `scripts/gate/check_g16_compatibility.sh` line 66** (这是 hard-fail, 改法同 A)

Before (line 66-78):
```bash
MAIN_RESULT=$(cargo test --test v380_to_v390_full_upgrade_test 2>&1 | grep -E "test result.*ok" | head -1 || true)
if echo "$MAIN_RESULT" | grep -q "ok"; then
    N_PASSED=$(echo "$MAIN_RESULT" | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+")
    if [ "$N_PASSED" -lt 18 ]; then
        echo "  ❌ FAIL: expected ≥18 tests in v380_to_v390_full_upgrade_test, got $N_PASSED"
        exit 1
    fi
    echo "  [4/7] ✅ PASS: $N_PASSED tests pass (≥18)"
else
    echo "  ❌ FAIL: v380_to_v390_full_upgrade_test did not pass"
    cargo test --test v380_to_v390_full_upgrade_test 2>&1 | tail -5
    exit 1
fi
```

After:
```bash
# FIX 2026-06-26 (Hermes / P14 DRIFT)
CARGO_OUTPUT=$(cargo test --test v380_to_v390_full_upgrade_test 2>&1)
CARGO_EXIT=$?
if [ $CARGO_EXIT -ne 0 ]; then
    echo "  ❌ FAIL: v380_to_v390_full_upgrade_test exit $CARGO_EXIT (drift check, P14)"
    echo "$CARGO_OUTPUT" | tail -5
    exit 1
fi
MAIN_RESULT=$(echo "$CARGO_OUTPUT" | grep -E "test result.*ok" | head -1)
if [ -z "$MAIN_RESULT" ]; then
    echo "  ❌ FAIL: v380_to_v390_full_upgrade_test exit 0 but no 'test result: ok'"
    echo "$CARGO_OUTPUT" | tail -5
    exit 1
fi
if echo "$MAIN_RESULT" | grep -q "ok"; then
    N_PASSED=$(echo "$MAIN_RESULT" | grep -oE "[0-9]+ passed" | grep -oE "[0-9]+")
    if [ "$N_PASSED" -lt 18 ]; then
        echo "  ❌ FAIL: expected ≥18 tests in v380_to_v390_full_upgrade_test, got $N_PASSED"
        exit 1
    fi
    echo "  [4/7] ✅ PASS: $N_PASSED tests pass (≥18)"
fi
```

**E. `scripts/gate/check_g16_compatibility.sh` line 81** (同 D, harness test)

改后预期: 5 个文件各 +5~10 行, 行为: cargo test 真 fail 时 gate 必 exit 1, 不再因 grep 静默"未匹配" 算 PASS

**改后验证**: 跑 `bash scripts/gate/check_drift_not_pass.sh`, 期望 5 anti-patterns 变 0 anti-patterns

---

## 2. 改 `.gitea/workflows/ci.yml` (P0 步骤 2: governance 阻断)

### 2.1 当前坏 (实查)

ci.yml 8 个 governance gate 步骤:
```yaml
- name: Gate ...
  run: bash scripts/gate/check_xxx.sh 2>&1 | tee xxx.log
```
**没有 `if: failure()` / `exit-on-fail`** = gate 失败不阻断

### 2.2 修法: 加 governance 阻断 step

在 ci.yml `postcheck` 末尾加一个新 step (在 final summary 之前), 用 `set -e + 严格 exit 检查`:

```yaml
      - name: Governance gates — hard block (P0 2026-06-26)
        run: |
          set -euo pipefail
          FAILED=0
          for g in check_anti_fabrication check_docs_consistency \
                   check_document_completeness check_g1_tpch_baseline; do
              echo "=== $g ==="
              if bash scripts/gate/$g.sh 2>&1 | tail -20; then
                  echo "  ✅ PASS: $g"
              else
                  echo "  ❌ FAIL: $g (governance hard block)"
                  FAILED=$((FAILED+1))
              fi
          done
          if [ $FAILED -gt 0 ]; then
              echo ""
              echo "=== GOVERNANCE HARD BLOCK: $FAILED gate(s) FAILED ==="
              echo "=== PR 不能 merge until 修复 ==="
              exit 1
          fi
```

### 2.3 关键设计点

- **位置**: `postcheck` job 末尾, `needs: [test]`
- **gate 列表**: 4 个 (不是全 8, 因有 4 个跟 SOAK / 24h 相关, 不在 daily PR 阻断范围)
- **fail = exit 1** = 阻断 PR 合并 (因 Gitea actions 会将 job failure → PR red)
- **验证**: 改后跑 `bash scripts/gate/check_anti_fabrication.sh` 验证不破坏现有行为

### 2.4 注: Gitea Actions 跟 GitHub Actions 差异

- Gitea 用 `actions/checkout@v4` ✓ (已配)
- `if: always()` ✓ (已用)
- `set -euo pipefail` + `exit 1` 在 Gitea runner 上**应该** work (hermes-ops §5.2 用了相同 pattern)
- 但实际验证需要改后跑一次 Gitea Actions dry-run, 这次只改 yaml, 不 push, 留给 ci.yml 自身的 dry-run 测

---

## 3. 改 `AGENTS.md` (P0 步骤 3: 强制读 governance)

### 3.1 当前 (实查)

`AGENTS.md` 现引用 3 个 governance 文档:
```
- `docs/governance/AI_COLLABORATION.md` - AI 协作规范
- `docs/governance/RELEASE_LIFECYCLE.md` - Release生命周期
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md` - **Issue 关闭验证流程 (强制执行)**
```

**不引用** (P0 关键):
- ANTI_FABRICATION_POLICY.md
- ADR-001 truthfulness-framework
- DOC_CHECK_CORRECTION_RULES.md
- GATE_CONDITIONS.md

### 3.2 改法: 在 AGENTS.md 加新 §"强制 governance 阅读清单"

在 AGENTS.md 的 "## Essential Commands" 之前 (line 36 附近) 插入:

```markdown
## 强制 governance 阅读清单 (P0 2026-06-26 新增)

**任何 AI agent 必须在开始 task 前读完以下 7 份**, 缺一不可:

1. `docs/governance/adr/ADR-001-truthfulness-framework.md` — G-01 ~ G-10 (claim ≠ evidence, 不引 doc claim 当 PASS)
2. `docs/governance/ANTI_FABRICATION_POLICY.md` — Type A/B/C/D 4 类违规 + Hard Gate vs Soft Gate
3. `docs/governance/ISSUE_CLOSING_VERIFICATION.md` — 关闭 Issue 前 5 步 (含 HTTP 405 workaround)
4. `docs/governance/DOC_CHECK_CORRECTION_RULES.md` — 改文档 7 步流程 (含实跑 gate 验证)
5. `docs/governance/AI_COLLABORATION.md` — 1.1 角色 + §5.5 多 AI 协调
6. `docs/governance/GATE_CONDITIONS.md` — G1-G16 门禁定义
7. `docs/governance/adr/ADR-008-test-claim-transparency.md` — P16 gate test integrity

**违反 P0 红线 (governance 落实不了 7 根因)**:
- ❌ 写 "5/5 PASS" 不引 P11-P16 实跑输出
- ❌ 写"完成"不引 git SHA + commit
- ❌ gate 失败不阻断 (CI 必须 exit 1)
- ❌ AI 协作 0 协调 (必须走 ADR-014 multi-ai-coordination)

**铁律**: 7 份读完才能写代码; 改任何 1 份必须实跑 gate 验证
```

### 3.3 改后预期

AGENTS.md 当前 12271 chars, 改后约 13000+ chars, +1 个新章节

---

## 4. 不变量 (跟 Stage 2 一致)

✅ **会做** (本批):
- 改 5 个 gate 脚本 (各 +5~10 行, 加 PIPESTATUS 检查)
- 改 `.gitea/workflows/ci.yml` (+1 governance hard-block step)
- 改 `AGENTS.md` (+1 强制阅读清单章节)
- 跑 P14 (check_drift_not_pass.sh) 验证 5 anti-patterns 变 0
- 跑 P11-P13, P15-P16 验证其他 gate 未回归
- 写整改报告 reports/STAGE3_GOVERNANCE_FIX_2026-06-26.md

❌ **不会做** (本批):
- 改 5 governance 文档 (Stage 2 等单字母)
- 改顶层 5 文档 (Stage 2 等单字母)
- 新建 ADR-014 (Stage 2 等单字母)
- 跑 cargo test / clippy (抢 CPU)
- 跑 SOAK (撞端口)
- 推 252 Gitea (改完后**只**在 doc-audit worktree 验证, push 等下一轮)
- 改任何 .rs / Cargo.toml / Cargo.lock
- 改 INDEX.md (Stage 2 待办)

---

## 5. 修改顺序

1. **改 5 个 gate 脚本** (P0 步骤 1)
2. **跑 P14** 验证 5 anti-patterns → 0
3. **跑 P11-P13, P15-P16** 验证未回归
4. **改 ci.yml** (P0 步骤 2)
5. **跑 ci.yml 语法检查** (`yamllint` 或 `python3 -c "import yaml; yaml.safe_load(...)"`)
6. **改 AGENTS.md** (P0 步骤 3)
7. **写报告** reports/STAGE3_GOVERNANCE_FIX_2026-06-26.md (含 before/after 对照 + 验证结果)
8. **git diff --stat 全部** 给你最后一遍审

每步之间:
```bash
git diff --stat <file>
```

---

## 6. 总变更预估

| 文件 | 改前 | 改后 | 变化 |
|------|------|------|------|
| `scripts/gate/check_p12_crash_test.sh` | 111 行 | ~118 行 | +7 |
| `scripts/gate/check_g14_real_crash.sh` | 118 行 | ~123 行 | +5 |
| `scripts/gate/check_g13_stability.sh` | 114 行 | ~119 行 | +5 |
| `scripts/gate/check_g16_compatibility.sh` | 112 行 | ~125 行 | +13 (2 处修) |
| `.gitea/workflows/ci.yml` | 155 行 | ~190 行 | +35 |
| `AGENTS.md` | 12271 chars | ~13000+ chars | +700+ chars |
| **新** `reports/STAGE3_GOVERNANCE_FIX_2026-06-26.md` | 0 | ~150 行 | +150 |

**总: 6 文件改 + 1 文件新建, 0 个 governance 文档, 0 个代码文件**

---

## 7. 风险/边界

- **改 gate 脚本风险**: 5 个 gate 当前可能假 PASS (grep 无匹配 → exit 0), 改后可能**真 FAIL** (cargo test 真的 exit ≠ 0)
  - 缓解: 不在 ci.yml 推 252, 只在 doc-audit worktree 验证
  - 验证: 改完 P14 跑出 0 anti-patterns ✓ (期望)
- **改 ci.yml 风险**: yaml 语法错 → Gitea CI 坏
  - 缓解: 改后跑 yaml 语法检查
- **改 AGENTS.md 风险**: 误改原内容
  - 缓解: 精确 patch, 改前 git diff 确认只 + 新章节

---

## 8. 等单字母批准

- **A** = 全做 (1+2+3) — 6 文件改 + 1 文件新建
- **B** = 只做 1 (P0.1 改 5 个 gate, 不动 ci.yml/AGENTS.md) — 5 文件改
- **C** = 只写报告, 不改文件 (跟上次"分析"配套)
- **D** = 别的指令

**默认我等单字母**。请回 A/B/C/D。
