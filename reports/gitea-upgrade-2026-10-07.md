# Gitea 双服升级整改报告（2026-10-07）

> 指令：整改发现的问题，gitea 服务器注意先备份再升级。
> 范围：192.168.0.250（本机 gitea-test）、192.168.0.252（openclaw-gitea-1）。
> 结论先行：**250 备份 + 升级 + AC 验证全部完成**；**252 整机失联，升级阻塞，恢复步骤已备好**。

## 1. 版本决策

| 项 | 内容 |
|---|---|
| 原版本 | 1.27.3（双服，API `GET /api/v1/version` 实测） |
| 目标版本 | **28.1.0**（2026-10-06 最新稳定；28.0.0 为 BREAKING：git 内部代理 #39426、RUN_RETENTION_DAYS） |
| 选取理由 | 28.1.0 含 28.0.x 热修；PR #39126（Actions token 合并假成功）closed 未合入，AC 修复只能实证验证，不能引 doc claim |
| 镜像通道 | docker.m.daocloud.io（可达）+ crane pull → docker load → tag（docker daemon 死代理 20171 不可用且 NO_PROXY 含 192.168.0.0/16，禁重启 daemon——Live Restore=false 会停全部容器） |

## 2. 250 备份（用户硬要求 ✓）

目录：`/home/ai/backups/gitea-preupgrade-250-20261007-092538Z/`（801M）

| 文件 | 大小 | 说明 |
|---|---|---|
| `gitea-db.pgdump` | 14M | pg_dump 全库，stderr 空 |
| `gitea-data.tar.gz` | 24M | 含 conf/app.ini、jwt/private.pem、git/.ssh |
| `git-data.tar.gz` | 763M | 仓库裸库全量 |
| `inspect-{gitea-test,postgres,runner}.json` | 36K | 容器 env/挂载/网络/重启策略 |
| `CHECKSUM.sha256` | — | **复核 6/6 成功**（sha256sum -c，exit=0；`pg_dump.stderr` 0 字节未入清单） |

回滚点：旧容器 `gitea-test-old-1.27.3`（Exited(0)）+ 镜像 `gitea/gitea:1.27.3` 保留本地，rename+start 秒回滚；DB 回滚靠 pgdump。

## 3. 250 升级过程

1. stop → rename 旧容器（回滚点）。
2. docker run 同构重建：env 与备份 inspect **逐项 diff = missing none / extra none**；网络 `gitea-test_gitea-test-net`、端口 2222:22 + 3000:3000、挂载不变；restart=no 不变。
3. 迁移 343-357 全过，`GET /api/v1/version` = `{"version":"28.1.0"}`。

## 4. 升级回归问题：PR 创建 500（已修复）

**症状**：`POST /api/v1/repos/openclaw/sqlrustgo/pulls` → 500，`unable to fetch head branch ..., err: exit status 128`；同仓已有分支间也复现。

**根因定位**（GIT_TRACE2_EVENT 实证，非猜测）：

```text
{"event":"error", "msg":"Unable to create '/git/openclaw/sqlrustgo.git/./objects/info/
commit-graphs/commit-graph-chain.lock': File exists.\n\nAnother git process seems to
be running in this repository, or the lock file may be stale"}
{"event":"exit", "code":128}
```

- 锁文件 mtime **2026-09-11**（升级前 26 天的陈旧锁 + 孤儿 `tmp_graph_uCqMrR`），`ps` 无任何 git 进程持有。
- 28.x fetch 阶段注入 `fetch.writeCommitGraph`（trace2 可见 gitea 主动读取该配置）→ write-commit-graph 撞陈旧锁 → exit 128；1.27.3 不执行该步骤，故此前正常。
- 手工复现 exit=0 是因为手工路径未触发 commit-graph 写入（环境差异），属排查中已排除的假阴性。

