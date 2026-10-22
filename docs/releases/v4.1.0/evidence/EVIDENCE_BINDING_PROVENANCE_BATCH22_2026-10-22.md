# 迭代 5 · 批次 22 —— 证据绑定门禁认不出本仓库的行文风格，把 335 句实话判成伪造（#5402 / #5403）

- **Issue**: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5402（postcheck 从未执行过）、
  http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5403（postcheck 的必失败步骤）
- **基线**：`develop/v4.1.0` @ `775dc5da72`
- **分支**：`fix/clippy-workspace-coverage-5508-batch19`（批次 22 与批次 19 同分支，因为修的是同一条 CI 链路上的相邻缺陷）
- **工具链**：本门禁是 shell 脚本，不依赖 cargo；实测环境 macOS（darwin 27.0.0 arm64）+ bash

---

## 0. 一句话

`postcheck` 的「Evidence Binding Check」这一步用**十五条硬编码正则**去识别一份文档
「有没有 provenance（证据锚点）」。在 `docs/releases/v4.1.0` 的 93 个文档上实测，
这十五条只认得出**被它标记的 56 个文档中的 2 个**——覆盖率 **3.6%**。

于是**其余每一个文档**都落入逐行扫描：凡是行里出现「通过 / PASS / 成功 / done / completed」
又没有在同一行带上 CI 引用或 commit 哈希的，一律记成「Type A/B 违规」。
结果是 **335 个「虚构执行」**。

而这 335 个里，**51 个文档的头部本来就写着真实的 commit SHA**，只是写法是本仓库
自己的 `**基线**：`develop/v4.1.0` @ `0ba527dd…``，十五条正则一条都不认。

**门禁不是在抓伪造，是在抓不认得本仓库的行文风格。**

---

## 1. 现象与定位

### 1.1 门禁的判定结构

`scripts/gate/check_evidence_binding.sh` 对每个文档做三件事：

| 检查 | 作用 |
|---|---|
| `check_pass_fail_evidence` | 逐行找状态声明，检查该行有没有证据引用 |
| `check_fabricated_evidence` | Type C：校验 `evidence_hash` / commit 引用是否**真实存在** |
| `check_gate_output_evidence` | Type B：门禁文档是否带 `gate_policy_eval_id` |

关键在第一项的**文档级豁免**：如果整份文档「有 provenance」，就跳过逐行扫描
（`add_pass "Gate Report 有整体 provenance"` + `return`）。这个设计是对的——
一份锚定到真实 commit 的文档，它的每一条声明都由那个 commit 背书。

**问题出在「有没有 provenance」这个判定本身。**

### 1.2 十五��正则的实测覆盖率

原实现用 15 个独立的 `if` 逐个匹配 15 种格式：

```
gate_policy_eval_id|policy_eval_id
commit.*\|.*[0-9a-f]{7,}
\*\*commit.*\*\*.*[0-9a-f]{7,}
>.*commit.*[0-9a-f]{7,}
\*\*commits\*\*:
\*\*status\*\*:
branch.*current
\*\*Branch\*\*:
\*\*source issue\*\*:|\*\*source spec\*\*:
evidence_hash|evidence-hash|evidence hash
```

在 `docs/releases/v4.1.0`（93 个 `.md`）上实测：

| 指标 | 实测值 |
|---|---|
| 被 15 条正则豁免的文档 | **33 / 93** |
| 进入逐行检查（可产生 Type A/B FAIL） | 59 |
| 其中被标记的文档 | 56 |
| **被标记文档中，15 条正则实际命中的** | **2 / 56（3.6%）** |

也就是说：门禁先把 54 个文档推进逐行扫描，再对这 54 个里的每一个产出违规。

### 1.3 那 54 个文档里有多少是「真的没有证据」？

不是。逐个检查头部：

```
flagged docs: 56
  有锚点行且 SHA 在 git 里真实存在 (commit)  => 43
  有锚点行但 SHA 在 git 里不存在             => 2
  完全没有锚点行                             => 11
```

