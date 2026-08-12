<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# SQLRustGo v3.9.0 文档不自洽分析与整改建议

> **分析日期**: 2026-06-01
> **分析范围**: `docs/releases/v3.9.0/` 全部文档
> **分析者**: Hermes Agent
> **目的**: 找出文档间不自洽之处，提出整改建议

---

## 一、问题总览

| 类别 | 问题数 | 严重程度 |
|------|--------|----------|
| 时间线不一致 | 4 | 🔴 高 |
| 阶段状态不一致 | 3 | 🔴 高 |
| 门禁状态混淆 | 5 | 🔴 高 |
| 任务完成度矛盾 | 3 | 🟠 中 |
| 文档引用缺失 | 2 | 🟡 低 |
| **总计** | **17** | — |

---

## 二、时间线不一致问题

### 2.1 GA 目标日期矛盾

| 文档 | GA 目标日期 | 说明 |
|------|-------------|------|
| [ROADMAP.md](../../../ROADMAP.md) | 2026-09-23 | 12 周 |
| [V390_VERSION_PLAN.md](plans/V390_VERSION_PLAN.md) | 2026-09-23 | W12 收口 |
| [V390_COMPREHENSIVE_ASSESSMENT.md](V390_COMPREHENSIVE_ASSESSMENT.md) | 2026-09-23 **at risk** | 调整后 22-26 周 |
| [RC3_PLAN.md](rc/RC3_PLAN.md) | 2026-09-23 **at risk** | 依赖 Z6G4 |

**问题**: ROADMAP.md 和 V390_VERSION_PLAN.md 未反映 RC3_PLAN 揭示的延期风险。

**整改建议**:
- ROADMAP.md 应标注 "GA 目标: 2026-09-23 (at risk, 调整后 22-26 周)"
- V390_VERSION_PLAN.md 应同步更新时间线

---

### 2.2 分支创建日期矛盾

| 文档 | W0/分支创建日期 |
|------|-----------------|
| ROADMAP.md | 2026-06-05 |
| V390_VERSION_PLAN.md | W0 (2026-07-01) |
| CHANGELOG.md | 2026-06-05 |

**问题**: V390_VERSION_PLAN.md 的 W0 日期是 2026-07-01，但实际分支创建是 2026-06-05。

**整改建议**:
- V390_VERSION_PLAN.md 应修正 W0 = 2026-06-05

---

### 2.3 Phase 完成时间矛盾

| 文档 | Phase 完成状态 |
|------|----------------|
| ROADMAP.md | Phase 0 (启动) |
| V390_COMPREHENSIVE_ASSESSMENT.md | Phase 0-6 全部 form-only 完成 |
| CHANGELOG.md | 当前阶段 RC2 |

**问题**: ROADMAP.md 显示 Phase 0 (启动)，但实际已到 RC2 后期。

**整改建议**:
- ROADMAP.md 应更新状态为 "Phase 6 收口 (form-only) → RC3 待启动"

---

## 三、阶段状态不一致问题

### 3.1 版本状态矛盾

| 文档 | 当前阶段 |
|------|----------|
| CHANGELOG.md | RC2 |
| ROADMAP.md | Phase 0 (启动) |
| V390_VERSION_PLAN.md | Draft |
| V390_COMPREHENSIVE_ASSESSMENT.md | RC2 后期, 准备 RC3 |

**问题**: 四份文档对当前阶段描述不一致。

**整改建议**:
- 统一为 "RC2 后期 (form-only) → RC3 待启动"
- ROADMAP.md 应更新 Phase 状态

---

### 3.2 文档状态矛盾

| 文档 | 状态声明 |
|------|----------|
| ROADMAP.md | Phase 0 (启动) |
| V390_VERSION_PLAN.md | Draft (基于 ChatGPT 架构师 2026-06-05 战略建议) |
| V390_COMPREHENSIVE_ASSESSMENT.md | RC2 ✅ (form-only) / RC3 ⏳ (P0 cut 待启动) |

**问题**: ROADMAP.md 和 V390_VERSION_PLAN.md 未反映实际进度。

