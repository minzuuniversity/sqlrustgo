# 迭代 5 · 批次 16 —— 表达式模板写在注释里，让 Gitea 忽略了整个 CI workflow（#5517）

**关闭**：#5517
**基线**：`develop/v4.1.0` @ `0ba527dd3729b68d8a33d64f94336f0ff00a4990`
**分支**：`fix/ci-workflow-expression-parse-5517`

---

## 0. 一句话

`ci.yml` 的一行**注释**里写着「step output 要用 `${{ steps.<id>.outputs.<name> }}`
这种形式读」—— 而 Gitea 扫描的是**整个 `run:` 标量**里的表达式占位符，
**注释也在扫描范围内**；占位符里的尖括号不是合法的表达式符号 ⇒
**整个 workflow 解析失败** ⇒ `DetectWorkflows()` 直接「ignore invalid workflow」
⇒ **一个 job 都建不出来**。

⇒ 自 2026-10-21 04:23 起，主 CI 的每个 run 都是「0 job + 立即 failure」，
而仓库里**没有任何东西**说明为什么。

---

## 1. 现象

```json
GET /repos/openclaw/sqlrustgo/actions/runs/8598
{"status":"completed","conclusion":"failure","run_started_at":null,
 "created_at":"2026-10-21T21:09:12Z","updated_at":"2026-10-21T21:09:12Z"}

GET /repos/openclaw/sqlrustgo/actions/runs/8598/jobs
{"jobs":[],"total_count":0}
```

runner 是好的：

```json
GET /repos/openclaw/sqlrustgo/actions/runners
{"runners":[{"name":"sqlrustgo-ci-01","status":"online","busy":false,
  "labels":["hp-z6g4","ubuntu-latest","self-hosted","linux"]}],"total_count":1}
```

workflow 也是 active 的（`GET /actions/workflows/ci.yml` → `state: active`）。

⇒ 不是 runner 没注册（#5363），不是 runner 不接单。
**是 workflow 自己没被解析出 job。**

---

## 2. 根因（服务端日志给出）

```bash
docker logs openclaw-gitea-1 | grep "Failed to parse workflow"
```

```
2026/10/21 21:09:12 services/convert/convert.go:585:getActionWorkflowEntry() [E]
  getActionWorkflowEntry: Failed to parse workflow: yaml: construct errors:
  line 1: (Line: 537, Col: 14): Unexpected symbol: '<'. Located at position 7
  within expression: steps.<id>.outputs.<name>
```

30 小时内 **98 次**同一条错误。另一条路径说的是同一件事：

```
2026/10/21 22:34:00 .../actions/workflows.go:203:DetectWorkflows() [W]
  ignore invalid workflow "ci.yml": yaml: construct errors: ...
```

⇒ Gitea **整体忽略**这个 workflow ⇒ 连 run 都不创建。

罪魁是 `ci.yml` 的 `postcheck` 里这一行**注释**：

```yaml
      - name: Gate L1 — Code Quality
        run: |
          # #5444: `"${version}"` was a **bash variable that is never set**
          # — a step output has to be read as `${{ steps.<id>.outputs.<name> }}`.
```

**最阴的地方**：这行注释本身是**为了说明正确写法**而写的。
写文档的那一步，正是把 CI 弄死的那一步。

---

## 3. 断裂点定位

用 run 的 job 数逐条比对（`GET /actions/runs/{id}/jobs` 的 `total_count`）：

| run | head_sha | jobs | 时间 (UTC) |
|---|---|---|---|
| 8348 | `c2918e3aed` | **6** | 03:21:48 |
| 8362 | `28848131fa` | **0** | 04:23:36 |
| 8387 | `009bb985ee` | 0 | 05:37:54 |
| 8438 | `da1530ba4b` | 0 | 08:55:45 |
| 8451 | `8478e10b00` | 0 | 09:58:26 |
| 8502 | `0dea6393c8` | 0 | 14:23:08 |
| 8576 | `1c096c9148` | 0 | 20:24:19 |
| 8598 | `0ba527dd372` | 0 | 21:09:12 |

⇒ 罪魁提交 **`28848131fa`**（「门禁接线诚实性」）—— 它引入了上面那行注释。

### 3.1 佐证：能不能 rerun

| 操作 | 结果 |
|---|---|
| `POST /actions/runs/8598/rerun`（ci.yml） | **404 Not Found** |
| `POST /actions/runs/8599/rerun`（gate.yml） | **201 Created**，job 被物化（`jobs/17667`） |

