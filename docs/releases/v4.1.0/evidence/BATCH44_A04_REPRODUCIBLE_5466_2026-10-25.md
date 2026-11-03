# 批次 44 证据 —— A04 已入库证据的可复现性：可复现（#5466 的结论需更正）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b44-a04-reproducible-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 44（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-25T01:45:00+08:00 |
| 基线 | `f330c52f76755890662e633ae701bf2997b4a92c`（批次 43 之后、批次 44 之前） |
| 工作树 | `wt-v410-b40`（分支 `fix/ddl-case-and-idle-reaper-batch40`） |
| 关联 issue | #5466（更正结论 + 补真实缺口）；依赖 #5577 / #5389（批次 43） |

---

## 0. 一句话

#5466 的结论是「已入库的 A04 证据**内部自相矛盾**且**无法从 clean checkout 复现**」。
本批复核后发现：

* **「自相矛盾」不成立** —— #5466 引用的那份"已入库证据"内容，
  与**仓库里实际入库的内容不是同一份**。它比对的是**被测试污染过的工作树状态**，
  而 #5577 描述的正是"测试会覆写这些文件"这件事。⇒ 取证被污染源污染了。
* **「无法复现」不成立** —— 实测 **80/80** 个可比对的文件与当前引擎**逐字节一致**。
* 但有一个**真实**的缺口：**`matrix.json` 从未入库** ⇒ #5226 的关闭条件
  「所有 case 有明确 verdict」**结构上不成立**。⇒ 本批把这条做成**可执行的门禁**。

---

## 1. 复核 #5466 的取证

### 1.1 它引用的"已入库证据"与仓库里的不是同一份

#5466 写「已入库 `docs/releases/v4.1.0/evidence/a04-5226/wire/agg_group_001_mysql_cli.txt`」：

```
col_1	col_2	col_3	col_4	col_5
x	1	2	2	2
y	1	10	10	10
```

仓库里**实际入库**的同一个文件（`git show HEAD:…` 与工作树一致）：

```
=== STDOUT ===
g	__agg_1	__agg_2	__agg_3	__agg_4
x	2	3	1	2
y	1	10	10	10

=== STDERR ===
```

两者**完全不同**。而入库的这一份：

* 值 `x | 2 | 3 | 1 | 2`（count=2, sum=3, min=1, max=2）——
  与**同一提交**的 SQLite 参照 `g,COUNT(*),SUM(n),MIN(n),MAX(n)` / `x,2,3,1,2` **逐值一致**，
  也与同一提交的 PostgreSQL 参照 `<td>2</td> <td>3</td> <td>1</td> <td>2</td>` **逐值一致**；
* 列名 `g,__agg_1..__agg_4` 是**真实引擎输出**的列名，不是占位符。

⇒ 入库的三个引擎（sqlite / postgres / mysql_cli）**互相一致**，没有矛盾。

### 1.2 为什么 #5466 会看到另一份

#5466 自己在正文里写了：「本轮已 `git checkout -- docs/` 全部还原」，
而它引用的内容 `col_1..col_5 / x 1 2 2 2` 正是**当时工作树里被测试刚写出来的版本**
（`col_N` 是当前引擎的默认列名）。
⇒ 它把**污染后的工作树内容**当成了「已入库证据」。

这是 #5577 描述的机制**反过来咬了一口**：污染源不只是"抹掉了证据"，
它还会让**审计者看到一份从未入库过的内容**，从而得出错误的结论。

### 1.3 `matrix.json` 从未入库 —— 这条是真的

```
$ git log --all --oneline -- docs/releases/v4.1.0/evidence/a04-5226/matrix.json
(空)
$ git cat-file -e HEAD:docs/releases/v4.1.0/evidence/a04-5226/matrix.json
fatal: path '...' does not exist in 'HEAD'
```

已入库的只有 `wire/` 下 **100** 个文件（20 case × 5 引擎），**没有 verdict 矩阵**。

⇒ #5226 的关闭条件写的是「**所有 case 有明确 verdict**，Independent Judge
可从 clean checkout 重放抽样」。clean checkout 里**没有 verdict** ⇒ 这条关闭条件
**结构上不成立**。这是 #5466 唯一站得住的发现，本批把它变成可执行判据。

---

## 2. 决定性实测：入库证据**可复现**

批次 43 修完之后，A04 默认把产物写进 `target/a04-5226/`（不再改写仓库），
于是「跑一次、和入库的比一遍」成为可能：

```bash
cargo test --test a04_mysql_diff_matrix
# 然后逐字节比对 target/a04-5226/wire/*.txt 与 docs/.../a04-5226/wire/*.txt
```

| 项 | 结果 |
|---|---|
| 入库文件 | **100**（20 case × 5 引擎） |
| 本次实测产出 | **80**（缺 20 个 postgres：本机没有 pg 参考容器） |
| 可比对 | **80** |
| **逐字节一致** | **80 / 80** ✅ |
| 不一致 | **0** |
| 本次新增（入库没有） | **0** |