**修复**：备份锁文件至 `/tmp/stale-lock-backup/` 后删除两文件；全仓库 `find /git -name "*.lock" -mmin +5` 扫描无其他残留。修复后 `POST /pulls` → **HTTP 201**。

## 5. AC 验证矩阵（250 / 28.1.0 实跑）

工具：`scripts/sync/verify_merge_ref.sh --selftest` + curl API 实测。

| AC | 场景 | 结果 | 证据 |
|---|---|---|---|
| AC1/AC4 | merge 200 + merged=True + **base ref tip == merge_commit_sha** | **PASS-EXACT ×2** | 首轮 `c1d2b359988f`；干净容器二次回归 `2c2740c8af6d`（ref tip 与 merge_sha 逐一相等），EXIT=0 |
| AC3a | merge 不存在 PR (999999) | 404 ✓ 非 200 | `pull request does not exist` |
| AC3b | 冲突 PR（v4.1.0→v4.0.0，`git merge-tree` 实证 CURRENT_VERSION.md 冲突） | 405 ✓ 非 200 | `mergeable=False` + "Please try again later"，属正确拒绝非假成功 |
| AC3c | 无冲突 PR double-merge | merge1=200 / merge2=405 ✓ | `The PR is already merged`；GET `merged=True, merge_commit_sha=da145c9f…` |
| AC2 | PATCH/POST/DELETE `/git/refs*` | 405（**定性：非 bug**） | v28.1.0 源码 `routers/api/v1/api.go`：`m.Get("/refs")` / `m.Get("/refs/*")` **仅注册 GET**，API 设计无写方法 |
| 探测 PR | tmp-probe* / tmp-ac3* / ac3base / ac3feat | 已全部清理 | DELETE 204；分支残留扫描 `[]` |

**说明**：AC2 的 405 在 1.27.3 与 28.1.0 表现一致，为 API 方法集设计（源码实证），不属于升级回归；如需服务端写 refs 能力需上游特性，不属本次整改范围。

## 6. 预存问题记录（按范围约束不修）

| 问题 | 证据 | 处置 |
|---|---|---|
| runner-test 容器坏死 | `/data` 空、无 `.runner`、`GITEA_INSTANCE_URL=http://gitea:3000`（DNS 别名不存在）、516,366 次 "token is empty" 重试（远超升级后窗口→升级前已坏） | 记录不修（DoD 外） |
| `ci.yml` 重复键 | `mapping key "run" already defined at line 291` | 仓库内容预存问题，不修 |
| CODEOWNERS 告警 | `incorrect codeowner user: yinglichina8848 / maintainer`（3 处） | 预存，不修 |

## 7. 252 状态：已恢复并完成升级（先备份 ✓）

- 09:46Z–12:05Z 整机失联 **2 小时 19 分钟**（物理层故障，机房介入后于 12:05Z 恢复）；ping 0% 丢包、API 实测 `{"version":"1.27.3"}`。
- **恢复后执行（一键脚本实跑成功）**：
  - **备份先行 ✓**：`/home/openclaw/gitea-preupgrade-252-20261007-120948Z`（910M：pgdump + gitea-data/git-data/pg-data 三卷 tar + compose + inspect + CHECKSUM）→ **scp 拉回 250** `/home/ai/backups/gitea-preupgrade-252-20261007-120948Z`，本地 `sha256sum -c` **8/8 OK**（备份比 252 本身活得久）。
  - **升级 ✓**：28.1.0 镜像 docker save→ssh→load 推送；compose `1.27.3→28.1.0`（.bak 保留）；`VERSION={"version":"28.1.0"}` 硬校验通过。
  - **锁预检 ✓**：`/data`+`git` 双路径 `find` → `NO STALE LOCKS`（整机重启自带清锁，250 同根因隐患不存在）。
  - **AC 矩阵 ✓**：AC1/AC4 selftest **PASS-EXACT**（PR #5085：merge 200 + merged=True + `merge_sha 23534687bbbc` == ref tip，exit=0）；AC3a 不存在 PR → **404**；AC2 PATCH refs → **405**；selftest 探测残留 `[]`、PR 已删。
  - 最终容器：`openclaw-gitea-1` gitea/gitea:28.1.0 Up、`openclaw-postgres-1` postgres:17-alpine Up、`openclaw-runner-1` Up。
