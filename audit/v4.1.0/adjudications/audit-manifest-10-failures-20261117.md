# `check_audit_manifest.sh` 的 10 条失败 —— 逐条归因与处置建议

**日期**：2026-11-17　**取证树**：`develop/v4.1.0` @ `2cd1a0c7f503378654e9b04ffc7505264086f9db`
（交付树 `/Volumes/workspace/dev/srg-gate-d1`；该树已含权威轮 013 的 29 项落账，
未含 012 归档 —— 012 的归档在 013 之后才合入，故本表里没有 `20261117-ga-gates-012`。）

**命令**：`bash scripts/gate/check_audit_manifest.sh`
**rc = 1**；**verify OK = 120，FAIL = 10**。

枚举范围是**已提交的树**（`git ls-tree HEAD`），不是工作区目录 ——
这是该门禁在 `GATE_CATALOG.jsonl` 里自己声明的 `evidence` 口径。

---

## 0. 结论（先说结果）

1. 这 10 条**不是同一种病**，是 **5 类**。
2. 其中 **9 条是「历史归档本身不可复核/无内容」**（2026-10/11 的老归档）——
   对它们的正确处置是**登记为不可复核并从结论引用中排除**，**不是**补造内容。
3. **只有 1 条（#8）是可精确修复的元数据漂移**，且已**实证**到唯一成因：
   目录搬迁时把 manifest 的 `run_id` 改了一行，而 `evidence_hash` 的规范载荷
   **包含 `run_id`** ⇒ 自身哈希必然失效。命令、日志、artifacts 及其哈希
   **可证未变**（见 §3 的复算）。
4. 门禁登记里这条 note **已陈旧**：写的是「7 条历史记录不可复核（5 hollow /
   1 unsealed / 1 无清单目录）」，今日实测是 **10 条** —— 多出的 3 条 =
   2 条「空记录」+ 1 条「搬迁漂移」（见 §4）。
5. 本门禁 `class=orphan_referenced` / `owner_task=P0-2` /
   `disposition=needs-execution-point` ⇒ 它**不在 CI、不在 GA 35 项**，
   所以今天不挡任何流水线；但一旦按授权把它挂上执行点，这 10 条会**立刻变红**
   ⇒ 这正是「先处置、再挂执行点」的原因。

---

## 1. 10 条逐条清单（实测原文口径）

| # | 归档 | 原因（门禁原文） | 归类 |
|---|---|---|---|
| 1 | `iaia410-a06-r1-20261011T073325Z-01df92a5` | `run dir without manifest` | **C 缺清单目录** |
| 2 | `iaia410-5112-evidence-20261015` | 6/6 条命令**没有记录命令行** (ids: 1,2,3,4,5,6) | **A 命令行缺失** |
| 3 | `iaia410-a01-20261011T062229Z-609a8e81` | 7/7 条命令没有记录命令行 (001–007) | **A** |
| 4 | `iaia410-a02-20261011T070228Z-609a8e81` | 8/8 条命令没有记录命令行 (001–008) | **A** |
| 5 | `iaia410-a03-20261011T070845Z-omp` | `commands 为空` ⇒ 空记录 | **B 空记录** |
| 6 | `iaia410-a04-20261011T180837Z-609a8e81` | 5/5 条命令没有记录命令行 (001–005) | **A** |
| 7 | `iaia410-a05-20261012T004948Z-bf48818c` | 9/9 条命令没有记录命令行 (ids 1,2,3,4,5,6,7,008) | **A** |
| 8 | `20261105-lb1-006-mcode` | `evidence_hash mismatch`：manifest=`sha256:74a3d0c5…` / recomputed=`sha256:f54c6130…` | **E 哈希漂移（可精确修复）** |
| 9 | `20261112-ab08-core3` | `commands 为空` ⇒ 空记录 | **B** |
| 10 | `20261112-soak-ab003` | `manifest has no results/evidence_hash; run it first` | **D 未封存** |

