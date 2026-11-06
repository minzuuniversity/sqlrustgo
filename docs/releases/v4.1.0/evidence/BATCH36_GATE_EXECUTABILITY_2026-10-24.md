# 批次 36 证据 —— 门禁**执行点矩阵**（GA 清单不再只是声明）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b36-gate-executability-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 36（本机实测，Mac-mini-7.local arm64 macOS 27.0.1） |
| timestamp | 2026-10-24T17:40:00+08:00 |
| 基线 | `develop/v4.1.0` = `7eff3f7558da` |
| 工作树 | `wt-v410-b36`（分支 `fix/gate-executability-batch36`） |
| 关联 issue | #5461（3 项 GA 门禁无执行点）、#5450（发布门禁脚本无执行者） |

---

## 0. 一句话

`STAGE_CONFIG.yaml` 的 `stages.<S>.required_gates` 是**声明式**清单，
而 CI 里唯一调用阶段驱动的地方带 **`--dry-run`** ⇒
**清单里的门禁一个都没执行**。实测：GA 的 31 项里 **12 项**在任何 CI workflow 里
都没有执行点。

本批把「有没有人跑」变成**可测量、fail-closed** 的判据
（`scripts/gate/check_gate_executability.sh`），并给清单接上真正的执行点
（新 job `stage-gates`，在承载分支上**真的**跑该阶段的清单）。

---

## 1. 现象（当次实测，2026-10-24）

### 1.1 CI 里唯一的阶段驱动调用是 dry-run

```
$ grep -n "check_stage.sh" .gitea/workflows/ci.yml
1362:          bash scripts/gate/check_stage.sh --version v4.1.0 --dry-run
```

`scripts/gate/check_stage.sh` 的执行分支：

```bash
if [[ "$DRY_RUN" == true ]]; then
    say "  [DRY]  $g_expanded"
    continue          # ← 一个门禁都没跑
fi
```

⇒ **清单被逐条打印，然后被跳过。**

### 1.2 缺口有多大

`stages.GA.required_gates` 共 **31 项**（其中 **27 项**是脚本门禁）。
逐项解析「这个脚本在 `.gitea/workflows/*.yml` 的任何 step 里出现过吗」
（**用 YAML 解析** ⇒ 注释天然被排除）：

| stage | 清单项 | 脚本门禁 | **无 CI 执行点** |
|---|---|---|---|
| DRAFT | 2 | 1 | 1 |
| ALPHA | 10 | 7 | 5 |
| BETA | 19 | 15 | **12** |
| RC | 26 | 22 | **17** |
| **GA** | **31** | **27** | **22** |

> 这一版是用「只看 CI 直接调用」的口径统计的。加入**传递可达性**
> （门禁调用门禁）后，数字降到 **GA 12 / BETA 6**（见 §2.2）——
> 差额是那些**间接**被执行的门禁（例如 `check_alpha_v410.sh` 会调
> `check_arch_invariants.sh`）。

⇒ #5461 记的是「GA 20 项里 3 项没有执行点」；**当次实测是 31 项里 12 项**
（项数在清单增长后变了；口径也从「直接调用」收紧成「传递可达」）。

### 1.3 #5461 的两条前提已过期（当次重测）

| #5461 的说法 | 2026-10-24 实测 |
|---|---|
| `postcheck` 因 `needs: [test]` 被完全遮蔽、0 次执行 | **已修**：`postcheck:` 现在是 `needs: []`；run 9669 / 9647 的 postcheck **确实执行了** |
| `check_coverage_v312.sh` 在 CI 里 0 调用点 | **已修**：`.gitea/workflows/ci.yml:1236` 现在调的就是 `check_coverage_v312.sh`（不再是废弃的 `check_coverage.sh` 桩） |
| `check_anti_fabrication.sh` 在 CI 里 0 调用点 | **仍成立**（只在 `postcheck` 里）⇒ 但它有执行点，不是缺口 |

⇒ 剩下的真实缺口是 **`check_sql_corpus_gate.sh`**（0 调用点）
＋ **12 项「只有清单、没有执行点」的 GA 门禁**。

---

## 2. 修法

### 2.1 一份实现：`scripts/gate/gate_executability.py`

**判据两条，缺一不可**：

| 判据 | 内容 | 防的是什么 |
|---|---|---|
| **C1 存在性** | `required_gates` 里的脚本（`{VER}` 按仓库既有拼写变体解析）必须落到**真实文件** | 清单里写着不存在的脚本 |
| **C2 可达性** | 该脚本必须**真的会被执行**：CI step 直接调用 ∨ 被「会被调用的门禁」传递调用 ∨ 落在「CI 里**非 --dry-run** 地、显式钉了 `--stage <S>`」的阶段驱动清单里 | 清单是**装饰性**的 |

实现要点：

* **workflow 用 YAML 解析** ⇒ YAML 注释天然被排除；
* **`run:` 块里的 bash 注释也要剥掉**（`strip_shell_comments`）——
  否则「注释里提到门禁」会被当成「调用了门禁」（本门禁第一次跑就踩了这个坑，
  假通过 88/88）；
