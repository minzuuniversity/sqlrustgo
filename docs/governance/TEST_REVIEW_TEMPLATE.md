# TEST REVIEW 模板 — SQLRustGo 测试审核规范

> **Version**: 1.0
> **Created**: 2026-06-02
> **Owner**: Hermes Agent
> **Status**: ACTIVE — 适用于 v3.8.0 及之后所有版本
> **强制执行**: 任何 PR 合并前必须完成本 REVIEW 流程

---

## 0. 测试审核与测试验收的区别

| 维度 | 测试设计 (TEST_DESIGN) | 测试审核 (TEST_REVIEW) | 测试验收 (ACCEPTANCE) |
|------|------------------------|------------------------|----------------------|
| **时间点** | 编码前 | 编码中/编码后 | 编码后 |
| **目的** | 设计"测什么、怎么测" | 验证"测试本身是否正确" | 验证"功能是否通过测试" |
| **关注点** | 覆盖度/边界/可执行 | 测试质量/独立性/可维护 | 功能完成度/门禁 |
| **执行人** | 测试设计者 | **独立审核者**（非原作者） | 门禁执行者 |
| **产物** | TEST_DESIGN.md | TEST_REVIEW.md | ACCEPTANCE.md |

**核心原则**：测试设计者 ≠ 测试审核者 ≠ 测试验收者，三权分立。

---

## 1. REVIEW 适用范围

每个 PR 必须产出：
1. **TEST_DESIGN.md**（编码前） — 由实现者编写
2. **TEST_REVIEW.md**（编码后） — 由独立审核者编写
3. **ACCEPTANCE.md**（门禁前） — 由门禁执行者编写

PR 编号 `PR-XXX` 对应文件命名：
- `PR-XXX_TEST_DESIGN.md`
- `PR-XXX_TEST_REVIEW.md`
- `PR-XXX_ACCEPTANCE.md`

---

## 2. TEST_REVIEW 必查项（8 大类 30 子项）

### 2.1 测试设计维度（Coverage Design）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.1.1 | 覆盖矩阵完整 | 每条 PR 需求 → ≥1 个测试用例 | 补测试 |
| 2.1.2 | 边界覆盖 | min/max/空/单元素/大集合/重复值 | 补测试 |
| 2.1.3 | 异常路径 | error 码、panic 安全、resource leak | 补测试 |
| 2.1.4 | 并发场景 | 多线程/竞态（若功能涉及） | 补测试或显式标 N/A |
| 2.1.5 | 性能/规模 | SF=1 至少 1 个 query 不退化 | 补或 DEFERRED |

### 2.2 测试独立性（Independence）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.2.1 | 无外部依赖 | 不依赖外部 DB/网络/文件系统（除临时） | 重写 |
| 2.2.2 | 可重复执行 | 同一测试连跑 10 次结果一致 | 修隔离逻辑 |
| 2.2.3 | 无顺序耦合 | 可单跑，可全跑，可乱序跑 | 修 setup/teardown |
| 2.2.4 | 资源清理 | 临时文件/连接/线程全部释放 | 加 Drop 或 finally |
| 2.2.5 | 无全局状态污染 | 修改全局 state 后必须恢复 | 加 save/restore |

### 2.3 断言质量（Assertion Quality）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.3.1 | 断言具体 | `assert_eq!(x, 42)` 而非 `assert!(x.is_ok())` | 重写 |
| 2.3.2 | 错误信息有用 | `assert!(x, "expected y, got {}", actual)` | 加 msg |
| 2.3.3 | 不依赖 panic 顺序 | 不依赖"先 panic 再 assert"路径 | 重排 |
| 2.3.4 | 不依赖 print 输出 | 关键值必须断言，不能靠 println 看 | 改 assert |
| 2.3.5 | 数值精度显式 | 浮点比较用 epsilon 或精确有理数 | 改方法 |

### 2.4 真实可执行（Real Executability）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.4.1 | 命令可跑 | `cargo test -p <pkg> --test <name>` 100% PASS | 修代码 |
| 2.4.2 | 无 #[ignore] 隐藏 | 所有 #[ignore] 必须有理由 + tracking issue | 移除或补理由 |
| 2.4.3 | 无 TODO 占位 | 不允许 "TODO: add test" 类注释残留 | 补全或删 |
| 2.4.4 | 无 commented-out 测试 | 不允许 `// #[test]` 注释测试 | 删除或激活 |
| 2.4.5 | 跑通时间合理 | 单 crate 测试 ≤5 分钟 | 优化或拆分 |

### 2.5 性能与稳定性（Performance & Stability）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.5.1 | 内存安全 | 8GB 限制内（CI 配置） | 减数据集 |
| 2.5.2 | 无 O(n²) 隐式 | 大输入下不退化为 O(n²) | 优化 |
| 2.5.3 | 无锁泄漏 | 多线程测试无死锁 | 修锁顺序 |
| 2.5.4 | 无 race condition | `cargo test --features race` 或 loom 验证 | 修同步 |
| 2.5.5 | 退出码 0 | 失败 panic → 明确 exit code | 改 panic 位置 |

### 2.6 文档与可读性（Documentation）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.6.1 | 测试名自解释 | `test_wal_recovery_preserves_uncommitted_data` | 改名 |
| 2.6.2 | 关键步骤注释 | 复杂 setup 必须有 why 注释 | 加注释 |
| 2.6.3 | 引用 SPEC 条款 | 测试函数 doc-comment 引用 SPEC id | 补 doc |
| 2.6.4 | README/CONTRIBUTING 更新 | 新测试模式需文档化 | 改文档 |
| 2.6.5 | 错误信息可定位 | 失败信息含文件:行号/上下文 | 改 eprintln |

