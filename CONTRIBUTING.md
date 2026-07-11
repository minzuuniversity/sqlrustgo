# Contributing to SQLRustGo / SQLRustGo 贡献指南

> **Maintainer**: openclaw <openclaw@gaoyuanyiyao.com>
> **Status**: Active
> **Last Updated**: 2026-07-11
> **当前版本**: v3.9.0 (GA, 2026-07-10)

欢迎为 SQLRustGo 做出贡献！本文件提供贡献代码的快速指南。

---

## 快速链接

- [架构决策](docs/governance/adr/) — 设计依据
- [发布生命周期](docs/governance/RELEASE_LIFECYCLE.md) — Alpha/Beta/RC/GA 流程
- [版本历史](docs/releases/) — 所有历史版本
- [当前版本文档](docs/releases/v3.9.0/) — v3.9.0 GA 文档

---

## 开发环境 / Development Setup

```bash
# 克隆仓库
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo

# 安装 Rust (1.85+ 推荐)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 构建项目
cargo build --all-features

# 运行测试
cargo test --all-features
```

---

## 开发工作流 / Development Workflow

1. **选择 issue** — 从 Gitea issue 列表选择一个（或新建一个）
2. **分支** — 从 `develop/v3.9.0`: `git checkout -b fix/<descriptive-name>`
3. **实现** + 测试 + 提交 — `git commit -m "fix(scope): description"`
4. **推送** — 到 fork 或 Gitea remote
5. **创建 PR** — 指向 `develop/v3.9.0`

---

## 代码规范 / Code Style

- 遵循 `rustfmt` 默认规范: `cargo fmt --all`
- 通过 clippy 检查: `cargo clippy --all-features -- -D warnings`（零警告）
- 公共 API 使用 `///` 文档注释
- 新功能需添加单元测试

---

## 提交信息规范 / Commit Message Convention

```
<type>(<scope>): <简短描述>

[详细说明]

[关联的 issue 引用]
```

**类型 (type)**: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

**范围 (scope)**: `executor`, `storage`, `parser`, `planner`, `wal`, `gate`, `docs`

**示例**:
```
fix(wal): 修正 WalStorage 提交路径中的 LSN 分配

之前所有 WAL 条目都被分配 lsn=0，导致检查点推进永不触发。
添加每实例 LSN 计数器，并将所有追加操作路由通过它。

Fixes #2588
```

---

## 测试 / Testing

```bash
# 运行单元测试
cargo test --lib

# 运行文档测试
cargo test --doc

# 运行特定测试
cargo test -p sqlrustgo-executor test_name

# 覆盖率（需要 cargo-llvm-cov）
cargo llvm-cov --all-features --workspace
```

---

## Pull Request 流程 / Pull Request Process

1. 如果是面向用户的变更，更新 `CHANGELOG.md`
2. 确保 CI 通过（clippy + tests + format）
3. 请求维护者审核
4. 审核通过后 squash-merge

---

## 治理 / Governance

- 所有变更需经过 **Alpha → Beta → RC → GA** 门禁
- 架构变更需要 ADR（架构决策记录）
- 真实性框架适用 — 不得伪造测试结果

详见 `docs/governance/` 下的完整治理框架。

---

## 许可证 / License

贡献代码即表示同意您的贡献将使用与项目相同的许可证（MIT）。

---

## 问题？/ Questions?

创建 issue 或联系维护者。