- **连接参数实测教训**（已写入脚本头）：OS sshd = **22**（`openclaw@192.168.0.252` + 默认 `id_ed25519` 直通）；**222 = Gitea git-shell**（openclaw 作为 Gitea 用户无匹配密钥→denied，known_hosts 的 `:222` 条目是历史 git 操作所留，此前据此误判为 OS 端口）。
- 失联期诊断矩阵（14 轮，全部失败——当时确证为物理宕机非软件问题，作为故障记录保留）：

| 排查路径 | 结果 |
|---|---|
| 主路径 192.168.0.252（ssh config **全文**复核：三个别名 `gitea-macmini`/`gitea-devstack`/`gitea-z6g4` 均 `192.168.0.252:222`，无隐藏端口/IP） | ARP `INCOMPLETE`/`FAILED`，端口 22/222/3000 全 closed |
| 换 DHCP IP（全 192.168.0.0/24 ICMP 扫描 + 逐活主机探 `/api/v1/version`） | 网段内无任何 Gitea 实例 |
| **子网级 TCP 扫描（无视 ICMP 过滤策略）**：254 IP × {3000, 222} + 254 IP × 22 | 3000/222/22 除本机 250 外**零监听**（方法自证：本机 250:3000 正确抓出） |
| **链路本地回退**：169.254.0.0/16 全段 65536 地址（`ping -I eno1` 绕过 virbr0 路由，65s 完成） | **零存活**——"DHCP 失效自分配 169.254" 假设证伪 |
| **按名发现**：LLMNR (UDP 5355，4 个主机名 × 2 广播) / NBNS (137) / mDNS | 均无应答 |
| **L2 邻居表快照**（全段 TCP 扫刚触发全量 ARP 解析） | `.252 = INCOMPLETE`；约 30 台活机（.1/.9/.10/.12/.48/.72/.73/.220-.243）**均无 gitea/ssh 服务** |
| 备用路由 192.168.3.0/24（静态路由 via 192.168.0.9，history 有 `ssh liying@192.168.3.6`） | 跳板 .9 转发但目的网段黑洞，3.6 及全段无响应 |
| IPv6 | 无邻居条目，未配置 |
| 隐蔽通道：本机 VM (virsh 空)、docker remote context（仅 local）、编排/tea 配置、VPN/隧道客户端（tailscale/wg/frp/ngrok 均未装）、SNMP (.1/.9 161/UDP 手搓 v2c GET 超时)、广播 ping | 全部排除 |
| Wake-on-LAN | 本仓全域 rg、reports/backups/ssh、bash/zsh history、NM leases、OpenCode 会话、全用户 home、`backup-from-252.sh`/MANIFEST——**均无 252 MAC**；网关 SNMP 不可用无法反查 |
| 本机网络自检 | 网关 .1 与邻机 ping OK——排除本机故障 |

- 后台 watcher **pid 273071**（12h × 60s，双条件：ping + version API）于 **12:05:31Z 首次 REACHABLE（attempt 100）、12:06:32Z 写 `GITEA_UP — READY FOR BACKUP+UPGRADE` 后按设计退出**（失联期 log 0 行即"从未可达"的证据）。
- 补充：`docs/governance/REMOTE_LIMITS.md` 将 252 标注为 "Auto-restart, self-healing"，但其自 09:46Z 失联已超 1 小时仍未自愈——故障层级超出进程/容器自愈范围（电源/网口/交换机端口级），需机房人工介入（12:05Z 实际由人工恢复）。
- MAC 穷举结果（WOL 前置）：本仓全域 rg、reports/backups/ssh、bash/zsh history、NetworkManager leases、OpenCode 历史会话、本机全部用户（ai/liying/openclaw/www，userroot/root 不可读）——**均无 252 MAC**。