### 2.7 安全与权限（Security）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.7.1 | 无 SQL 注入 | 动态 SQL 用参数化 | 改 prepared |
| 2.7.2 | 无路径穿越 | 临时文件用 tempfile crate | 改 tempfile |
| 2.7.3 | 无密钥硬编码 | 测试无 hardcoded credential | 改 mock |
| 2.7.4 | 无越权访问 | 权限测试用最小权限 token | 改 token |
| 2.7.5 | 输入消毒 | fuzz/恶意输入不 panic | 加边界处理 |

### 2.8 集成与门禁（Integration & Gate）

| # | 检查项 | 通过标准 | 不通过处理 |
|---|--------|----------|------------|
| 2.8.1 | clippy 0 warning | `cargo clippy -- -D warnings` PASS | 修代码 |
| 2.8.2 | fmt 0 error | `cargo fmt --check` PASS | `cargo fmt` |
| 2.8.3 | 集成测试可触发 | 集成测试在 CI 默认跑 | 调 CI config |
| 2.8.4 | 门禁脚本就绪 | 对应 gate 脚本包含新测试 | 改脚本 |
| 2.8.5 | 证据可重放 | 任意审核者可一键复现 PASS | 写 Makefile |

---

## 3. REVIEW 流程

```
[实现者]  写完代码 + 测试
   ↓
[实现者]  自查 §2 全部 30 项（自查清单）
   ↓
[实现者]  git commit + push branch
   ↓
[审核者]  PR 创建后 24h 内执行 REVIEW
   ↓
[审核者]  填写 PR-XXX_TEST_REVIEW.md
   ↓
[审核者]  结论：APPROVED / REQUEST_CHANGES / BLOCKED
   ↓
[门禁]    APPROVED → 跑 ACCEPTANCE 流程
   ↓
[门禁]    不通过 → 退回 PR
```

---

## 4. PR-XXX_TEST_REVIEW.md 文件模板

```markdown
# PR-XXX TEST REVIEW

> **PR**: PR-XXX <Title>
> **Reviewer**: <独立审核者>
> **Review Date**: YYYY-MM-DD
> **Source PR-DESIGN**: PR-XXX_SPEC.md
> **Source PR-TEST_PLAN**: PR-XXX_TEST_PLAN.md
> **Source PR-TEST_DESIGN**: PR-XXX_TEST_DESIGN.md
> **Source PR-Code**: <commit SHA>

---

## 1. 审核结论

| 维度 | 结论 | 备注 |
|------|------|------|
| 测试设计 | ✅/⚠️/❌ | ... |
| 测试独立性 | ✅/⚠️/❌ | ... |
| 断言质量 | ✅/⚠️/❌ | ... |
| 真实可执行 | ✅/⚠️/❌ | ... |
| 性能稳定 | ✅/⚠️/❌ | ... |
| 文档可读 | ✅/⚠️/❌ | ... |
| 安全合规 | ✅/⚠️/❌ | ... |
| 集成门禁 | ✅/⚠️/❌ | ... |

**总评**: APPROVED / REQUEST_CHANGES / BLOCKED

---

## 2. §2.1 覆盖设计 (Coverage Design) — 详细

| # | 项 | 状态 | 证据 |
|---|----|------|------|
| 2.1.1 | 覆盖矩阵 | ✅ | test_xxx 覆盖 SPEC §3.1 |
| 2.1.2 | 边界 | ⚠️ | 缺 min/max 边界 |
| ... |

## 3. §2.2 独立性 — 详细
...

## 8. §2.8 集成门禁 — 详细
...

## 9. 复现步骤

```bash
# 完整复现命令
cargo test -p <pkg> --test <name> -- --nocapture
```

## 10. 必须修复项（REQUEST_CHANGES）

1. ... 
2. ...

## 11. 建议项（非阻断）

1. ...
```

---

## 5. 角色与责任

| 角色 | 责任 | 不能做 |
|------|------|--------|
| **实现者** | 写 SPEC/TEST_PLAN/TEST_DESIGN/Code/Tests/ACCEPTANCE | 不能自己 REVIEW |
| **审核者** | 写 TEST_REVIEW.md | 不能改测试代码 |
| **门禁执行** | 跑门禁脚本，写 ACCEPTANCE 总结 | 不能放水 |

**强制要求**：审核者 ≠ 实现者。若项目无人可审核，标"BLOCKED - need reviewer"，PR 不合并。

---

## 6. 与现有文档的关系

- `TEST_PLAN.md` — 阶段级（一个版本一个），定义"测什么大方向"
- `PR-XXX_TEST_PLAN.md` — PR 级，定义"这个 PR 测什么"
- `PR-XXX_TEST_DESIGN.md` — PR 级，定义"具体怎么测"
- `PR-XXX_TEST_REVIEW.md` — PR 级，审核"测试本身质量"
- `PR-XXX_ACCEPTANCE.md` — PR 级，验收"功能是否完成"

五件套缺一不可，PR 不允许只写 TEST_PLAN + Code 跳过 TEST_DESIGN/REVIEW/ACCEPTANCE。

---

## 7. 强制执行

- 本模板 2026-06-02 起强制执行
- 任何 v3.8.0+ PR 合并前必须含 TEST_REVIEW.md
- 缺失 → PR 状态自动 BLOCKED
- 已有 PR（合并中）补做 7 天内完成

---

**最后更新**: 2026-06-02  
**更新者**: Hermes Agent  
**关联文档**: 
- `DOC_CHECK_CORRECTION_RULES.md`
- `RELEASE_LIFECYCLE.md`
- `ISSUE_CLOSING_VERIFICATION.md`