A 类 5 条（#2,#3,#4,#6,#7）／B 类 2 条（#5,#9）／C 类 1 条（#1）／
D 类 1 条（#10）／E 类 1 条（#8）＝ **10**。

---

## 2. 逐类处置建议

> 处置原则（外部专家已定，见 `audit/v4.1.0/adjudications/expert-review-20261117.md`
> 第 4 条）：**红门禁应阻断 GA，而不是通过断开执行入口来维持绿色。**
> ⇒ 这 10 条要**如实处置**，不能靠「不挂执行点」把红藏起来。

| 类 | 条数 | 性质 | 建议处置 | 明确**不做**的事 |
|---|---|---|---|---|
| **A 命令行缺失** | 5 | `exit_code` 声明**不可复核**（命令文本没记下来） | 登记为「历史不可复核」，并**逐条登记**在门禁处置台账；任何结论**不得引用**这 5 个归档 | **绝不补造命令行** —— 那是伪造证据（AFP） |
| **B 空记录** | 2 | `commands` 为空，**没有任何可复核内容** | 登记为「空记录，任何结论不该引用它」（验证器原文即如此措辞）；从引用面排除 | 不删目录（会动归档面）、不补内容 |
| **C 缺清单目录** | 1 | 目录有文件但**无 manifest** | 补 manifest ＝**新造归档**（不是修复）⇒ 须属主裁定；否则登记处置 | 不新造归档 |
| **D 未封存** | 1 | 有 manifest 但无 `results`/`evidence_hash` | 同 B：登记处置、从引用面排除 | 不补 results（补出来的不是当时跑出来的） |
| **E 哈希漂移** | 1 | 证据**内容完好**，只有 `run_id` 行被搬迁改动 | **可精确修复**：在当前 manifest 上**重算** `evidence_hash` 并落库，同时在 `RELOCATION_NOTE.md` 追加一行说明（重算依据 + 原值 + 新值） | 不改任何 stdout/stderr/commands/artifacts；不删除归档 |

**E 类为什么可精确修复 —— 实证**（`audit/v4.1.0/audit_run.py` 的
`evidence_hash_of()`，第 58–80 行）：

```python
canonical = json.dumps({
    "schema_version": manifest["schema_version"],
    "run_id":        manifest["run_id"],      # ← 包含 run_id
    "frozen_commit": manifest["frozen_commit"],
    "artifacts":     manifest["artifacts"],
    "results":       results,
}, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
```

而该归档目录里的 `RELOCATION_NOTE.md` 写明：内容是从
`gitea252/mcode-macmini/governance-kanban-expert-ladder` **逐文件搬来**的原文，
**只改了 manifest 的 `run_id` 一行**以与目录名一致；命令、stdout/stderr、
evidence_hash **未改动**。

复算（本轮实测，两次现算）：

| 输入 | 重算出的 evidence_hash | 与 manifest 内记录 |
|---|---|---|
| 当前 manifest（`run_id=20261105-lb1-006-mcode`） | `sha256:f54c613031dd67da5…` | ✗ 不匹配 |
| **仅把 `run_id` 还原为搬迁前的 `20261105-lb1-006`** | `sha256:74a3d0c5368ff856…` | ✓ **完全相等** |

⇒ 唯一漂移就是那一行 `run_id`；`commands` / `stdout_sha256` / `stderr_sha256` /
`artifacts` **逐字节未变**。修复因此是**纯元数据重算**，不是「重跑一遍」，
也不需要碰任何日志。

---

## 3. 登记口径已陈旧（须一并修）

`docs/releases/v4.1.0/GATE_CATALOG.jsonl` 里 `check_audit_manifest.sh` 的 `note` 写：

> 「判据本身可用，但已提交树上 rc=1：**7 条**历史记录不可复核（5 hollow 无命令行 /
> 1 unsealed / 1 无清单目录），**10 条 sound** 不受影响」

