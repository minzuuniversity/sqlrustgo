# 批次 34 —— 门禁随机红不是「门禁判定错」，是管道把「找到了」报成「没找到」

> **基线**：`develop/v4.1.0` @ `3cbe678f57ce718f5fed98b03f369942424235f1`
> **provenance:** generated_at=2026-10-24, branch=fix/gate-sigpipe-pipefail-batch34,
> commit=`3cbe678f57ce718f5fed98b03f369942424235f1`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-b34-sigpipe-gate-flake-001`
> （本文件汇编的门禁引擎：`scripts/gate/check_storage_db_scope_coverage.sh`、
> `scripts/gate/check_evidence_binding.sh`、`scripts/gate/check_ci_checkout_consistency.sh`、
> `scripts/gate/verify_pr_dag.sh`、`scripts/gate/check_correctness_tests_production_storage.sh`、
> `scripts/gate/check_stage_config_consistency.sh`、
> `scripts/gate/check_off_production_path_registry.sh`、
> `scripts/gate/check_test_manifest_honesty.sh`；
> 证据绑定门禁 `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。
> ⚠️ **本批全部为本机实测**（Mac-mini-7.local），**不是 CI run** ——
> CI 侧验证待 #5568 合并后补）

---

## 0. 结论

`#5401` 登记的是「门禁**恒红**」。当前 develop 上的实际状态不是恒红，而是
**随机红**：同一份树连续 10 次跑，红 4 次，每次红的**方法名都不同**。

根因不是门禁的判据写错了，而是判据**通过管道提问**：`grep -q` 命中即退出并
关闭管道读端，写端因此吃到 SIGPIPE（141）；脚本开着 `set -o pipefail`，
于是「找到了」被报成「没找到」。这是一类**假失败**：门禁报告了一个不存在的问题。

本文档给出机制、量化、修复、变异证据，以及全仓 sweep 的分类结果。

---

## 1. 现象与定位