* **`--dry-run` 的阶段驱动不算执行点** —— 它只打印 `[DRY]`；
* **阶段驱动必须显式钉 `--stage <S>`** —— 否则 develop 上的一次调用会把
  GA 的清单也算成有执行者，「清单 ↔ 执行者」就对不上了；
* **传递可达性** —— 门禁之间本来就走 scripts/gate/xxx.sh 这个形状，
  文本扫描即可，不需要解析 bash。

### 2.2 新的执行点：CI job `stage-gates`

```yaml
  stage-gates:
    runs-on: [hp-z6g4]
    needs: []
    if: github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/heads/release/') || … 
    steps:
      - name: Checkout            # 与其它 job **逐字节相同**的 canonical body
      - name: Restore cargo registry cache (issue #5451)
      - name: Stage gates — BETA list executes for real (beta/v* carrier)
        if: startsWith(github.ref, 'refs/heads/beta/')
        run: bash scripts/gate/check_stage.sh --version v4.1.0 --stage BETA
      - name: Stage gates — RC list executes for real (rc/v* carrier)
        if: startsWith(github.ref, 'refs/heads/rc/')
        run: bash scripts/gate/check_stage.sh --version v4.1.0 --stage RC
      - name: Stage gates — GA list executes for real (main / release/v* / hotfix/v* carrier)
        if: github.ref == 'refs/heads/main' || …
        run: bash scripts/gate/check_stage.sh --version v4.1.0 --stage GA
```

* **显式钉 `--stage`** ⇒ 「清单 ↔ 执行者」一一对应；
* **只在承载分支上跑** —— 阶段门禁是**晋级事件**的判据，不是每次 develop push
  都该付的代价；
* `needs: []` —— #5455 的教训：门禁必须能在一个「已知会红」的分支上跑。

### 2.3 门禁自身也进清单、进 CI

* `stages.GA.required_gates` 增加
  `scripts/gate/check_gate_executability.sh`（GA 清单 31 → **32** 项）；
* `ci.yml` 的 `postcheck` 增加一步 `Gate executability check (#5461 / #5450)`
  （~1s，**每次 PR 都跑**）⇒ 有人往清单里加了一个没人跑的门禁，
  **当天**就被挡住，而不是等到发布前。

### 2.4 改动前后

| | 清单项 | OK | 无执行点 |
|---|---|---|---|
| 改动前 | 88 | 76 | **12** |
| 改动后 | **89** | **89** | **0** |

---

## 3. 判别力证据（变异自测 A/B/C/D）

`bash scripts/gate/check_gate_executability.sh --selftest`：

| 臂 | 注入 | 期望 | 实测 |
|---|---|---|---|
| A | 清单里有一个**没人调用**的门禁 | 红，且**点名它** | ✅ `NO-EXECUTOR` 且点名 |
| B | 给它补上 CI 调用 | 绿 | ✅ 绿 |
| C | 只用 `check_stage.sh --dry-run` | **不算**执行点 ⇒ 仍红 | ✅ 仍判 `NO-EXECUTOR` |
| D | `run:` 块里的 **bash 注释**提到门禁 | **不算**调用 ⇒ 仍红 | ✅ 仍判 `NO-EXECUTOR` |

D 臂是**本门禁自己的第一个 bug**：第一版把 ci.yml `test` job 里那段
bash 注释（`# It is NOT enforced by scripts/gate/check_stage.sh: …`）
当成了「非 dry-run 的驱动调用」⇒ 矩阵假通过 88/88。加了 D 臂后立刻暴露并修掉。

---

## 4. 编译与门禁

| 检查 | 结果 |
|---|---|
| `check_gate_executability.sh` | **89 / 89 OK** |
| `check_gate_executability.sh --selftest` | **PASS**（A/B/C/D） |
| `check_ci_checkout_consistency.sh` | **PASS**（C1–C5 全过 ⇒ 新 job 的 checkout 与 canonical body 逐字节相同） |
| `check_stage_config_consistency.sh` | **PASS** |
| `check_off_production_path_registry.sh` | **PASS=10 FAIL=0** |
| `.gitea/workflows/ci.yml` YAML 解析 | **OK** |

---

## 5. 边界与未完

* 本批**只**解决「有没有执行点」。清单里的门禁**本身红不红**是另一件事 ——
  已知既有红（`check_sem1_savepoint.sh`、`check_g1_tpch_baseline.sh`、
  RC/GA 门禁的 A5 覆盖率 67% < 75%、D6 `regression_test`）**不在本批范围**，
  也没有被本批掩盖。
* `stage-gates` job 只在**承载分支**上跑 ⇒ develop 上不会因为它红。
  但它意味着：**GA 承载分支上的 CI 会真的执行 GA 的 31 项清单**，
  其中若干项当前是红的 —— 那是**真相**，不是本批引入的。
* #5491（GA-2 的 SOAK 判据只检查脚手架文件在不在）与
  #5450（`RELEASE_GATE_CHECKLIST.md` 的 4 个门禁脚本 3 个不存在）
  **未在本批实施**，下一批。