一键脚本实跑记录（2026-10-07 12:09–12:12Z，本报告第 7 节顶部为实跑结果）：

```bash
bash /tmp/opencode/252_upgrade_prep.sh
# 连接: ssh openclaw@192.168.0.252（OS sshd=22 + 默认 id_ed25519, 恢复后实测; 222 是 Gitea git-shell）
# 实跑: [0] 自检 1.27.3 ✓ → [1] 备份 910M + 拉回 250 (8/8 checksum) ✓ → [2] 推镜像 ✓
#      → [3] TAG REWRITTEN + VERSION=28.1.0 ✓ → [4] NO STALE LOCKS ✓（heredoc 修复转义后补跑）
#      → [5] AC 矩阵: selftest PASS-EXACT + AC3a 404 + AC2 405 ✓, 残留 [] ✓
```

恢复监测三层全部按设计触发：12h watcher（GITEA_UP 后退出）→ 永久 cron `*/5`（**12:10:02Z `CRON_STATE_CHANGE down -> up GITEA_UP`**，state=up，状态变迁才记录、不自动执行升级）→ 会话内复检（12:05:41Z 首个 UP 探测）。

## 8. 操作时间线（UTC）

| 时间 | 事件 |
|---|---|
| 09:25 | 250 备份开始（目录时间戳 20261007-092538Z） |
| 09:46 | 252 失联被发现；watcher 启动 |
| ~09:50 | 250 升级 28.1.0 完成（version API 实证） |
| 09:55-10:04 | PR 创建 500 排查：trace 定位 commit-graph 锁 |
| 10:05 | 清锁，POST /pulls → 201 |
| 10:07 | 首轮 selftest PASS-EXACT（c1d2b359988f） |
| 10:08-10:12 | AC2/AC3 场景验证（404/405/200+405 矩阵齐） |
| 10:13 | 去调试 env 重建容器（env diff 零差异） |
| 10:14 | 干净容器二次 selftest PASS-EXACT（2c2740c8af6d） |
| 10:46-10:58 | 252 深度诊断第 7-14 轮（169.254 全段/子网 TCP/LLMNR/SNMP/VPN/VM/L2 邻居表），矩阵见第 7 节 |
| 12:05 | 252 物理恢复（失联 2h19m）；ping 0% 丢包；OS sshd=22 + id_ed25519 认证实测 OK |
| 12:06 | watcher 写 `GITEA_UP` 后退出（attempt 101） |
| 12:09-12:12 | 一键脚本实跑：备份 910M 拉回 250（8/8）→ 推镜像 → 升级 28.1.0 → NO STALE LOCKS → AC 矩阵全绿（selftest PASS-EXACT + 404 + 405，零残留） |
| 12:10 | cron 检测 `CRON_STATE_CHANGE down -> up`（三层监测全部触发） |

## 9. 风险与回滚

- 250 回滚：`docker rm gitea-test && docker rename gitea-test-old-1.27.3 gitea-test && docker start gitea-test`（数据卷共享，秒级）；DB 级回滚用 `gitea-db.pgdump`。
- 250 当前状态健康：version=28.1.0、备份 checksum 6/6（exit=0）、AC 矩阵绿、探测分支/PR 零残留。
- 252 回滚：compose 已留 `.bak`（`image: gitea/gitea:1.27.3`），`cp .bak 回去 + up -d` 即回滚；卷级备份 `/home/openclaw/gitea-preupgrade-252-20261007-120948Z`（910M）+ 250 侧同名存档双份，CHECKSUM 8/8。
- 252 当前状态健康：version=28.1.0、备份双端、AC 矩阵绿（PR #5085/自测分支零残留）、postgres:17 + runner 容器均 Up。
