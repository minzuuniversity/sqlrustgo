# Tasks — fix-5380-sync-ssh-hang

## T1 — RED：先写会失败的回归测试

- [x] T1.1 新建 `scripts/sync/test_5remotes_sync_ssh.sh`
- [x] T1.2 S1 静态检查：ssh 选项完整性 / 无明文凭据 / 无全盘 `find /`
- [x] T1.3 S2 行为测试：stub `ssh` 忠实模拟「失效 agent → 阻塞」，
      用**本地 bare repo** 充当 5 个远端，stub `curl` 充当 Gitea API
- [x] T1.4 S3 行为测试：断言远端命令构造正确（`update-ref` + 临时 ref 清理）
- [x] T1.5 **确认 RED**：对当前未修复的脚本运行 → PASS=0 / FAIL=11，
      且脚本**真的挂到 Step 3 被看门狗杀掉**（退出码 124）

## T2 — GREEN：最小实现

- [x] T2.1 抽出统一 SSH 选项（`IdentityAgent=none` / `IdentitiesOnly=yes` /
      `BatchMode=yes` / `ConnectTimeout` / `ServerAlive*`），替换全部 ssh 调用点
- [x] T2.2 调用方未设 `GIT_SSH_COMMAND` 时由脚本自行设置（覆盖 git 的 SSH 通道）
- [x] T2.3 容器仓库发现改为定向探测（已知布局优先，`find` 收窄兜底）
- [x] T2.4 移除硬编码明文凭据 → `GITEA_USER` / `GITEA_TOKEN`，缺失即明确报错
- [x] T2.5 新增 `GITEA_HOSTS` / `GITEA_PUSH_URL` 覆盖开关
- [x] T2.6 **确认 GREEN**：PASS=11 / FAIL=0

## T3 — REFACTOR

- [x] T3.1 选项定义集中，无重复字面量
- [x] T3.2 错误信息可操作（指明该设哪个环境变量）
- [x] T3.3 `scripts/sync/README.md` 同步更新
- [x] T3.4 `usage()` 改为按「首个代码行」截断，不再写死行号
- [x] T3.5 空 `GITEA_HOSTS` 显式拒绝（bash 3.2 空数组 + `set -u`）
- [x] T3.6 shellcheck 告警清零（本次改动引入的部分）

## T4 — GATE

- [x] T4.1 `bash scripts/sync/test_5remotes_sync_ssh.sh` → PASS=11 / FAIL=0
- [x] T4.2 真实 5 远端同步跑通（脚本本体，非手工命令）→ 退出码 0
      （先人为制造漂移：gitea250 回退一个提交到 `7a73c42397`）
- [x] T4.3 `bash -n` 语法检查；`shellcheck -S warning` 对改动文件 clean
- [x] T4.4 变异验证：3 个变异全部 CAUGHT
      （MUT-3 首轮存活 → 静态检查改为匹配代码行的 `export GIT_SSH_COMMAND=`）
- [x] T4.5 凭据门禁自检：注入 4 种泄露形态，全部被捕获；移除后恢复 PASS

## T5 — PR / RECORD / LEARN

- [x] T5.1 证据文档 `docs/releases/v4.1.0/evidence/SYNC_SCRIPT_5380_2026-10-20.md`
- [x] T5.2 lessons `docs/governance/lessons/LESSON_5380_SYNC_SSH_2026-10-20.md`
- [x] T5.3 在 #5380 / #5381 留证据评论
- [ ] T5.4 提交 + PR + 合并

## 未闭环（需 owner 配合，见 #5381）

- 凭据**轮换**——移除仓库里的明文 ≠ 凭据失效，历史 git 历史中仍有
- 13 个文档文件中的凭据字面量——需人工逐条改写并保留证据链
- `git filter-repo` 清理历史——破坏性操作，需 owner 决策
