# 审计记录完整性（2026-11-05 实测）

> 一句话：**仓里 17 条审计记录，只有 10 条能被机器复核；6 条在结构上不可能被复核，1 条连清单都没有。**
> 这一份就是"报告都说好、一测就露馅"的机器证据。

## 1. 怎么量的

`python3 audit/v4.1.0/audit_integrity_report.py`（度量工具，不是门禁：不叫 `check_*`、
不进 STAGE_CONFIG 阶段清单，退出码恒 0）。它对每条记录摊开五个数：

| 字段 | 含义 |
|---|---|
| `declared_cmds` | manifest 声称执行过几条命令 |
| `cmd_recorded` | 其中**记下了命令行**的有几条 |
| `logs` / `metas` | 日志文件 / 命令元数据 是否在场 |
| `sealed` | 清单自哈希（`manifest_sha256` 或 `evidence_hash`）是否已封板 |

## 2. 结果（同一次运行，可复现）

| run_id | 声称 | 记了命令行 | 日志 | meta | 封板 | 判定 |
|---|---|---|---|---|---|---|
| iaia410-5112-evidence-20261015 | 6 | **0** | **0** | 0 | 是 | hollow |
| iaia410-a01-20261011T062229Z | 7 | **0** | 21 | 7 | 是 | hollow |
| iaia410-a02-20261011T070228Z | 8 | **0** | 16 | 8 | 是 | hollow |
| iaia410-a03-20261011T070845Z | **0** | 0 | 0 | 5(孤儿) | **否** | partial |
| iaia410-a04-20261011T180837Z | 5 | **0** | 15 | 5 | 是 | hollow |
| iaia410-a05-20261012T004948Z | 9 | **0** | 27 | **0** | 是 | hollow |
| iaia410-a06-r1-20261011T073325Z | — | — | — | — | — | **没有 manifest.yaml** |
| 20261009-a01-001 … 20261105-p07-failclosed-002（10 条） | 44 | 44 | 44 | n/a | 是 | sound |

合计：`hollow 5 / partial 1 / no-manifest 1 / sound 10`（记录总数 17）。

## 3. 三类缺陷的形状完全不同

1. **hollow（5 条，全部是 2026-10-11~10-15 的 iaia410 记录）**
   manifest 逐条写着 `exit_code: 0`，但**没有一条记下它跑的是什么命令**。
   日志的 sha256 都对得上，可对得上的是"某堆字节的散列"，不是"某个动作发生过"。
   最极端的是 `iaia410-5112-evidence-20261015`：声称 6 条命令 exit 0，
   `commands/` 目录根本不存在，`SHA256SUMS` 第一行是字面的 `SHA256SUMS placeholder`。
   **这类记录无法修复**：没有命令行就没有复现路径，历史输出也不可从记录重建。
2. **partial（1 条）** `iaia410-a03`：脚手架建了没封板（`manifest_sha256: None`），
   `commands: []`，但目录里躺着 5 个孤儿 meta。
3. **no-manifest（1 条）** `iaia410-a06-r1`：目录里只有 `findings/`，没有 manifest。

## 4. 本轮把两道既有门禁收紧（AGENTS 3.2：改脚本，不改清单）

判据不是新增门禁，而是挂到**既有**门禁 `check_audit_manifest.sh` / `audit_run.py verify` 上：

* **必须记录命令行**：`verify` 现在要求每条被声称的命令都留下 `cmd`（或同义键 `command`），
  否则报 `N/M 条命令没有记录命令行 ⇒ exit_code 声明不可复核`；`commands: []` 同样直接红。
* **运行目录必须有清单**：原先 `check_audit_manifest.sh` 只 glob `*/manifest.yaml`，
  "有目录、没清单"被**静默跳过**（`iaia410-a06-r1` 从来没报过红）。
  现在按命名约定（`iaia410-*`、`YYYYMMDD-*`）点名。

**改前会漏掉什么**：4 条真实缺陷（a01/a02/a04 的"无命令行"、a06 的"无清单"）被当成没问题。
**改后会多报什么**：门禁从 3 红变成 **7 红**（10 条 sound 记录不受影响，仍 verify OK）。

## 5. 反向变异（夹具 `scripts/gate/test_audit_manifest_integrity.py`，6 臂）

夹具自己造一条**合法的** `iaia410-manifest/v1` 记录（v2 的写入端早已不在仓里），
散列用被测工具自己的 `manifest_sha256_of()` 算，避免夹具与实现各写一套口径。

| 臂 | 内容 | 期望 | 实测 |
|---|---|---|---|
| CTRL | 健康 v2 记录 | 绿 | 绿 |
| M1 | 抹掉命令行 `cmd` | 红 | 红 |
| M2 | 换成同义键 `command` | 绿（判据盯"有没有命令行"，不是拼写） | 绿 |
| M3 | `commands: []` | 红 | 红 |
| M4 | 删掉一条日志（既有判据仍承重） | 红 | 红 |
| M5 | 有目录无清单 | 门禁必须点名 | 点名 |
| M6 | 空仓控制组 | 绿（不许凭空报错） | 绿 |

## 6. 需要人做的决定（Agent 不得自行处理）

6 条 iaia410 历史记录 + 1 个无清单目录 = **7 条不可复核的历史记录**。
可选处置只有三种，全部是 owner 动作：

1. **退役删除**（走独立 PR，Agent 不得发起 —— 删除审计记录是人的专属动作）；
2. **保留但显式标注为不可复核**，并在所有引用它们的文档里撤回结论；
3. **重做**：由 owner 指定要恢复哪些工作包的证据，我们按现 schema 重新封板。

⇒ Gitea ISSUE 见看板行 IL-5（`docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md`）。

## 7. 复现命令

```bash
python3 audit/v4.1.0/audit_integrity_report.py          # 上面的表
bash scripts/gate/check_audit_manifest.sh               # 现场状态（当前 rc=1，7 条红）
python3 scripts/gate/test_audit_manifest_integrity.py   # 6 条反向变异
```

## 已知自指（看板 IL-6）

`check_audit_manifest.sh` 按目录扫描 `audit/v4.1.0/runs/`，**不区分"已归档的记录"与"正在写入的记录"**。
后果：一条 run 正在执行时，它自己的运行目录已经在 `runs/` 里，但 `manifest.yaml` 的
exit_code 与日志哈希还没写（`run` 只在全部命令跑完后落盘），于是门禁在 run 进行中把该目录
也判红，红数从 7 变 8；同一条命令在 run 结束后重跑又回到 7。

实测复现（同一条命令，run 内 rc=1 / run 后 rc=0）：

```bash
bash scripts/gate/check_audit_manifest.sh 2>&1 | grep -c '^FAIL:'   # run 内 8, run 后 7
```

本轮没有改门禁去规避它（改法需要反向变异证据，见看板 IL-6 的验收），而是把 run 20261105-auditintegrity-002
的计数探针改成只数 `iaia410-` 前缀的历史记录，使自指不污染断言。
第一版 run（20261105-auditintegrity-001）因此 FAIL(1)，取证留在 /tmp/srg-forensic/，未提交进仓。
