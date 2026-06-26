# Stage 3.1 — 修 P12/P13/P16 gate 自身 bug Diff Plan (B 批)

> **作者**: Hermes Agent (hermes-z6g4 @ Z6G4 192.168.0.252)
> **日期**: 2026-06-26
> **依据**: 李哥批 B = 先修 P12/P15/P16 gate 自身 (让 4 FAIL → 0 FAIL)
> **基线 SHA**: 3d817386d (origin/develop/v3.9.0)
> **Worktree**: .worktrees/doc-audit-2026-06-26
> **目的**: 列精确 diff, 等单字母批准后执行
> **状态**: 待批

---

## 0. 4 个 FAIL gate 根因 (实查, P0 真状态)

| Gate | 状态 | 根因 | 范围 |
|------|------|------|------|
| P12 | FAIL | 1 个 `#[ignore]` (long_run_stability_72h_test.rs) 不在 registry | ✅ 本批修 |
| P13 | WARN | baseline JSON 多行格式, grep 抓 2 个数字 → `[: 118 115: integer expression expected` | ✅ 本批修 (gate 自身) |
| P14 | FAIL | 5 anti-patterns (已 Stage 3 修过 4 target file, 剩 28 个 baseline 外) | ❌ 超出本批 |
| P15 | FAIL | 8 个 gate 缺 oracle 引用 + 1 gate lost oracle | ❌ 超出本批 (gate 内容) |
| P16 | FAIL | P16 python 用错 GATE_DIR fallback `/home/ai/sqlrustgo/scripts/gate`, **脚本没 export GATE_DIR** | ✅ 本批修 |

**本批目标: 修 P12 + P13 + P16, 让 4 FAIL → 1 FAIL (剩 P15)**。

---

## 1. P12 修法 (1 file 改)

### 1.1 现状 (实查)

`tests/long_run_stability_72h_test.rs` line 6 有 `#[ignore]` (72h long-run 测试, 当前跑 5s smoke), **没在 registry**。

### 1.2 diff (改 `tests/baseline/ignore_registry.json`)

在 `ignored_tests` 数组最后追加 1 条:

```json
    {
      "file": "tests/long_run_stability_72h_test.rs",
      "line": "5:#[ignore]",
      "reason": "72h long-run stability test (lineitem SELECT COUNT loop 72h). Currently runs as 5s smoke; full 72h run is gated by SOAK-3265 in .worktrees/soak-3265. P12 marker: 1 #[ignore] attribute.",
      "issue_link": "#3265",
      "added_in": "v3.9.0-rc7",
      "category": "long_run_stability"
    }
```

### 1.3 验证 (改后)

跑 P12, 期望:
- ❌ 1 file unregistered → 0
- Source count 62, Registry count 74 (从 73 → 74)

---

## 2. P13 修法 (1 file 改)

### 2.1 现状 (实查)

`tests/baseline/test_count.json` 同时存 current (118/6555/55) 和 previous (115/6210/31)。
P13 脚本 line 78:
```bash
baseline_cargo=$(grep -oE '"cargo_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' || echo 0)
```
`grep -oE` 抓**所有**匹配 = 2 个数字 ("118" + "115"), `baseline_cargo="118 115"`。然后 `[: 118 115: integer expression expected` = 比较时多值。

### 2.2 diff (改 `scripts/gate/check_test_count_monotonic.sh` line 78-80)

Before:
```bash
baseline_cargo=$(grep -oE '"cargo_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' || echo 0)
baseline_active=$(grep -oE '"active_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' || echo 0)
baseline_ignored=$(grep -oE '"ignored_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' || echo 0)
```

After (加 `| head -1` 取第 1 个, 即 current version):
```bash
# FIX 2026-06-26 Hermes / P13 baseline parsing: take FIRST match (current version)
# baseline JSON has both current + previous counts, need to grab current only
baseline_cargo=$(grep -oE '"cargo_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' | head -1 || echo 0)
baseline_active=$(grep -oE '"active_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' | head -1 || echo 0)
baseline_ignored=$(grep -oE '"ignored_tests": [0-9]+' "$BASELINE" | grep -oE '[0-9]+' | head -1 || echo 0)
```

### 2.3 验证 (改后)

跑 P13, 期望:
- 不再 `[: integer expression expected` 报错
- baseline 解析 = 118/6555/55 (current), 比较 current (115/6411/62) vs baseline (118/6555/55) = cargo -3 FAIL, active -144 FAIL, ignored +7 PASS
- P13 gate 自身从 WARN (语法错) → 真正状态 (cargo/active 减少 = FAIL, ignored 增加 = PASS)

注: P13 会**变成 FAIL** 因为 test count 减少 (115 < 118)。这是**真实状态**, 不是 bug。
- 跟 Stage 2 目标"GA target TBD"一致: 24h/72h/168h real soak 0 完成 → 一些 test 被 disable/disable
- 改 governance 文档解释原因 + 走 ADR 流程才能让 P13 PASS

**本批只修 gate 自身语法 bug**, 不解决 test count 减少的根本问题。

---

## 3. P16 修法 (1 file 改)

### 3.1 现状 (实查)

P16 脚本 line 108:
```python
GATE_DIR = os.environ.get("GATE_DIR", "/home/ai/sqlrustgo/scripts/gate")
if not os.path.isdir(GATE_DIR):
    sys.exit(0)
```