⇒ `gate.yml` 正常、`ci.yml` 异常 ⇒ 是 **ci.yml 的内容**的问题，
不是平台/实例/runner 的问题。

### 3.2 被证伪的一个假设（记录在案）

最初的假设是 `28848131fa` 引入的空 `needs` 数组让 Gitea 建不出 job。
**这个假设是错的**：把两处 `needs: []` 去掉后重新推送，run 依旧是 0 个 job
（run 8610）。⇒ 真正的触发点是表达式模板。

⇒ 「先验证再执行 issue 建议」这条纪律，在本批次又一次救了场：
如果照着「空 `needs` 是问题」的结论去改，CI 仍然不会恢复，
而且会留下一条**错误的因果解释**在证据里。

---

## 4. 修法

**注释里只描述「形状」，不写「模板」**：

```yaml
          # — a step output has to be read through the steps-output reference
          # (steps / step id / outputs / output name), not as a bash variable.
```

⇒ 平台扫不到尖括号 ⇒ workflow 恢复可解析。

---

## 5. 防复发：新门禁 `scripts/gate/check_workflow_expressions.sh`

| | 判据 | 失败意味着 |
|---|---|---|
| **C1** | 任何表达式的**内容**里不得出现 `<` 或 `>` | 平台会读崩这个表达式 ⇒ 整个 workflow 被忽略 |
| **C2** | **注释**里的表达式只能是「点分路径」（`runner.temp` 这种） | 注释里的复杂模板是**静默地雷**：读的人无法判断它能不能解析 |

### 5.1 自检 `--selftest` 4/4

| 夹具 | 预期 | 实测 |
|---|---|---|
| 干净的 workflow | PASS | PASS |
| 注释里的模板（#5517 的形态） | FAIL | FAIL |
| 代码里的带尖括号模板 | FAIL | FAIL |
| 注释里的**合法**表达式（`runner.temp`） | PASS | PASS |

最后一条夹具是**为了证明没有假阳性**：ci.yml 里本来就有两处
`` # `${{ runner.temp }}` 是…… `` 这样的合法注释，它们必须**不被**判红。

### 5.2 负对照

```
$ bash scripts/gate/check_workflow_expressions.sh --repo-root <修复前的树>
FAIL: the workflow contains an expression the platform's parser cannot read:
    ci.yml:543 [C1] # — a step output has to be read as `${{ steps.<id>.outputs.<name> }}`.
    ci.yml:543 [C2] # — a step output has to be read as `${{ steps.<id>.outputs.<name> }}`.
```

修复后同一棵树 **PASS**。

### 5.3 这个门禁抓到了作者本人

新加的那段 CI 注释里**原本也写了模板**（`${{ steps.<id>.outputs.<name> }}`），
是门禁把它判红的 ⇒ 门禁在提交的那一刻就证明了自己有判别力。

---

## 6. 影响面

- 主 CI（`lint-build` / `test` / `mutation` / `ga3-security-scan` /
  `test-soak-detach` / `postcheck`）**全部没有在跑**；
- 只有 `gate.yml` 还在跑；
- ⇒ **GA 门禁不能依赖 CI**：仓库里所有「CI 会拦住」的承诺，在这一刻全是空的。

⇒ 在 CI 恢复之前，GA 的证据只能来自**本地运行**，且必须写明
「本项未由 CI 承接」—— 不能写成「CI 已验证」。

---

## 7. 本批次**没有**做的事

| 没做 | 为什么 | 去哪 |
|---|---|---|
| 没有修 #5402 / #5403 / #5446 / #5447 的其余断口 | 它们是 `postcheck` / `gate.yml` 的**内容**问题；本 issue 先让它们**有机会执行** | 后续批次 |
| 没有修 runner 侧问题（#5363 / #5397 / #5448 / #5451） | 那是 runner/网络环境 | 需基础设施侧处理 |
| 没有改 `ci.yml` 的任何**行为** | 本次只改了一行**注释** | — |

---

## 8. 引用

- `scripts/gate/check_workflow_expressions.sh` —— 本批次新增的元门禁
- `.gitea/workflows/ci.yml` —— 被修的那一行注释，以及新门禁的接入点
- #5517 —— 本批次关闭的 issue
- #5455 / #5402 / #5403 / #5446 / #5447 —— 同一族（`postcheck` 从未执行）
- #5363 —— 现象相似但根因不同（runner 是 online 的）