**整改建议**:
- ROADMAP.md 状态应更新为 "Phase 6 收口 (form-only)"
- V390_VERSION_PLAN.md 状态应更新为 "ACTIVE (RC2 后期)"

---

## 四、门禁状态混淆问题（最严重）

### 4.1 G1-G10 状态表述混淆

| 文档 | G1-G10 状态 |
|------|-------------|
| V390_COMPREHENSIVE_ASSESSMENT.md | ✅ 10/10 form-only PASS |
| 同文档后续 | ⚠️ G1 0/6 实际跑 TPC-H |
| 同文档后续 | ⚠️ G7 compressed-time, real pending |
| 同文档后续 | ⚠️ G8 mock only, real pending |
| RC3_PLAN.md | 真实生产级覆盖率 ~35% |

**问题**: 同一文档内先说 "PASS"，后说 "实际未跑"，容易误导读者。

**整改建议**:
- 门禁状态应明确区分 "form-only PASS" vs "REAL PASS"
- 建议使用统一格式：
  ```
  G1: ✅ form-only PASS | ⏳ REAL pending
  G7: ✅ compressed PASS | ⏳ 24h real pending
  ```

---

### 4.2 真实覆盖率表述不一致

| 文档 | 覆盖率表述 |
|------|-----------|
| V390_COMPREHENSIVE_ASSESSMENT.md | 真实生产级覆盖率 ~35% |
| 同文档表格 | 真实生产准备度 ⏳ 35% |
| 同文档评分 | 综合评分 7.0/10 (真实校准) |
| 同文档形式评分 | 综合评分 8.5/10 (form-only) |

**问题**: 多处表述容易混淆，读者可能误以为 8.5/10 是真实评分。

**整改建议**:
- 所有评分表格应明确标注 "(form-only)" 或 "(REAL)"
- 综合评分应优先展示真实评分，形式评分作为补充

---

### 4.3 G1 TPC-H 状态表述混淆

| 文档 | G1 状态表述 |
|------|-------------|
| V390_COMPREHENSIVE_ASSESSMENT.md §4.2 | ✅ PASS (6/6 form-only steps) |
| 同文档 §4.2 后续 | ⚠️ RC3_PLAN 关键发现: G1 TPC-H gate 0/6 steps 实际跑 TPC-H |
| 同文档 §16.1 | G1 形式 9.0/10 | G1 真实 9.0/10 (矛盾) |

**问题**: §16.1 评分表格中 G1 真实评分也是 9.0/10，与前面说的 "0/6 实际跑" 矛盾。

**整改建议**:
- §16.1 G1 眞实评分应改为 "6.0/10 (0/6 实际跑 TPC-H)"
- 或标注为 "⏳ pending REAL verification"

---

## 五、任务完成度矛盾问题

### 5.1 16/16 子任务完成表述

| 文档 | 任务完成表述 |
|------|-------------|
| V390_COMPREHENSIVE_ASSESSMENT.md | 16/16 子任务完成 |
| 同文档 | 77 #[ignore] tests 待 unignore |
| 同文档 | 43 TBD perf placeholders 待 fill |
| 同文档 | 0/22 wire TPC-H 待实现 |
| RC3_PLAN.md | 11 follow-up issues open |

**问题**: 说 "16/16 子任务完成"，但有大量待完成项。

**整改建议**:
- 应表述为 "16/16 Phase 任务完成 (form-only)"
- 明确标注 "13 critical-path items 待 RC3/RC4/GA"

---

### 5.2 P0 任务状态表述

| 文档 | P0 任务状态 |
|------|-------------|
| V390_COMPREHENSIVE_ASSESSMENT.md §3.2 | ✅ G4 form-only PASS |
| 同文档 §3.2 | ✅ G3 form-only PASS |
| 同文档 §3.2 | ✅ G2 form-only PASS |
| 同文档 §3.2 | ✅ G5 form-only PASS |
| 同文档 §10.2 | ⚠️ form-only 关闭 ≠ 真实生产级关闭 |

**问题**: P0 任务说 "完成"，但实际是 form-only。

**整改建议**:
- P0 任务状态应统一为 "✅ form-only PASS | ⏳ REAL pending RC3"

