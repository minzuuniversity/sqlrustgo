# CI 执行面不可用（2026-11-05 实测）：runner-03 缺本地镜像

> 结论口径：这是一份**报告**，不是修复。CI 起不来时，任何"CI 已核对"的说法都必须写成
> **没能核对**（INCOMPLETE），不得当成通过。

## 现象

PR #5593 推 `1095a68ab6` 后，两个 workflow 的全部 8 个 job 都在**同一条错误**上终止，
没有任何一个 job 真正执行过仓库里的命令：

| run | workflow ref | job | 结果 |
|---|---|---|---|
| 10830 | `ci.yml@refs/pull/5593/head` | lint-build / test / test-crates / mutation / ga3-security-scan(skipped) / test-soak-detach / stage-gates(skipped) / **postcheck** | 除 2 个 skipped 外全部 failure |
| 10831 | `gate.yml@refs/pull/5593/head` | evidence-graph-gate / legacy-gate | failure |

关键日志（job 24575 = postcheck，job 24576/24577 = gate.yml 两个 job，全文只有 10 行）：

```
sqlrustgo-runner-03(version:v0.6.1+1-g8e6b3be9) received task ... of job postcheck
  🚀  Start image=sqlrustgo-runner:v1
  🚀  docker pull image=sqlrustgo-runner:v1 platform= username= forcePull=false
Image exists? false
pulling image 'docker.io/library/sqlrustgo-runner:v1'
Error response from daemon: pull access denied for sqlrustgo-runner,
repository does not exist or may require 'docker login': denied
```

## 判据：不是 workflow 写错，是 runner 侧缺镜像

同一台机器族（labels 完全相同：`hp-z6g4, ubuntu-latest, self-hosted, linux`）上的
**对照实验**——上一次成功的 run 10015（2026-10-25，`gate.yml@refs/heads/develop/v4.1.0`）
的两个 job 落在 runner-04 与 runner-02，两个 job 都 success：

```
sqlrustgo-runner-04 ... job evidence-graph-gate   -> job 21540 success
sqlrustgo-runner-02 ... job legacy-gate           -> job 21541 success
```

Gitea 当前在线 runner：`sqlrustgo-runner-02 / 03 / 04`（id 1735/1736/1737，全 online），
`sqlrustgo-ci-01`（id 1729）offline。⇒ 差异只在**具体被派到哪个 runner**：
runner-03 上本地不存在 `sqlrustgo-runner:v1`，而该 tag 只存在于内网构建、
不在 docker.io，因此 runner-03 回退到 docker.io 拉取并被拒绝。

## 这条故障切断了哪些声明

`postcheck` job 里挂着本轮新加的三条自测（看板 KR-2 看板台账一致性、
KR-3 门禁登记表一致性、P0-2 三态夹具 self-test）。CI 起不来 ⇒

- **KR-2 / KR-3 不能因为"已经接线"就判完成**：接线证据必须是某个 Actions run 的 job 日志，
  现在拿不到，只能保持 **待复核**，并把"没能核对"写进 evidence。
- 看板 IL-7（把三态 runner 真挂到阶段清单）同样被这条故障挡住：挂载点可以改，
  但没有一个能跑起来的 job 去承载它。
- 本轮所有结论的证据面因此只剩 **仓内 run 归档**（`audit/v4.1.0/runs/20261105-tristate-001`）。
  它不含 `ci_gate.sh`，不构成门禁 PASS 声明。

## 需要人做什么（owner / hermes）

1. 在 runner-03 上把本地镜像 `sqlrustgo-runner:v1` 补齐（与 runner-02/04 同一构建），
   或把 runner-03 下线/摘 label，直到镜像就位。
2. 顺带核对：本仓 CI 历史 1358 次 run 里只有 39 次 success（2.87%）。
   本轮至少这一类失败是**基础设施**造成的，与提交内容无关——
   分母里混着这类噪声时，"CI 红"不能作为质量结论，只能作为待查项。

> 我不在这台机器上补镜像：runner 主机与镜像构建属 252/hermes 正在改的基础设施面，
> 并发改动会互相踩。按 AGENTS 的停止条件，这类越界动作的正确做法是报告给人。
