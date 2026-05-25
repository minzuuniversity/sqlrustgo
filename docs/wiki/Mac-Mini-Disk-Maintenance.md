# Mac Mini 磁盘维护

> 开发环境磁盘清理指南，避免因磁盘满导致开发工具异常。

## 空间利用率历史

| 时间 | 可用 | 已用 | 操作 |
|------|------|------|------|
| 2026-04-30 清理前 | 8.5G | 58% | — |
| 2026-04-30 清理后 | 50G | 19% | 释放 **41.5G** |

## 常见空间大户

| 路径 | 典型大小 | 清理方法 |
|------|---------|---------|
| `workspace/dev/*/sqlrustgo/.git/objects` | 5-30G | `git gc --aggressive` 或删除废弃 clone |
| `workspace/dev/*/sqlrustgo/.worktrees` | 1-3G | `rm -rf .worktrees/ && git worktree prune` |
| `~/.npm/_cacache` | 6G | `npm cache clean --force` |
| `~/.cache/uv` | 2.4G | `rm -rf ~/.cache/uv/` |
| `~/Library/Caches/Homebrew` | 1.2G | `rm -rf ~/Library/Caches/Homebrew/*` |
| `~/Library/Caches/Trae CN` | 560M | `rm -rf ~/Library/Caches/Trae\ CN/` |
| `~/Library/Caches/go-build` | 580M | `rm -rf ~/Library/Caches/go-build/` |
| `~/.cache/huggingface/` | 310M | `rm -rf ~/.cache/huggingface/` |
| `~/.cache/qmd/` | 100M | `rm -rf ~/.cache/qmd/` |
| `~/.cache/oh-my-opencode/` | 6M | `rm -rf ~/.cache/oh-my-opencode/` |
| `~/Library/Caches/pip/` | 150M | `rm -rf ~/Library/Caches/pip/` |

## git 对象膨胀的原因

SQLRustGo 仓库的 `.git/objects` 容易膨胀，原因：

1. **Rust 编译缓存**并不在 git objects 中（在 `target/`）
2. **多重 clone** — 同一个项目克隆了 3 次（openheart, heartopen, yinglichina163）
3. **Full history** — 每次克隆拉取完整 git 历史（包含所有 branch、worktree 对象）
4. **worktree 残留** — `.worktrees/` 各自携带独立的 git 对象数据库

## 定期维护脚本


## 预防措施

- **不要** `git clone --depth 1` 以外的完整克隆（除非必须）
- 废弃的 clone 直接删除（heartopen 30G 即为此类）
- worktree 使用完毕后及时清理
- 定期运行 `npm cache clean --force`
- `uv`/`huggingface`/`qmd` 等 AI 工具的缓存不会自动清理

## 工作目录结构（清理后）


## 相关

- [agent-development-workflow](../wiki/agent-development-workflow) — AI 开发工作流
- [ci-cd-platform-deployment](../wiki/ci-cd-platform-deployment) — CI/CD 平台