今日实测是 **10 条失败 / 120 条 OK**。按成因对账，note 覆盖的是
5（A）+ 1（D 未封存）+ 1（C 无清单目录）＝ 7；**新增的 3 条**是
**2 条「空记录」**（`iaia410-a03-omp`、`20261112-ab08-core3`）与
**1 条「搬迁漂移」**（`20261105-lb1-006-mcode`）。
（note 未逐条列名，上述映射由本次实测的成因行反推；若要改 note，
建议按 §2 的逐条清单改写，别再写聚合计数。）

**结论**：处置这 10 条时，`GATE_CATALOG.jsonl` 的 `note`/`evidence` 必须**同步更新**，
否则登记与实测又会漂（这正是 C2/C3/C5「现场重算必须与登记一致」要防的那类漂移，
只不过这次漂的是 `note` 这种人工字段）。

---

## 4. 归属与阻塞

- 门禁：`scripts/gate/check_audit_manifest.sh`，
  `class=orphan_referenced` / `owner_task=P0-2` / `disposition=needs-execution-point`
  / `can_fail=true` / `neg_fixture=true`
- 它的 `effect` 声明是**诚实的**：`catches: []`，理由「尚无可证伪夹具 … 无声明的
  『能抓』比错的『能抓』更坏」⇒ 本轮**不**给它补「能抓」声明。
- **阻塞**：按授权把它挂上执行点之前，这 10 条必须先处置完 —— 否则 CI/GA 会红。
  这与 P0-4「61 道死门禁 + 83 个孤儿门禁逐个处置」是**同一批工作**的
  一部分（该门禁 `orphan_referenced` 属于那 83 道）。
- 既有议题：ISSUE 5596 / 看板 IL-5（note 里点名）。

---

## 5. 本轮明确**没有**做的事（避免被误读成「已处置」）

- **没有**改任何门禁脚本（本轮只做只读诊断与归因）。
- **没有**删任何归档目录、**没有**移动归档、**没有**补造命令行 / results /
  manifest —— 那会把「不可复核」伪装成「可复核」。
- **没有**把 `check_audit_manifest.sh` 挂上执行点（要先处置这 10 条）。
- **没有**动门禁分母 168（未新增 `check_*`、未改阈值）。

---

## 6. 复现命令

```bash
cd <交付树>            # git checkout --detach 2cd1a0c7f503378654e9b04ffc7505264086f9db
bash scripts/gate/check_audit_manifest.sh > /tmp/audit_manifest_out.txt 2>&1; echo "rc=$?"
grep -c "verify OK" /tmp/audit_manifest_out.txt   # 120
grep -E "^FAIL"      /tmp/audit_manifest_out.txt   # 10
```

E 类复算（两条对照）：

```python
import copy, importlib.util, yaml
spec = importlib.util.spec_from_file_location("audit_run", "audit/v4.1.0/audit_run.py")
ar = importlib.util.module_from_spec(spec); spec.loader.exec_module(ar)
m = yaml.safe_load(open("audit/v4.1.0/runs/20261105-lb1-006-mcode/manifest.yaml"))
print(ar.evidence_hash_of(m))                                  # sha256:f54c6130…（当前）
o = copy.deepcopy(m); o["run_id"] = "20261105-lb1-006"
print(ar.evidence_hash_of(o))                                  # sha256:74a3d0c5…（= 记录值）
```

---

## 7. 相关材料

- `audit/v4.1.0/adjudications/expert-review-20261117.md` —— 专家意见第 4 条
  （红门禁应阻断 GA，不得断开执行入口维持绿色）
- `audit/v4.1.0/adjudications/p0-4-dead-orphan-gate-survey-20261117.md` ——
  83 道「无执行点」门禁的处置调研（本门禁在其中）
- `docs/releases/v4.1.0/GATE_CATALOG.jsonl` —— 登记 SSOT（本门禁的 note 已陈旧）
- `docs/releases/v4.1.0/DEAD_GATES_CLASSIFICATION_2026-11-08.md` —— 61 道死门禁的三类分档
