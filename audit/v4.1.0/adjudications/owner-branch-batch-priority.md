待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**待决**：剩余 10 条「需要专门批次」的分支，**先做哪几条**（工作量的先后顺序），
以及 FL-1 那条尚未定位的第二支失败模式该往哪个方向查。

**材料**：
- `docs/releases/v4.1.0/BRANCH_MERGE_VERDICTS_2026-11-06.md`（365 个分支的逐条
  裁定；§2.5「精判」是本轮逐个打开后的具体判定与依据）。
- 10 条待专门批次的清单与各自理由：见 §2.5 表格（session-context-5057 /
  5262-b2-binaries / omp-ignore-registry-5098 / ci-develop-baseline-red-6335 /
  gate-paths-batch / backup-before-force-… / ci-3847-d6b-no-reexec 等）。
- 归档 run `20261106-branchbatch-001`（本轮已取走 F-08 列查找 + anti-fabrication
  三态自检 + 证据文档；剩余分支未动）。
- FL-1 的现状：真因（idle-reaper 在语句执行期间掐断连接）已修（PR #5614），
  修前 20 轮 2 红 → 修后 42 轮全绿；但**另有一支失败模式**：服务端日志里没有任何
  错误/告警、连接却提前断（修前 20 轮里出现过 1 次）。
- CI 现状：push 与 PR 都会建 run，runner 1735/1736/1737 均 online，但每个 run
  都是 `completed/failure` 且 **job 数 = 0** ⇒ workflow 在建 job 之前就失败，
  **没有任何门禁真正执行过**（同样形态 2026-11-05 的 run 10884/10885 就存在）。
- 看板行的阻塞分布：`阻塞 17`（含 ENG-13/#5584、EX-10/EX-5 席位、IL-9 CI 面、
  P0-3 分支保护、HO-2 附录、LEDGER-2/KAN-2 口径）。

**要裁的**：
  (1) 10 条分支专门批次的**优先级排序**（前三条 + 理由），并说明哪些**不该做**
      （例如含 `EXP-DISABLED` 实验步骤、或 CI 侧当前无法验证的）。
  (2) 既然 CI 执行面不可用，CI 类分支（`ci-3847-d6b-no-reexec`、
      `fix/ci-develop-baseline-red-6335`）现在做还是等 CI 恢复？
  (3) FL-1 第二支失败模式的排查方向（候选：客户端 LOCAL INFILE 发送与服务端读包
      边界的竞态 / 服务端主动关闭连接 / socket 缓冲与超时）。

**判据**：给出排序与取舍，并指出每条的**可验收判据**（要有机器可判的出口）。
材料不足 ⇒ INCOMPLETE。
