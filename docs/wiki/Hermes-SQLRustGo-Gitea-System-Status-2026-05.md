# Hermes + SQLRustGo + Gitea 系统综合状态 2026-05

> 本页整合 Hermes Agent 记忆、SQLRustGo 项目状态、Gitea DevStack 配置与已知问题。
> 来源：Hermes Agent 持久记忆 (memory)、用户 Profile、实时数据库探查。
> 状态时间戳：2026-05-01 11:25 CST


## 2. SQLRustGo 项目状态

### 2.1 版本与分支

| 属性 | 值 |
|------|-----|
| 当前分支 | `develop/v2.8.0` |
| VERSION 文件 | `beta/v2.8.0` |
| Git tag | `v2.8.0-beta1` |
| 主仓库 | `~/workspace/dev/openheart/sqlrustgo` |
| TRAE AI 并行工作目录 | `~/workspace/dev/yinglichina163/sqlrustgo` (v2.8.0) |

> ⚠️ 冲突预防：Hermes 应在 `openheart/sqlrustgo` 工作，TRAE AI 在 `yinglichina163/sqlrustgo`。
> 两个目录并行修改同一分支可能冲突。

### 2.2 Governance 状态 (v2.8.0)

- **R1-R7 规则组**: 226 PASS ✓
- **AV1-AV9 验证组**: 3 CRITICAL 未过 (AV3, AV4, AV5) — 阻塞发布
- **P0 问题**: 全部完成 ✓
- **待完成文档**: `gate_spec.md`, `release_process.md`, `Governance-System.md` (wiki)
- **待推送 PR**: `docs/update-v280-rc-state-20260501` → `develop/v2.8.0`

### 2.3 开发规范 (关键规则)

- **完成一个任务 → PR → 审核合并 → 再继续下一个**。不允许连续编码不合并。
- **数据库"绝对可信"原则**: 所有 DB 数据可直接用于调试。
- **性能测试**: 缩小数据集代替 `ignore`，保持测试运行。
- 测试名称反映真实数据量（如 `test_ivfpq_10k_performance`）
- 根因调试：逐个 crate 运行测试定位超时，不盲目猜测

### 2.4 Git 身份

SQLRustGo 仓库的 `pre-commit` hook 强制 `user.email="openheart@gaoyuanyiyao.com"`。
其他 email 的 commit 会失败并提示 "❌ Wrong Git identity!"。


## 4. HP Z6G4 服务器状态

| 指标 | 值 |
|------|-----|
| 主机名 | gaoyuan (node ID `cc67e32a`) |
| CPU | 80 核心 |
| 内存 | 409 GB |
| OS | Linux (Nomad agent) |
| Nomad | v1.9.6 running |
| Gitea | 1.22.1 running |
| act_runner | 4 进程 (PID 8249/8265/8318/8376) |

### 4.1 磁盘清理记录 (2026-04-30)

Mac Mini 磁盘清理：8.5G → 50G 可用 (19%)，释放 41.5G。

清理项：
- `heartopen/` (30G, git 对象过时)
- `yinglichina163/worktrees/` (2.6G)
- `~/.npm/` (6G)
- `~/.cache/uv/` (2.4G)
- `~/Library/Caches/Homebrew/` (1.2G)
- `Trae CN/` (559M)
- `~/.cache/go-build/` (581M)
- `~/.cache/huggingface/` (313M)
- `~/.cache/pip/` (147M)

定期维护命令：`npm cache clean && rm -rf ~/.cache/uv ~/.cache/huggingface ~/.cache/qmd`


## 6. 当前阻塞项

### 6.1 P0 — Nomad Runner 部署失败

- **问题**: Nomad job `gitea-runner` 部署失败，59 个失败任务，0 个健康 runner
- **影响**: Jobs 680-689 全部卡住，CI 完全停摆
- **需要**: 调查 Nomad 部署失败根因，修复 job spec，重新部署

### 6.2 P0 — AV3/AV4/AV5 Critical 测试未过

- **问题**: SQLRustGo Governance AV1-AV9 中 3 个 CRITICAL 测试阻塞发布
- **需要**: 定位并修复 AV3, AV4, AV5

### 6.3 P1 — 文档待完成

- `gate_spec.md`
- `release_process.md`
- `Governance-System.md` (wiki 更新)

### 6.4 P1 — 待推送 PR

- `docs/update-v280-rc-state-20260501` → `develop/v2.8.0`

---

## 7. 相关页面

- [sqlrustgo](../wiki/sqlrustgo) — 项目概况
- [gitea-devstack](../wiki/gitea-devstack) — Gitea 基础设施配置
- [ci-cd-platform](../wiki/ci-cd-platform) — CI/CD 平台架构
- [nomad-scheduler](../wiki/nomad-scheduler) — Nomad 调度系统
- [governance-system](../wiki/governance-system) — 治理模型
- [hermes-agent](../wiki/hermes-agent) — Hermes Agent 自身配置
- [gitea-cloud-sync](../wiki/gitea-cloud-sync) — 云端同步机制
- [mac-mini-disk-maintenance](../wiki/mac-mini-disk-maintenance) — 磁盘维护指南