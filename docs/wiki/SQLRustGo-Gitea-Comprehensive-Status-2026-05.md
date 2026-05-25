# SQLRustGo + Gitea DevStack 综合状态总览 (2026-05)

## 1. SQLRustGo 项目状态

### 当前版本（v2.9.0 — Baseline Verified ✅）
- **VERSION**: `develop/v2.9.0`
- **分支**: `develop/v2.9.0`（唯一开发分支）
- **测试**: 3630 tests, 28 集成, Corpus 85.4%
- **已合并 PR**: #203(DDL+MVCC), #209(64测试), #210(126测试), #211(190测试)
- **待修复**: EXISTS, LEFT JOIN完整, CREATE VIEW, ALTER TABLE, REPLACE INTO

### 版本阶段规则
- 阶段标识写在 `VERSION` 文件，不使用 `release/beta` 等分支名
- 所有开发在 `develop/v2.x.y` 分支
- 只有 RC 阶段才创建 `release/v2.x.y` 分支

### v2.9.0 门禁状态
| 门禁项 | 状态 | 备注 |
|--------|------|------|
| A-Gate (编译+测试) | ✅ PASS | 3630 tests |
| Clippy | ✅ PASS | zero warnings |
| Fmt | ⚠️ 待查 | bench-cli/main.rs 格式问题 |
| Hermes Gate | ✅ PASS | gate/hermes_gate.sh |

### 待完成 Issue
| ID | 优先级 | 描述 | 状态 |
|----|--------|------|------|
| #46 | P0 | 测试覆盖率 ≥70% | 未开始 |
| #41 | P1 | MySQL 5.7 兼容 | 部分 cherry-pick |
| #37/PR#51 | P1 | 列级权限系统 | PR 合并失败(422) |
| #53 | P2 | CI 优化 (PR#54) | 待合并 |
| #47 | P1 | 文档审核 | 未开始 |
| #48 | P2 | 分类整理 | 未开始 |


## 3. CI/CD 平台
### CI/CD 平台（Runner 健康 ✅）

#### Gitea Actions Runner 状态（2026-05-03）
- **Gitea**: v1.22.1，Actions API `/actions/runners` 端点不存在（非 bug，Gitea 1.22.x 限制）
- **Runner 诊断**: DB 直接查询 `action_runner` 表
- **当前状态**: 19 注册，4 在线（nomad-runner-v5 + devstack-runner-1~4）
- **最新 Job**: #177 success (2026-05-02 23:17 UTC)

#### 已知限制
- Gitea 1.22.1 Swagger 确认：无 `/actions/runners` 等 CRUD API
- Runner 监控: `docker exec devstack-postgres-1 psql -U gitea -d gitea -c "SELECT id, name, online, last_online FROM action_runner ORDER BY id DESC LIMIT 20;"`
- 历史累积: 19 个 runner 注册（多次迭代残留），不影响功能

### 架构
- **Gitea Actions**: v1.22.1 Docker on Z6G4
- **Nomad**: v1.9.6 双节点（hp-z6g4 / z440）
- **Runner 镜像**: `sqlrustgo-runner:nomad-root-v2`（USER root）
- **并发容量**: 8 总任务（4 devstack-runner + 4 Nomad runner）
- **禁用**: `actions/cache` 和 `actions/upload-artifact`（Gitea 1.22.1 下会卡死 CI）

### Nomad Job v23 配置

### 历史根因（v03→v23，10次迭代）
| # | 根因 | 修复 |
|---|------|------|
| 1 | entrypoint.sh TOKEN 截断 | 修正 shell heredoc |
| 2 | Nomad 约束使用短 node.id | 使用 node.class=runner |
| 3 | 镜像标签 nomad-v006 不存在 | 构建并推送新镜像 |
| 4 | USER 1000 写权限 + CONFIG_FILE 污染 | USER root, CONFIG_FILE="" |

### CI 工作流优化（PR#54，待合并）
- `paths-ignore`: 文档变更跳过全量编译
- `cargo cache`: registry + target 增量
- `lint` 提前到 build 前（fail fast）
- `fetch-depth: 1`: 快速克隆
- 新增 `docs-check` 轻量 job


## 5. 多平台开发环境

### 平台分配

| 平台 | 角色 | 主要任务 |
|------|------|----------|
| Z6G4 (80c) | OpenCode | 列级权限 (PR#51) |
| Z440 (28c) | Hermes | Load Balancing (PR#45) |
| TRAE AI (Mac) | TRAE | MySQL 兼容, ReadWriteSplit |
| Mac Mini | Hermes 主控 | 测试覆盖, 门禁, 分类 |

### 工作目录
- **主开发**: `~/workspace/dev/openheart/sqlrustgo`（Gitea origin）
- **TRAE AI**: `~/workspace/dev/yinglichina163/sqlrustgo`（需避免冲突）

### 强制开发工作流（AGENTS.md）
所有 AI 代理必须遵循 Superpowers 流程，不允许直接编码。


## 7. Wiki 与知识管理

### 当前 Wiki 状态
- **位置**: `~/wiki`（WIKI_PATH）
- **页面数**: 24 页（`master` 分支渲染）
- **同步**: `~/wiki/sync_to_gitea_wiki.py` → `openclaw/sqlrustgo.wiki`（master 分支）
- **Cron**: 每 15 分钟 cloud-sync

### 关键 Wiki 页面
- [sqlrustgo-gitea-status-2026-04](../wiki/sqlrustgo-gitea-status-2026-04) — 旧版状态（2026-04）
- [ci-cd-platform-deployment](../wiki/ci-cd-platform-deployment) — CI/CD 完整架构
- [gitea-nomad-health-checks](../wiki/gitea-nomad-health-checks) — 健康检查清单
- [v2.8.0-progress](../wiki/v2.8.0-progress) — v2.8.0 进度与门禁
- [v2.8.0-beta-schedule](../wiki/v2.8.0-beta-schedule) — Beta 调度策略
- [gitea-cloud-sync](../wiki/gitea-cloud-sync) — 云端同步机制


## 9. 未解决问题（Open Items）

| # | 描述 | 优先级 | 阻塞 |
|---|------|--------|------|
| 1 | PR#54 CI 优化合并 | P1 | 需手动或临时禁用 status check |
| 2 | Issue #46 测试覆盖率 | P0 | 无 |
| 3 | Issue #41 MySQL 兼容 | P1 | 57 个测试失败 |
| 4 | PR#51 列级权限合并 | P1 | 422 错误，token 权限不足 |
| 5 | OpenCode Git remote 指向 | P2 | 可能继续推 GitHub |
| 6 | hermes-z6g4/hermes-devstack 账户名 | P3 | 配置不符 |
| 7 | Nomad 旧镜像标签清理 | P3 | registry 杂乱 |
| 8 | GitHub main 落后 | P3 | 不影响，等 GA 时覆盖 |