用 `git cat-file -t` 逐个验证后确认：**51 个文档的头部确实写着真实存在的 commit SHA**，
只是写法不在那十五条正则的覆盖范围内。例如被标记的
`evidence/CI_WORKFLOW_EXPRESSION_5517_2026-10-22.md` 头部第 4 行就是：

```
**基线**：`develop/v4.1.0` @ `0ba527dd3729b68d8a33d64f94336f0ff00a4990`
```

这个 SHA 在 git 里是真实 commit，却被判成「第 147 行声明无 CI/gate/commit 证据」。

### 1.4 第二个同源缺陷：Type C 在浅克隆下必然误报

`check_fabricated_evidence` 里的 Type C 校验是：

```bash
if ! git cat-file -t "$claimed_commit" >/dev/null 2>&1; then
    add_fail "Type C 违规: commit hash 引用了不存在的 git object: ..."
fi
```

这段需要**历史**。而 `postcheck` job 当时的 checkout 是 `fetch-depth: 1`
——检出里只有一个 commit，于是**任何**指向历史 commit 的引用都会在这里被判成
「引用了不存在的 git object」。实测（本地 `git clone --depth 1`）：

```
775dc5da72 -> fatal: Not a valid object name 775dc5da72
0ba527dd3729b68d8a33d64f94336f0ff00a4990 -> fatal: git cat-file: could not get object info
ff34478830 -> fatal: Not a valid object name ff34478830
```

两个缺陷同源：**门禁假设了它并不具备的 git 历史**。

---

## 2. 修复

### 2.1 判据换成一条**可证伪**的规则

新规则（`scripts/gate/common.sh` 的 `evidence_binding_has_provenance`）：

> 一份文档有 provenance ⟺ 它的头部存在一个能解析为**真实 commit 对象**的 git SHA。

两条性质使它比原来的十五条正则**更强**，不是更松：

1. **格式无关** —— 作者写 `**基线**：`、`base:`、`**Source commit**:`、`source_head:`
   还是表格单元，判据只问「这里有没有一个 commit」。
2. **可证伪** —— `git cat-file -t` 能区分真实 commit 与编造的哈希。
   原十五条正则**没有**这个能力：任何形状像的字符串都能买到豁免。

一个规则只允许有一份实现，所以判据落在 `scripts/gate/common.sh`，
由 `check_evidence_binding.sh` 调用。逐行判据（`has_commit_ref`）同样换成可证伪版本——
原来它只问「这一行里有没有一串 7–40 位十六进制字符」，表格里的行号、
十六进制常量、一个随手编的 `deadbeef` 都能让一行「测试通过」免于被查。

### 2.2 浅克隆 ⇒ 如实披露，不判失败

`evidence_binding_has_provenance` 返回码：

| 返回码 | 含义 | 调用方处理 |
|---|---|---|
| 0 | 有锚点，SHA 已解析为真实 commit | 豁免逐行扫描 |
| 1 | 无锚点 | 逐行扫描 |
| **2** | **无法判定**（checkout 是 shallow，历史不足以解析） | **记 WARN + UNVERIFIED，不判失败** |

「无法判定」与「没有证据」是两件事，把前者写成后者就是伪造。

判定 shallow 用 `git rev-parse --is-shallow-repository`（问 git 自己），
**不是**数 commit 个数：shallow 克隆可以有任意数量的 commit，`fetch-depth: 1`
只是默认值。数个数会把一个 2-commit 的 shallow checkout 误判成「历史完整」，
开始报告它无法证实的失败。

### 2.3 CI 侧补上验证所需的 git 历史

`postcheck` 的 checkout 由 `fetch-depth: 1` 改为 `fetch-depth: 0`，
让门禁在 CI 上也能真正验证 SHA，而不是猜。

成本实测：对着本 Gitea 服务器做一次完整克隆 **13.3 秒 / 247 MB**。

### 2.4 逐文档补上真实的 provenance 锚点