---

## 六、文档引用缺失问题

### 6.1 ROADMAP.md 引用不存在文档

ROADMAP.md §7 预期产物:
```
- GA 治理报告: `docs/governance/GA_GOVERNANCE_DEMO_v3.9.0.md`
```

**问题**: 该文档不存在。

**整改建议**:
- 修正为 `docs/releases/v3.9.0/ga/GA_GATE_REPORT.md`
- 或创建该文档

---

### 6.2 alpha/ 目录缺失

V390_COMPREHENSIVE_ASSESSMENT.md §11.1:
```
├── alpha/                             (待创建或 alpha1 报告已 merge)
```

**问题**: alpha/ 目录不存在，alpha1 报告位置未明确。

**整改建议**:
- 创建 `docs/releases/v3.9.0/alpha/ALPHA1_RELEASE_NOTES.md`
- 或明确标注 alpha1 报告已合并到其他文档

---

## 七、整改优先级建议

| 优先级 | 问题类别 | 整改文件 | 工作量 |
|--------|----------|----------|--------|
| **P0** | 门禁状态混淆 | V390_COMPREHENSIVE_ASSESSMENT.md | 2h |
| **P0** | 时间线不一致 | ROADMAP.md, V390_VERSION_PLAN.md | 1h |
| **P1** | 阶段状态不一致 | ROADMAP.md, V390_VERSION_PLAN.md | 1h |
| **P1** | 任务完成度矛盾 | V390_COMPREHENSIVE_ASSESSMENT.md | 1h |
| **P2** | 文档引用缺失 | ROADMAP.md | 0.5h |
| **P2** | alpha 目录缺失 | 创建 alpha/ALPHA1_RELEASE_NOTES.md | 1h |

---

## 八、整改建议汇总

### 8.1 ROADMAP.md 整改

1. 更新状态: "Phase 0 (启动)" → "Phase 6 收口 (form-only) → RC3 待启动"
2. 更新 GA 目标: "2026-09-23" → "2026-09-23 (at risk, 调整后 22-26 周)"
3. 修正文档引用: GA 治理报告路径

### 8.2 V390_VERSION_PLAN.md 整改

1. 更新状态: "Draft" → "ACTIVE (RC2 后期)"
2. 修正 W0 日期: "2026-07-01" → "2026-06-05"
3. 同步 GA 目标日期风险标注

### 8.3 V390_COMPREHENSIVE_ASSESSMENT.md 整改

1. 门禁状态统一格式: "✅ form-only PASS | ⏳ REAL pending"
2. §16.1 G1 眞实评分修正: "6.0/10 (0/6 实际跑)"
3. 任务完成表述: "16/16 Phase 任务完成 (form-only)"
4. 明确标注 "13 critical-path items 待 RC3/RC4/GA"

### 8.4 CHANGELOG.md 整改

1. 当前阶段表述: "RC2" → "RC2 (form-only) → RC3 待启动"
2. 添加 HONESTY NOTE 引用 RC3_PLAN

---

## 九、整改后预期效果

| 指标 | 整改前 | 整改后 |
|------|--------|--------|
| 时间线一致性 | ❌ 4 处矛盾 | ✅ 统一 |
| 阶段状态一致性 | ❌ 4 处矛盾 | ✅ 统一 |
| 门禁状态清晰度 | ❌ 混淆 | ✅ form/real 区分 |
| 任务完成度表述 | ❌ 模糊 | ✅ 明确 |
| 文档引用完整性 | ❌ 2 处缺失 | ✅ 完整 |

---

## 十、维护信息

| 项目 | 值 |
|------|-----|
| 报告版本 | v1.0 |
| 创建日期 | 2026-06-01 |
| 分析者 | Hermes Agent |
| 下次审查 | RC3 cut 时 |

---

**结论**: v3.9.0 文档体系存在 17 处不自洽问题，主要集中在时间线、阶段状态、门禁状态表述混淆。建议优先整改门禁状态表述（P0），确保 form-only vs REAL 区分清晰，避免误导读者认为 v3.9.0-rc2 是 production-ready。