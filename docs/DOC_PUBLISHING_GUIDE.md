# SQLRustGo 文档发布指南

> **版本**: v1.0
> **更新日期**: 2026-05-29
> **维护人**: hermes-z440

---

## 一、文档类型与发布方式

SQLRustGo 文档分为两类：

### 1. 源码文档（Canonical Source）

这些文档**必须**保持在源码仓库中，因为它们与代码同步更新：

| 文档 | 位置 | 说明 |
|------|------|------|
| OpenAPI 规范 | `docs/api/openapi.yaml` | HTTP API 的正式规范 |
| rustdoc HTML | `target/doc/` | 编译时生成，非提交 |
| OO 文档索引 | `docs/oo/README.md` | 版本追踪的 OO 文档入口 |
| API 文档入口 | `docs/api/README.md` | rustdoc + OpenAPI 导航 |

**生成命令**:
```bash
# OpenAPI 规范（utoipa 生成）
cargo doc -p sqlrustgo-network --no-deps
# 或使用 utoipa-gen 从代码注解生成

# rustdoc HTML
cargo doc --all-features --no-deps
cargo doc --open  # 浏览器查看
```

### 2. Gitea Wiki 文档

Gitea Wiki 用于**补充性文档**，如项目概述、架构决策、开发指南：

- **地址**: http://192.168.0.252:3000/openclaw/sqlrustgo/wiki
- **API**: `POST /api/v1/repos/openclaw/sqlrustgo/wiki/pages`
- **已有**: 28 个 wiki 页面

**Wiki 同步脚本**（规划中）:
```bash
# 将 docs/ 下的文档同步到 Gitea Wiki
scripts/doc/sync-wiki.sh
```

---

## 二、文档生成模式

### 当前模式（v3.5.0）

```
openapi.rs (utoipa 注解)
    ↓ cargo doc -p sqlrustgo-network
docs/api/openapi.yaml (提交到源码)
    ↓
用户: git clone → cargo doc --open
    或直接查看 docs/api/openapi.yaml
```

### 理想模式（规划中）

```
openapi.rs (utoipa 注解)
    ↓ CI: cargo doc
gh-pages branch (自动部署)
    ↓
https://openclaw.github.io/sqlrustgo/ (静态站点)
```

**注意**: Gitea 当前不支持此模式，需要外部 CI/CD。

---

## 三、文件分类规范

| 文件类型 | 是否提交到源码 | 说明 |
|----------|---------------|------|
| `.md` 源码文档 | ✅ 必须 | CHANGELOG, README, DEV_PLAN 等 |
| `openapi.yaml` | ✅ 必须 | OpenAPI 规范（utoipa 生成） |
| `openapi.rs` | ✅ 必须 | utoipa 注解模块 |
| `rustdoc HTML` | ❌ 禁止 | 由 `cargo doc` 生成，不提交 |
| `target/` | ❌ 禁止 | 编译产物，.gitignore |
| Wiki 页面 | ✅ 通过 API 同步 | 非源码，通过 wiki API 管理 |

---

## 四、rustdoc 生成流程

### 本地生成

```bash
# 启用私有项文档（已配置在 .cargo/config.toml）
cargo doc --all-features --no-deps

# 查看
cargo doc --open
```

### CI 生成（规划中）

```yaml
# .github/workflows/docs.yml
- name: Generate rustdoc
  run: cargo doc --all-features --no-deps

- name: Deploy to Pages
  if: github.ref == 'refs/heads/main'
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./target/doc
```

---

## 五、Gitea Wiki 同步

### 当前 Wiki 页面

| 页面 | 说明 |
|------|------|
| Architecture | 系统架构 |
| Home | Wiki 入口 |
| GMP Platform Overview | GMP 平台概述 |
| Governance System | 治理体系 |
| 等 | ... |

### Wiki 同步脚本（规划）

```bash
#!/bin/bash
# scripts/doc/sync-wiki.sh

WIKI_API="http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/wiki/pages"
TOKEN="$GITEA_PAT"

# 同步 docs/oo/ 到 Wiki
for md in docs/oo/*.md; do
  title=$(basename "$md" .md)
  content=$(cat "$md")
  curl -X POST "$WIKI_API" \
    -H "Authorization: token $TOKEN" \
    -d "{\"title\":\"$title\",\"content\":\"$content\"}"
done
```

---

## 六、快速参考

| 任务 | 命令 |
|------|------|
| 生成 rustdoc | `cargo doc --all-features --no-deps` |
| 查看 rustdoc | `cargo doc --open` |
| 生成 OpenAPI | `cargo doc -p sqlrustgo-network` |
| 检查格式 | `cargo fmt --check` |
| 运行测试 | `cargo test --all-features` |

---

## 七、相关文件

- [DOC_IMPROVEMENT_PLAN.md](../DOC_IMPROVEMENT_PLAN.md)
- [OSS_DOC_ANALYSIS.md](../OSS_DOC_ANALYSIS.md)
- [docs/api/README.md](api/README.md)
- [docs/oo/README.md](../oo/README.md)