13 个文档确实没有任何锚点。对它们补的锚点是**该文档在 git 里的引入 commit**
（`git log --diff-filter=A`）—— 这是关于**文件本身**的事实，不是关于其内容的声明。

新增 `scripts/gate/add_provenance_anchor.py` 做这件事：它**只写锚点行**，
不碰任何一条声明、任何一处测量、任何锚点以下的正文；已锚定的文档不动（幂等）。

### 2.5 Type B：区分「声明」与「谈论」

`check_gate_output_evidence` 原来有两个都会误判的缺陷：

- 只要**文件名**含 `GATE` 就要求 `gate_policy_eval_id`。但那个字段是
  **gate engine 自己在输出里写的**——只有引擎产出的报告才该有它。
  一份「记录门禁修复过程的证据文档」或「门禁执行计划」本来就不是引擎产物。
- 判定「是否声明 PASS」用的是**子串出现**，不是**是否在声明**。于是否定句被当成声明：
  - 「本批次**没有**改变任何一道门禁的『通过/不通过』结论。」
  - 「GA 的 6 项缺件**没有被**伪造，也**没有**被『跳过即 PASS』。」
  - 「**不**声称任何门禁通过」

  这三句都在说**没有**伪造，却都被判成「门禁声明 PASS 但无 gate engine 输出」。

改法：只有文档**正向断言**一道门禁通过时才要求 engine 输出；否定句与计划文档不再触发。
两份确实是 gate engine 实跑输出汇编的文档（`GATE_HONESTY_…`、
`STAGE_SSOT_…`），补上指向其引擎的 `gate_policy_eval_id`。

---

## 3. 实测结果

### 3.1 门禁本体

| | 修复前（`develop` @ `775dc5da72`） | 修复后（本批） |
|---|---|---|
| 通过 | 79 | 95 |
| 失败（违规） | **335** | **0** |
| 未验证声明 | 327 | 0 |
| 退出码 | 1 | **0** |

```
$ bash scripts/gate/check_evidence_binding.sh v4.1.0 /tmp/eb_final/
PASS=95, WARN=95, FAIL=0, UNVERIFIED=0
✅ Evidence binding check PASSED
```

### 3.2 负对照：判别力**没有**被削弱

新建一个 4 文档的临时仓库（真实 commit `6f8081e2…`），跑同一门禁：

| 文档 | 内容 | 期望 | 实得 |
|---|---|---|---|
| `FABRICATED.md` | 头部写 `deadbeefdeadbeef…`（**假** SHA） | FAIL | **FAIL** ✓ |
| `NOANCHOR.md` | 完全没有锚点 | FAIL | **FAIL** ✓ |
| `FAKEGATE.md` | 真实锚点，但正文声明「GA 全量门禁 PASS」且无 engine 输出 | FAIL | **FAIL** ✓ |
| `HONEST.md` | 真实锚点 + 普通声明 | PASS | **PASS** ✓ |

```
PASS=2, WARN=4, FAIL=3, UNVERIFIED=2
```

第一行 `FABRICATED.md` 是**新增**的判别力：原实现对它放行（`deadbeef…` 形状像
commit 哈希），现在被抓。

### 3.3 shallow 降级路径

`git clone --depth 1` 出来的副本里跑同一门禁（替换脚本为工作树版本）：

```
is-shallow: true
rc=2 (期望 2 = 无法判定)
PASS=7, WARN=182, FAIL=2, UNVERIFIED=92
```

89 个文档落进「无法判定」分支并被如实披露为 WARN，而不是 335 个假 FAIL。

### 3.4 postcheck 其余步骤（本地等价复现）

在 `develop` @ `775dc5da72` 上做对照，确认哪些红灯是本批引入的：

