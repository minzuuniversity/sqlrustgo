<!--
PULL_REQUEST_TEMPLATE.md — SQLRustGo PR 模板
依据: Issue #2883 (P1-5) + Issue #2917 (P1-4 CI gate integration)
生效: v3.8.0 起所有 PR 必填
关联: docs/governance/TEST_REVIEW_TEMPLATE.md (5-类文档)
-->

## 关联 Issue

- Issue 编号：`#`
- 类型：[Bug / Feature / Refactor / Doc / Gate / Spec]
- 5-Principle: P1 / P2 / P3 / P4 / P5
- Node 编号: N

---

## 一、5-类文档清单（强制）

> 每项必须在 PR 描述或链接文档中提供。无 = 自动 FAIL。

- [ ] **SPEC** — `docs/releases/v3.8.0/PR-XXX_SPEC.md` (设计/范围/接口)
- [ ] **TEST_PLAN** — `docs/releases/v3.8.0/PR-XXX_TEST_PLAN.md` (覆盖目标/阶段)
- [ ] **TEST_DESIGN** — `docs/releases/v3.8.0/PR-XXX_TEST_DESIGN.md` (具体测试方法)
- [ ] **REVIEW** — `docs/releases/v3.8.0/PR-XXX_TEST_REVIEW.md` (独立审核记录)
- [ ] **ACCEPTANCE** — `docs/releases/v3.8.0/PR-XXX_ACCEPTANCE.md` (门禁执行证据)

> 模板: 参考 `docs/governance/TEST_REVIEW_TEMPLATE.md`

---

## 二、Gate Integration Declaration（强制）

> 此 PR 是否新增/修改/启用了任何 gate 维度？勾选所有相关：

- [ ] D1-Alpha（TPC-H / 核心 SQL）
- [ ] D2-Beta（SQL 92 / 窗口 / CTE）
- [ ] D3-SGL（共享全局锁）
- [ ] D4-WAL（崩溃恢复）
- [ ] D5-DeepSeek（体验分）
- [ ] **D6-Integration**（49+ 集成测试，P0-1 引入）
- [ ] **D5.5-Test Plan Audit**（TEST_PLAN ↔ Cargo.toml，P1-3 引入）
- [ ] **D7-Arch/Sem Debt**（ARCH-1~3 + SEM-1~4，P1-2 引入）
- [ ] 其他：_________________

如果勾选任何维度，**必须**说明：
- 触发阶段: alpha / beta / rc / ga / all
- 触发脚本: `bash scripts/gate/...`
- 输出: `artifacts/gate/v3.8.0/...`

---

## 三、Test Command Output（强制）

```bash
# 必跑：clippy + fmt + build
cargo clippy --all-features -- -D warnings
cargo fmt --check --all
cargo build --all-features

# 必跑：相关 test
cargo test --test <name> --all-features
```

**实际结果**:
- clippy: `[PASS/FAIL]`
- fmt: `[PASS/FAIL]`
- build: `[PASS/FAIL]`
- test: `[X/Y PASS, 0 FAIL]`

> 任何 FAIL 必须在此 PR 修复，否则关闭。

---

## 四、Evidence 清单

- [ ] `artifacts/gate/v3.8.0/evidence.json` 已生成（若涉及 gate 改动）
- [ ] PR description 中含 `stdout_sha256` 引用（若涉及 alpha gate）
- [ ] `bash scripts/gate/audit_testing.sh v3.8.0 <stage> <out>` 输出 PASS/WARN
- [ ] Markdown 链接: `bash scripts/gate/check_docs_links.sh` PASS

---

## 五、合并检查

- [ ] 分支: `fix/issue-<NUMBER>-<slug>` (从 `develop/v3.8.0` 切出)
- [ ] Commit 格式: `feat/fix/chore(scope): <message>`
- [ ] 关联 PR 已 force_merge 或正常合并
- [ ] Worktree 已清理
- [ ] Issue 已关闭（如果 Closes 关键字）

---

## 六、风险评估

- 兼容性: [None / Backward-compat / Forward-compat / Breaking]
- 性能影响: [+/-/0, 量化数据]
- 安全影响: [+/-/0, 描述]
- 数据迁移: [None / Required]

---

## 七、Checklist

- [ ] 关联 Issue 已指定
- [ ] 5-类文档已提供
- [ ] Gate integration 已声明
- [ ] Test command output 已填写
- [ ] Evidence 清单已勾选
- [ ] 合并检查清单已勾选
- [ ] 风险评估已填写

---

> **本模板强制执行**: 任何缺失项 = PR 自动 require-changes
> **维护**: Hermes C (hermes@sqlrustgo.ai)
> **关联**: Issue #2883 (P1-5) / Issue #2917 (P1-4) / Issue #2918 (P1-3)