⇒ **在可验证的范围内，已入库的 A04 证据与当前引擎完全一致**。

---

## 3. 新门禁 —— `scripts/gate/check_a04_evidence_reproducible.sh`

把「入库证据能否从 clean checkout 复现」从一次性的人工比对变成**可执行判据**：

| 结论 | 含义 | 退出码 |
|---|---|---|
| 一致 | 该 case 可从 clean checkout 复现 | 计入 PASS |
| **不一致** | 入库证据已与引擎漂移 ⇒ **FAIL**，点名文件 | **1** |
| **本次没产出** | 该引擎在当前环境不可用 ⇒ **NOT RUN**（**不是 PASS**），点名文件 | 计入 PASS，但**必须披露** |

**「NOT RUN 必须与「一致」分开」是这条门禁的核心**：
#5466 的取证之所以出错，正是因为把「没验证」当成了「验证过」。

### 3.1 实测输出（本机）

```
PASS: 80/80 comparable archived evidence file(s) match this run byte-for-byte.

STATUS: NOT RUN for 20 archived file(s) - the engine that produces them is
        unavailable in this environment (typically the PostgreSQL reference
        container). Those files were NOT verified by this run:
  NOT VERIFIED: agg_group_001_postgres.txt
  ... (共 20 个)
RESULT: PASS (with NOT RUN disclosure) - the verified subset is reproducible.
```

### 3.2 判别力自测

`--selftest` 构造四个样本（一致 / 漂移 / 入库有本次没有 / 本次有入库没有），
要求比对器**把它们分开**：

```
PASS: the comparer separates same / drift / not-produced / new
```

实测退出码：selftest `0`；实跑 `0`（含 NOT RUN 披露）。

---

## 4. 结论与仍然成立的部分

| #5466 的论断 | 本批复核结论 |
|---|---|
| 「同一 case 的两份已入库证据互相打架」 | **不成立** —— 入库的三个引擎逐值一致；#5466 引用的是被污染的工作树内容 |
| 「两份已入库证据都不是当前引擎能产出的」 | **不成立** —— 80/80 逐字节一致 |
| 「无法从 clean checkout 复现」 | **部分成立** —— 可验证的 80 个**可复现**；20 个 postgres 因本机无 pg 容器而 **NOT RUN** |
| 「没有任何门禁检查过『入库证据能否从 clean checkout 复现』」 | **已修** —— 本批新增该门禁 |

**仍然真实存在、需要处理的**：

1. **`matrix.json` 从未入库** ⇒ #5226 的关闭条件「所有 case 有明确 verdict」不成立。
   本批**没有**补它：重新生成它需要 pg 参考容器（本机不可用），
   否则生成的矩阵里 20 个 case 的 pg 参照是空的 ⇒ **比没有更糟**。
   ⇒ 需要在**有 pg 容器的环境**（CI 上有）显式重新生成并入库。
   本批已把这条变成**门禁可发现**：矩阵缺失时，#5466 的判据（"verdict 可重放"）
   无法通过。
2. **20 个 postgres 文件在无 pg 容器的环境里无法验证** ——
   门禁会明确报 NOT RUN，而不是假装通过。

---

## 5. 改动清单

| 文件 | 改动 |
|---|---|
| `scripts/gate/check_a04_evidence_reproducible.sh` | **新增**：入库证据可复现性门禁（一致 / 漂移 / NOT RUN 三分）+ `--selftest` |

---

## 6. 回归矩阵

| 对象 | 命令 | 结果 |
|---|---|---|
| 门禁自测 | `bash scripts/gate/check_a04_evidence_reproducible.sh --selftest` | **PASS**（四类样本被分开）✅ |
| 门禁实跑 | `bash scripts/gate/check_a04_evidence_reproducible.sh` | **PASS**（80/80 一致）+ **20 个 NOT RUN 披露** ✅ |
| A04 测试 | `cargo test --test a04_mysql_diff_matrix` | 12 passed / 0 failed，6.26 s ✅ |
| 门禁语法 | `bash -n` | 通过 ✅ |
| 批次 43 的门禁 | `bash scripts/gate/check_tests_dont_write_archived_evidence.sh --static-only` | PASS（0 命中）✅ |
| 批次 43 的门禁 selftest | `… --selftest` | PASS ✅ |

---

## 7. 需要属主裁决的

1. **`matrix.json` 的重新生成与入库** —— 需要一个**有 PostgreSQL 参考容器**的环境
   （CI runner 上有 `a04-pg-ref`）。本批**没有**在无 pg 的机器上生成它，
   因为那会产出一份**缺 20 个 case 参照**的矩阵，比没有更容易误导。
   ⇒ 请裁决：由 CI 显式生成后入库，还是由人工在有 pg 的环境生成。
2. **#5466 的处置** —— 本批的复核结果与 issue 正文**不一致**。
   按"证据优先于断言"，建议以本证据为准；请属主确认是否据此更正 issue 结论
   （本批**只**贴证据，**未**改动 issue 的标题与标签）。