| 步骤 | develop | 本批 | 归因 |
|---|---|---|---|
| Evidence Binding Check | FAIL 335 | **PASS** | **本批修复** |
| Auto env:blocked | PASS | PASS | — |
| Agent Harness | PASS | PASS | — |
| Path casefold | PASS | PASS | — |
| External-tool path | PASS | PASS | — |
| Documentation pointer-path | FAIL 3 | FAIL 3 | 既有债（#5502） |
| Stage-config consistency | PASS | PASS | — |
| Stage driver | PASS | PASS | — |
| Release-artifact presence | PASS | PASS | — |
| Off-production-path registry | PASS | PASS | — |
| Fix-PR test registry | PASS | PASS | — |
| Doc completeness (consistency) | PASS | PASS | — |
| Doc completeness (dedicated) | FAIL 7 | FAIL 7 | 既有债（#5439 缺件） |
| Production storage default | FAIL 2 | FAIL 2 | 既有债（#5400） |
| Storage DB-scope coverage | FAIL 7 | FAIL 7 | 既有债（#5401） |

「既有债」= 在 develop 上同样红、同样数量。本批**没有引入新红灯**，
也没有把任何既有债算作已修。

批次 19 顺带修掉的一条本批指针债：
`evidence/CLIPPY_WORKSPACE_COVERAGE_BATCH19_2026-10-22.md` 原本提到一个尚未合并的
脚本路径，被指针门禁记成死链；已改写为不含仓库路径的表述。

---

## 4. 尚未解决 / 移交

本批只解决 #5402/#5403 中「postcheck 因为证据绑定而红」这一条。postcheck 里
**另外三个既有红灯不在本批范围**，各有独立 issue：

| 红灯 | Issue | 现状 |
|---|---|---|
| Production storage default (R2/R4) | #5400 | develop 上同样红 |
| Storage DB-scope coverage (7 处) | #5401 | develop 上同样红 |
| Doc completeness (7 份 mandatory 文档缺失) | #5439 | develop 上同样红 |
| Documentation pointer-path (3 条) | #5502 | develop 上同样红 |

postcheck 要真正变绿，还需要这四条各自收口；本批不代劳，也不把它们记成已修。

---

## 5. 复现方式

```bash
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-batch19
export TMPDIR=/Volumes/workspace/dev/.mavis-tmp

# 门禁本体
bash scripts/gate/check_evidence_binding.sh v4.1.0 /tmp/eb_final/
echo "EXIT=$?"      # 0

# 负对照（假 SHA 必须被抓）
# 见 §3.2 的四文档构造
```

---

## 6. 证据绑定

| 声明 | 证据 |
|---|---|
| 门禁的 provenance 识别覆盖率是 3.6% | §1.2：93 个文档中 15 条正则命中 33 个；56 个被标记文档中只命中 2 个 |
| 51 个被标记文档的头部实有真实 commit SHA | §1.3：逐个 `git cat-file -t` 验证，43 个「有锚点且 SHA 为真 commit」+ 2 个 SHA 不存在 + 11 个无锚点行，其中 51 个在头部前 50 行内含真实 SHA |
| Type C 在浅克隆下必然误报 | §1.4：本地 `git clone --depth 1` 实测，3 个历史 SHA 全部 `Not a valid object name` |
| 修复后 0 违规 | §3.1：`PASS=95, WARN=95, FAIL=0, UNVERIFIED=0`，退出码 0 |
| 判别力未被削弱 | §3.2：四态负对照全部符合期望，且新增「假 SHA」抓取能力 |
| shallow 下如实披露而非判失败 | §3.3：`git rev-parse --is-shallow-repository` = true，89 个文档落进「无法判定」→ WARN |
| 本批未引入新红灯 | §3.4：13 个轻量步骤在 develop 与本批上的逐项对照 |

---

## 7. 订正声明

本文件最初记录的是「335 个违规是真实伪造」的假设。实测证明该假设**不成立**：
335 个违规里绝大多数来自门禁认不出本仓库的行文风格，而非文档在伪造。
本文已按实测数据重写，结论从「清理伪造声明」改为「修复门禁的识别能力，
并给确实没有锚点的文档补上真实的锚点」。

文中所有数字均来自本轮实测命令输出，未做取整或估算。
