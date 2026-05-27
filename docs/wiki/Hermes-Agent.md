# Hermes Agent

Hermes Agent 是一个 CLI AI 代理系统，运行于 Mac Mini 上，通过 `~/.hermes/` 目录配置。支持多种 AI 模型后端（OpenRouter、Anthropic、DeepSeek、MiniMax 等）和跨平台交付（Telegram、Discord、CLI、WhatsApp）。

## 架构概要

- **进程管理**: `launchd` 守护进程 `com.hermes.agent.plist`，保持后台持续运行
- **配置路径**: `~/.hermes/config.yaml`
- **记忆系统**: `~/.hermes/memory/` — 持久化跨会话记忆，每个会话自动注入
- **技能系统**: `~/.hermes/skills/` — 可复用的程序性记忆（SKILL.md + 引用文件）
- **工具集**: terminal, file, web_search, browser, delegate_task, cronjob, etc.

## 关键特性

- **持久记忆**: 分 `memory`（环境事实）和 `user`（用户画像）两个存储
- **技能系统**: 条件触发 + 编号步骤 + 陷阱 + 验证步骤
- **会话搜索**: 基于 FTS5 的跨会话全文检索
- **子代理**: `delegate_task` — 并行子任务，支持嵌套编排（orchestrator）
- **后台进程**: terminal(background=true) + notify_on_complete 模式
- **定时任务**: `cronjob` — create/list/update/pause/resume/remove/run

## 用户配置

- **用户工作风格**: 首席架构师 — 规则优先、形式化输出、约束驱动
- **语言偏好**: 中英混用，正式表达（∀, ∃, IF/THEN）
- **SSH 首选**: 直接 SSH 到服务器，拒绝"请你运行"的中间人模式
- **sudo 密码**: "details"

## 记忆系统设计原则

- 只保存持久性事实（用户偏好 > 环境事实 > 程序性知识）
- 不保存任务进度、会话结果或 TODO 状态
- 复杂工作流 → 保存为 skill
- 陈旧/错误技能 → 立即 patch，不等用户要求

## 相关页面

- [sqlrustgo](../wiki/sqlrustgo) — 承载的记忆 80% 为 SQLRustGo 项目
- [gitea-devstack](../wiki/gitea-devstack) — Gitea 基础设施通过 terminal tool 管理
- [qmd-search](../wiki/qmd-search) — wiki 搜索系统的互补工具
- [gbrain-knowledge](../wiki/gbrain-knowledge) — 三层知识体系的一部分