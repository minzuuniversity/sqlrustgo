# P0-5 幽灵执行点处置取证（2026-11-05）

> 口径声明：本文所有数字都是本机实测或本仓生成物重放，不含 `ci_gate.sh`，
> 因此**不构成门禁通过声明**。每条数字后面都给了复现命令。

## 1. 实测到的三组数字

| 项 | 数量 | 复现 |
|---|---|---|
| `.github/workflows/*.yml`（GitHub Actions，本仓永不运行） | 13 | `ls .github/workflows/*.yml \\| wc -l`（同 `gov_probe.py metrics` 的 `gh_wf`） |
| `.gitea/workflows/*.yml`（Gitea Actions 实际读取） | 8 | `ls .gitea/workflows/*.yml \\| wc -l`（`gt_wf`） |
| 其中**真的调用门禁脚本**的 workflow | 3 | `python3 scripts/gate/gate_executability.py --json` 取 `roots` 的调用点前缀 |

3 个真实调用点所在 workflow：`ci.yml`、`gate.yml`、`ci-trusted-baseline.yml`。
其余 5 个 `.gitea` workflow 不含门禁调用（soak / 探针 / cell-diff / reconciliation）。

## 2. 幽灵门禁的成因（可复算，不是我的解释）

`gov_probe.py metrics` 里 `phantom=4` 的定义就是
`(只被 .github 引用的门禁) - (活执行点根集合)`（`gov_probe.py:150`）。
按该定义现场重算，命中集合与 metrics 输出的 `phantom_list` 完全一致：

| 门禁 | 只出现在 | 脚本是否存在 |
|---|---|---|
| `check_alpha_v380.sh` | `.github/workflows/r-gate.yml` | 存在 |
| `check_cross_version_debt.sh` | `.github/workflows/r-gate.yml` | 存在 |
| `check_no_log_tbl_json.sh` | `.github/workflows/gate-no-log-tbl-json.yml` | 存在 |
| `check_sql_corpus_gate.sh` | `.github/workflows/ci-corpus.yml` | 存在 |

复现：对两个目录逐文件剥离注释行后匹配 `check_*.(sh|py)`，与 `.gitea` 侧求差集；
再读 `runs/<id>/logs` 里 `gov_probe metrics` 的 `phantom_list` 对照。

⇒ 这 4 道「CI 已覆盖」的说法全部不成立：引用它们的文件本身永不运行。
`metrics` 的 `phantom` 计数**未改动**（改它会等于把分母搬走迁就现实）。

## 3. 本轮处置（全部机器可核对）

1. **根集合显式声明**：新增 `docs/releases/v4.1.0/CI_EXECUTION_ROOTS.jsonl`
   （声明头 + 21 条文件行：13 archive-only / 8 live）。此前扫描根只存在于
   `gate_executability.py` 的隐式 glob 里，无人能核对"声明的"与"实际扫的"是否同一批文件。
2. **存档横幅**：13 个 `.github/workflows/*.yml` 顶部加
   `ARCHIVE-ONLY — 仅存档、不执行`（含"引用它作为 CI 已覆盖的证据无效"）。
3. **假 workflow 迁出**：.gitea/workflows/ci-test.md（1 行 markdown，冒充 workflow，
   任何引擎都不会读它）迁到 `docs/ci/legacy/ci-test-2026-05-01.md`，
   并同步更新 `docs/governance/CI_GATE_CONTRACT.md`、`docs/agent-harness/REPOSITORY_MAP.md` 的引用。
4. **soak 归一**：`soak_168h.yml` 标为 canonical，`z6g4-168h-soak.yml`（v3.12 期 #4499 遗留）
   标为 superseded，`soak_probe.yml` 明确其角色是 runner 存活探针而非 SOAK 配置。
   顺带修一处实际漂移：canonical 的 `workflow_dispatch.branch` 默认值仍写 `develop/v3.9.0`，
   已改为 `develop/v4.1.0`（只是输入默认值，不等于 SOAK 跑过）。
5. **核对器**：`audit/v4.1.0/probe_ci_roots.py`（R1–R8，含与 `gate_executability --json`
   实际调用点的耦合 R7：声明若与门禁真正扫的文件脱钩，声明就只是装饰）。

## 4. 反向变异（AGENTS 2.5）

`python3 audit/v4.1.0/probe_ci_roots.py mutation-selftest` ⇒ `ok=8 bad=0`：

| 臂 | 注入 | 必须命中 |
|---|---|---|
| CTRL | 不注入 | rc=0 |
| M1 | 磁盘上加一个未声明 workflow | R3 |
| M2 | 声明里留一个已删文件 | R2 |
| M3 | 摘掉 archive-only 文件的横幅 | R4 |
| M4 | 往执行目录塞 `.md` 冒充 workflow | R5 |
| M5 | 把门禁真正调用的 workflow 改判成 archive-only | R8 |
| M6 | 删掉一条声明行 | R3 |
| M7 | 把 `soak_canonical` 指向被取代的那份 | R6 |

每臂都断言"必须命中指定判据"，不接受"随便红一下"。
写第一版时 M7 因为复用被 M5 改过的声明对象而命中了 R8（原因不对），已改为按臂重读声明。

## 5. 没处置的部分（诚实列出）

- 那 4 道 phantom 门禁**现在依然没有执行点**。要么在 `.gitea` 侧补执行点，
  要么把它们从阶段清单里摘掉 —— 后者属于门禁清单变更，按 AGENTS 2.1 只能 owner 在独立 PR 做。
- `scripts/gate/probe_hard_gate_mutation.sh` 的默认参数是
  `.github/workflows/ci-pr.yml`（archive-only），它断言的 `fmt/clippy/docs/build/...`
  硬门禁是 GitHub 镜像的 job 集合，不是这里真正会跑的集合。改默认目标会改变该脚本的判定对象，
  属 AGENTS 3.2，且需与 CI 实际 job 名对齐后才有意义 ⇒ 留给 owner 决定（见 ISSUE #5597 的 CI 执行面）。
- 168h SOAK 仍未真跑（看板 ING-1），canonical 文件存在不等于有证据。
- 本轮不能取 CI 侧证据：runner-03 缺 `sqlrustgo-runner:v1`（看板 IL-9 / ISSUE #5597）。

## 6. 看板状态

P0-5 ⇒ **待复核**：文件级处置与声明闭环已成立（probe check rc=0、selftest 8/8），
但"4 道 phantom 门禁的执行点归属"与"CI 侧复核"两件事仍未闭环，单血统取证不足以升完成。