`scripts/gate/check_storage_db_scope_coverage.sh`（CI `postcheck` job 的
「Storage DB-scope coverage check (#5192)」步骤）的判定原本是：

```bash
if ! printf '%s\n' "$impl_body" | grep -q "fn $m("; then
    echo "FAIL: $w does not override \`$m\`" >&2
```

实测（修复前，同一份树连续 10 次）：

```
run1 rc=0 fails=0      run6 rc=0 fails=0
run2 rc=0 fails=0      run7 rc=0 fails=0
run3 rc=0 fails=0      run8 rc=1 fails=1      FAIL: wal_storage does not override `has_view`
run4 rc=1 fails=3      run9 rc=1 fails=1      FAIL: wal_storage does not override `insert_in_db`
run5 rc=1 fails=2      run10 rc=1 fails=1
```

红的**方法名每次都不同**，且都是**确实存在**的方法
（`wal_storage` 的 impl 块里 `has_view` / `insert_in_db` / `scan_in_tx_db` 都在）。
这就是「不可复现的红」的全部特征。

---

## 2. 根因

### 2.1 机制

1. `printf '%s\n' "$impl_body"` 把整个 impl 块（约 50 KB）写进管道；
2. `grep -q` **命中即退出**，管道读端被关闭；
3. 写端还没写完，下一次 `write()` 返回 `EPIPE` ⇒ bash 内建 `printf` 吃
   **SIGPIPE** 并以 **141** 退出；
4. `set -o pipefail` 让管道的状态取「最后一个非零的退出码」⇒ 管道状态 = 141；
5. `! 141` 为真 ⇒ 进入 FAIL 分支 ⇒ **「方法存在」被判成「方法缺失」**。

注意第 3 步只发生在**写端还没写完**的时候，所以命中位置越靠前、payload 越大，
命中率越高；这解释了「每次红的方法名都不同」。

### 2.2 量化

对 16 个 db-scope 方法 × 2 个 wrapper × 60 轮 = **1920 次采样**，逐次记录
(a) 管道状态、`(b)` 用无管道参考值（`grep -q <<< "$impl_body"`）给出的真值：

| 指标 | 次数 | 占比 |
|---|---|---|
| 管道返回 141（SIGPIPE） | 32 | 1.67% |
| 管道状态与真值不一致 | 32 | 1.67% |
| **其中「方法其实存在却被判缺失」的假失败** | **32** | **100%** |

即：**所有**不一致都是假失败，没有一次是真判对。

按 1.67%/检查、每轮 32 个检查估算，整门禁每轮的红概率
≈ 1 − (1 − 0.0167)³² ≈ **42%**，与「10 次跑红 4 次」的实测吻合。

### 2.3 为什么这不只是「这台机器的事」

管道缓冲在 Linux 是 64 KB、macOS 是 16 KB，而 `wal_storage.rs` 的 impl 块
是 49,704 字节、49,704 > 16 KB（macOS 必现）、接近 64 KB（Linux 取决于
bash 的 stdio 分块与命中位置）。**结论与平台无关**：这是一个可以被
payload 大小与命中位置同时触发的竞态，CI 镜像上同样可能发生。

---

## 3. 修复

判定改为 `gate_text_contains_literal`（`scripts/gate/common.sh`）：

```bash
gate_text_contains_literal() {
    local text="$1" needle="$2"
    case "$text" in
        *"$needle"*) return 0 ;;
        *) return 1 ;;
    esac
}
```

纯 bash `case`，**无子进程、无管道** ⇒ 没有写端可被打死 ⇒ 不存在竞态。

同一 worktree、同一份树、同口径对照（各 60 轮）：

| 判定形式 | RED / 60 |
|---|---|
| 旧：`printf '%s\n' "$impl_body" \| grep -q "fn $m("` | **23** |
| 新：`gate_text_contains_literal "$impl_body" "fn $m("` | **0** |

---

## 4. 变异证据（#5401 验收标准 3）

> 验收标准原文：「一个新增 `db: &str` trait 方法的变异能被该门禁抓住
> （有变异证据，不是靠人读代码断言）」。

新增 `--selftest`：把门禁脚本与 storage 源码**复制到临时目录**（工作树全程
不被改），在 trait 上注入一个没人覆盖的 `db: &str` 方法。

| 臂 | 注入 | 期望 | 实测 |
|---|---|---|---|
| A 基线 | 无 | 绿 | 绿 |
| B 变异 | trait 加 `gate_selftest_probe(&self, db: &str)`，两个 wrapper 都不覆盖 | 红，且**点名该方法** | 红且点名 ✓ |
| C 补救 | 同一方法，两个 wrapper 都覆盖 | 重新变绿 | 绿 ✓ |

**C 臂是 B 臂有意义的前提**：一个无论什么原因都红的门禁同样能「抓」变异，
但它也会让 C 臂失败。

实测输出：

```
SELFTEST negative control: pipe-based form mis-answered 200/200 times (0 = the bug did not reproduce here)
SELFTEST: 0 failure(s)
SELFTEST: PASS (baseline green, mutation caught, remediation green, text predicates stable)
```

负控制（旧的管道写法）在 2 MiB payload 上 **200/200 次全部答错**；新写法在
同样 payload 上 800 次断言全部正确。

---

## 5. 全仓 sweep 与分类

### 5.1 口径

`sweep` 找出所有满足三个条件的站点：

1. 脚本开启了 `set -o pipefail`；
2. 管道末端是 `grep` 且选项里带 `q`（`-q` / `-qE` / `-qF` …）；
3. 该 `grep` 是**管道的一部分**（`||` 里的 `grep` 不算）。

全仓 **171 处**，其中 CI workflow 会调用的脚本里 **50 处**。

### 5.2 判定真风险的标准

只有当**写端的 payload 能超过管道缓冲**、且 `grep -q` 提前退出时，SIGPIPE
才可能发生。按 payload 逐处量：

| payload 形态 | 站点数 | 风险 |
|---|---|---|
| 整个文件 / 命令输出（cargo、脚本全量输出） | 3 | **真风险** |
| 单行（`read` 出来的一行） | 16 | 无（写端一次写完） |
| 代码片段 / 固定小串 / 小数组 | 31 | 无 |
| `grep` 直接读文件参数（`grep -q PAT "$FILE"`，无管道） | 2 | 无（误报） |
| `wc -l \| grep -q`（`wc` 先消费完上游） | 1 | 无 |

### 5.3 处置

- **CI 侧 50 处全部改掉**：`<<<`（herestring）取代管道，grep 的**方言、选项、
  模式串一字未动**。herestring 把文本交给一个临时文件，grep 读到文件末尾，
  没有任何写端可被打死。

  语义逐条核对过：`echo "$V"` 与 `printf '%s\n' "$V"` 都补一个换行，
  `<<< "$V"` 也补一个 ⇒ 等价；`printf '%s' "$V"` 不补换行而 `<<<` 会补，
  但那 5 处用的都是**非 `$` 锚定**的模式，结论不变。

- **CI 侧剩余 3 处**：`check_full_gate_verification.sh` 的 2 处是 `||`（误报，
  `grep` 直接读文件参数）、`check_rc_ga_gate.sh:946` 的 `wc -l | grep -qE`
  （`wc` 先消费完上游，写端不会被打死）。

- **非 CI 脚本的 121 处**：payload 全部是单行或固定小串，按上面的口径不构成
  真风险，**登记为运营债**，不在本批改动（避免无谓的 diff 面积与回归风险）。

---

## 6. 改后逐门禁实测（行为对照）

| 门禁 | 结果 |
|---|---|
| `check_evidence_binding.sh v4.1.0` | `PASS=129, WARN=123, FAIL=0, UNVERIFIED=0`（与改前**逐字一致**） |
| `check_ci_checkout_consistency.sh` | `ALL CHECKS PASS` |
| `verify_pr_dag.sh --selftest` | `passed: 5 / failed: 0` |
| `check_correctness_tests_production_storage.sh --selftest` | `passed: 5 / failed: 0` |
| `check_stage_config_consistency.sh` | `RESULT: PASS`（FAIL 0 / DISCLOSURE 0） |
| `check_off_production_path_registry.sh` | `PASS=10 FAIL=0` |
| `check_test_manifest_honesty.sh` | `PASS: 474 declared targets, 0 orphan test files` |
| `check_storage_db_scope_coverage.sh` | `PASS: 2 wrappers override all 16 database-scoped methods` |
| `check_storage_db_scope_coverage.sh` × 60 | RED **0** / 60 |

---

## 7. 顺带销账：P0-04（5 远端漂移）

`docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` 的 P0 计数由 2 降到 1，合计 15 → 14。

当次实测（`scripts/sync/5remotes_drift_check.sh`，只读）：

```
develop/v4.1.0	gitcode	gitea250	0	0
develop/v4.1.0	gitcode	gitea252	0	0
develop/v4.1.0	gitcode	gitee	0	0
develop/v4.1.0	gitcode	github	0	0
develop/v4.1.0	gitea250	gitea252	0	0
develop/v4.1.0	gitea250	gitee	0	0
develop/v4.1.0	gitea250	github	0	0
develop/v4.1.0	gitea252	gitee	0	0
develop/v4.1.0	gitea252	github	0	0
develop/v4.1.0	gitee	github	0	0

MAIN: OK — identical across all remotes (ac3fa16afb4a ac3fa16afb4a ac3fa16afb4a ac3fa16afb4a ac3fa16afb4a)

RESULT: all branches within threshold (2)
```

收敛动作与 9 处冲突的逐项裁决见
`docs/releases/v4.1.0/REMOTE_SYNC_CONVERGENCE_2026-10-23.md`。

**边界声明**：销账的是「远端**已经**漂移」这个**事实**。「drift gate 对 develop
也 fail-closed」「CI 周期跑 drift」这两条**尚未实现**，作为运营债留在 P2 ——
不假装已实现。

---

## 8. 附带发现（已修）

跑 `auto_env_blocker.sh` 时发现它默认是 **write 模式**，一次给 1501 个文档写入了
`env:blocked` 标记。本次已把工作树整体回滚到只剩本批的 15 个文件
（`git checkout -- .` 后按清单恢复），并删除了该脚本产生的
`docs/releases/v3.9.0/logs/gate_rc_*.log`。

**这条要在 CI 上注意**：`auto_env_blocker.sh` 若在 CI 里以 write 模式运行，
会改写大量文档。本次只改了它的**判定写法**（`<<<`），没有改它的模式默认值；
是否应在 CI 里固定为 check 模式，留作后续治理项。
