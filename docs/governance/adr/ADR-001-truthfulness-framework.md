# ADR-001: Truthfulness Framework

## Status

**Accepted** — v3.7.0 GA (2026-05-30)

## Context

在 v3.7.0 GA 建设过程中，发现多个 Governance Failure Mode：

1. **Gate False Positive**: v3.6.0 Beta Gate 声称 7/9 PASS，实际 0/8 通过
2. **Stale Document**: RC_TO_GA_FINAL_REPORT.md 显示 coverage 68.8%，但代码已达 85.81%
3. **Coverage Dispute**: 用户用 `--lib only` 测得 84.98%，指控 GA_GATE_REPORT 造假
4. **Unverified Claims**: 文档声明未经实测验证

这些问题的根因是：治理体系缺乏"Truthfulness"约束——只检查"规则是否满足"，不检查"文档是否与实际一致"。

## Decision

建立 **Truthfulness Framework**，作为 Governance 的基础原则：

### G-01: Claim ≠ Evidence

任何 Claim 必须有对应的 Evidence。Evidence 必须是：
- **命令输出**: cargo test/output 等可执行验证的输出
- **文档引用**: 引用 SSOT（Single Source of Truth）中的具体定义
- **实测数据**: 实际运行的测试结果，非模拟数据

禁止：
- ❌ Claim 无 Evidence 支持
- ❌ Evidence 来自不可验证的来源
- ❌ 将"文档引用"当作"实测证据"

### G-02: Document Claim 必须标记来源

所有文档中的 Claim 必须标记来源类型：

```
Claim: "Parser coverage 47%"
Source Type: [实测|SSOT引用|历史文档]
Source: ALPHA_GATE_REPORT.md §A5 / RC_GATE_REPORT.md commit aa830bcd
```

### G-03: Gate 报告必须包含实际命令

每份 Gate 报告必须包含：
1. **实际执行的命令**（非标准命令描述）
2. **命令输出**（实际 stdout/stderr）
3. **结论**（基于输出的判断）

### G-04: Coverage 测量方法必须一致

所有 Gate 的覆盖率测量必须使用统一方法：

```bash
# L1 Crates 综合测量方法
for crate in "${L1_CRATES[@]}"; do
    if cargo llvm-cov test --package "$crate" --all-features --tests 2>/dev/null | grep "^TOTAL" | grep -q "%"; then
        # --tests 优先
    else
        # --lib fallback
    fi
done
```

禁止在不同 Gate 报告中混用 `--lib only` 和 `--tests`。

### G-05: Document State ≠ Execution State

文档状态（PASS/FAIL）和实际执行状态可能不一致。Gate 报告必须明确区分：
- **Configured**: Gate 脚本和配置存在
- **Executed**: 实际运行了 Gate 命令
- **Verified**: 验证了 Gate 结果与文档一致

### G-06: 文档 Claim 必须有 Freshness 标记

所有引用历史数据的 Claim 必须标注数据年龄：

```
Parser coverage: 47.16%
Freshness: v3.5.0 Alpha Gate Report (2026-05-28)
Current status: Unknown (未重新测量)
```

### G-07: STRICT PROOF MODE

当用户要求"STRICT PROOF MODE"时：
- **No Evidence = FAIL**: 没有证据支撑的 Claim 直接判定为 FAIL
- **实测优先**: 先实测再下结论
- **质疑文档**: 文档数据需要重新验证

## Consequences

### Positive

- Governance 文档的可信度提升
- Gate 报告与实际执行结果一致
- 防止 Stale Document 导致的错误判断
- 支持跨版本追溯

### Negative

- Gate 报告编写工作量增加（需要包含实际命令输出）
- Agent 需要花更多时间收集 Evidence
- 部分历史文档可能需要更新以满足 Freshness 标记要求

### Neutral

- Truthfulness Framework 不改变 Gate 阈值，只改变 Claim 的验证方式
- 旧文档无需强制更新，但新 Claim 必须遵守

---

## Metadata

- **Author**: Hermes Agent
- **Date**: 2026-05-30
- **Related ADRs**: ADR-002 (Claim Registry), ADR-003 (Decision Registry)
- **Related PRs**: v3.7.0 GA (dd1cfdbd)
- **Supersedes**: N/A (new ADR)