`GATE_DIR` env var 没在 bash 里 export, python fallback 到 hardcoded `/home/ai/sqlrustgo/scripts/gate` (不存在的路径) → `sys.exit(0)` → 0 tests 输出 → step 1 FAIL。

### 3.2 diff (改 `scripts/gate/check_gate_test_integrity.sh` line 107-108 之前)

在 `GATE_TESTS_RAW=$(python3 - <<'PYEOF'` 之前加 1 行:

Before (line 106-108):
```bash
GATE_TESTS_RAW=$(python3 - <<'PYEOF'
import os
import re
import sys

GATE_DIR = os.environ.get("GATE_DIR", "/home/ai/sqlrustgo/scripts/gate")
```

After:
```bash
# FIX 2026-06-26 Hermes / P16 GATE_DIR fallback: export GATE_DIR so python heredoc picks it up
export GATE_DIR="${GATE_SCRIPTS_DIR}"
GATE_TESTS_RAW=$(python3 - <<'PYEOF'
import os
import re
import sys

GATE_DIR = os.environ.get("GATE_DIR", "/home/ai/sqlrustgo/scripts/gate")
```

### 3.3 验证 (改后)

跑 P16, 期望:
- Step 1: "found N gate-referenced tests" PASS (N 应为 25-29 个)
- Step 2: 检查每个 test 的 `#[ignore]` 状态 — **可能**会发现 0 个或几个 violation
- Step 3: 最终 P16 状态取决于 Step 2 发现

**预期**: P16 Step 1 PASS, Step 2 可能 PASS (0 violations) 或 FAIL (1-2 个真 #[ignore] on gate test)

---

## 4. 不修改 (本批范围外)

- ❌ P14 (5 anti-patterns 已在 Stage 3 修 4 个 target file, 剩 baseline 外 28 个 — 本批不动)
- ❌ P15 (8 个 gate 缺 oracle + 1 gate lost oracle — gate 内容问题, 需 ADR 流程)
- ❌ 任何 cargo test / clippy (抢 CPU)
- ❌ SOAK (撞端口)
- ❌ 推 252 Gitea
- ❌ 改 5 governance 文档
- ❌ 改顶层 5 文档
- ❌ 新建 ADR-014

---

## 5. 修改顺序

1. **改 `tests/baseline/ignore_registry.json`** (P12, +1 entry)
2. **跑 P12 验证** (期望 1 file unregistered → 0)
3. **改 `scripts/gate/check_test_count_monotonic.sh`** (P13, +3 `| head -1`)
4. **跑 P13 验证** (期望不再 `[: integer expression expected`)
5. **改 `scripts/gate/check_gate_test_integrity.sh`** (P16, +1 `export GATE_DIR=...`)
6. **跑 P16 验证** (期望 step 1 PASS, 找到 N gate-referenced tests)
7. **跑全套 6 meta-gate** 验证 4 FAIL → 1 FAIL (剩 P15)
8. **写报告** `reports/STAGE3_1_GATE_FIXES_2026-06-26.md` (含 before/after 对照 + 验证)

每步之间:
```bash
git diff --stat <file>
bash -n <file>  # 语法检查
```

---

## 6. 风险/边界

### 6.1 已知风险

- **P13 改后变 FAIL**: test count 减少 (115 < 118), 真实状态**不是**语法错。这是**应该**的状态, 但要让用户 (李哥) 知道: 改 P13 后 P13 从 WARN → FAIL, 但**真信号** (test count 减少) 现在可见
- **P16 改后可能仍 FAIL**: Step 1 PASS, Step 2 可能发现 0-N 个 `#[ignore]` on gate test。如果 P16 真有 0 violation → P16 改 PASS

### 6.2 已知限制

- 不解决 P15 (8 gate 缺 oracle) = **永远**剩 1 FAIL
- 不解决 test count 减少 = P13 改后 FAIL (但**真信号**)
- 本批只修**3 个** gate 自身 bug

### 6.3 rollback

```bash
git checkout -- tests/baseline/ignore_registry.json
git checkout -- scripts/gate/check_test_count_monotonic.sh
git checkout -- scripts/gate/check_gate_test_integrity.sh
```

---

## 7. 总变更预估

| 文件 | 改前 | 改后 | 变化 |
|------|------|------|------|
| `tests/baseline/ignore_registry.json` | 73 entries | 74 entries | +1 entry (~10 lines) |
| `scripts/gate/check_test_count_monotonic.sh` | 130+ lines | +3 字符 | +0 lines (3 `| head -1`) |
| `scripts/gate/check_gate_test_integrity.sh` | 280+ lines | +1 行 | +1 line |
| **新** `reports/STAGE3_1_GATE_FIXES_2026-06-26.md` | 0 | ~150 行 | +150 |

**总: 3 文件改 + 1 报告, 0 个 governance 文档, 0 个代码文件**

---

## 8. 等单字母批准

- **A** = 全做 (P12 + P13 + P16 + 报告)
- **B** = 只做 P12 + P16 (P13 改后变 FAIL 风险, 跳)
- **C** = 只写报告, 不改文件
- **D** = 别的指令

**默认我等单字母**。请回 A/B/C/D。
