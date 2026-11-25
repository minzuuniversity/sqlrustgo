# R2 外审包 —— 生成说明与实测记录

> 任务：V410-READINESS-IMPLEMENTATION-PLAN / Task 9 Step 2–3
> 授权依据：属主裁决 **O8**（发 R2，R1 不动）+ 控制者裁决 **R64 / R68**
> 生成执行者：Task 9 实施者（**未**代属主签署任何东西）
> 生成时间：2026-11-25
> R1 目录：`…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0/`
> R2 目录：`…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2/`
> 构建脚本：`…/DSH-EXTERNAL-REVIEW-001/r2-build/`（`build_r2.sh` 是唯一入口）

---

## 0. 一句话

**R2 是 R1 的兄弟目录副本，改了 R2 身份（`review_id` / `review_level` / `source_commit`），
按 O8 授权修正了 4 类问题（计数漂移 / dead 口径 / source_commit / 自排除规则），
然后用 R1 自带的工具链重新生成并自校验。R1 的 `package/` 与 zip 在全过程前后逐字节未变。**

---

## 1. R1 保持不动（O8）

| 项 | 值 |
|---|---|
| R1 `package/` 文件数 | 21（含 `FREEZE.json`） |
| R1 `package/FREEZE.json` sha256（生成前） | `8363343618a6e659b9e94a52d2ba2f16cd5d9c3d9f3eb94dbe7dcfa514b85e36` |
| R1 `package/FREEZE.json` sha256（生成后） | `8363343618a6e659b9e94a52d2ba2f16cd5d9c3d9f3eb94dbe7dcfa514b85e36`（**相同**） |
| R1 zip sha256（生成前 / 后） | `4f6c3ad01789b1a719043e0b64b89ec3e0ab09903512183bb21aebf0804f1996`（**相同**） |
| R1 `package/` 逐文件复核 | 生成前 21 行 sha256 快照 vs 生成后 21 行 → **`diff` 无输出，逐字节未变** |
| R1 文件 mtime | `FREEZE.json` 11月23 07:14、zip 11月23 07:09 —— **未变**（证明没有写入） |

> 快照是硬约束的执行凭据：生成前逐文件 sha256 落盘，生成后逐一比对。比对脚本是
> `r2-build/build_r2.sh` 的第 0 步与第 7 步，任何不一致都会让构建**以非零码退出**。

## 2. 为什么在副本上生成（brief 与工具行为的正面冲突）

计划正文 Step 3 写「用 `finalize_package.py` 与 `gen_export_approval.py` 生成 R2，**不触碰 R1 的
`package/` 目录**」。**按工具的构造做不到**：

- `finalize_package.py` 的 docstring 明写「Writes only into `sqlrustgo-v4.1.0/package/`」，
  且 `REVIEW_ID` / `SOURCE_COMMIT` / `REVIEW_LEVEL` / `COLLECTED_AT` **全部硬编码成 R1**；
- `freeze_package.py` 把 `FREEZE.json` 写进 `package/` 内，默认拒绝重冻结（除非 `--force`）；
- `gen_export_approval.py` 读 `package/SHA256SUMS` 并重算 zip 哈希。

直接跑 = **覆盖 R1 的 `package/`**。因此：

1. 先把 R1 的 `package/` 逐文件 sha256 快照（见 §1）；
2. 把 `sqlrustgo-v4.1.0/` **整个复制**成兄弟目录 `sqlrustgo-v4.1.0-r2/`；
3. **只在副本上**改 R2 身份并生成；**原目录一个字节都不写**；
4. 生成后逐文件复核（§1）。

## 3. R2 的身份与三项授权修正

| 项 | R1 | R2 |
|---|---|---|
| `review_id` | `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123` | `SQLRUSTGO-V410-REFACTOR-HARNESS-R2-20261125` |
| `review_level` | R1 | **R2** |
| `source_commit`（包级） | `cd8cefe623` | **`fd849ae20541901d5e59f45cd3ba2c51093f5d5e`（BASE）** |
| `source_commit`（逐条 evidence） | `cd8cefe623` | `cd8cefe623`（**保留** —— 见 §3.3） |
| `collected_at`（包级） | 2026-11-23T06:30:00+08:00 | 2026-11-25T09:45:00+08:00（包级） |
| `collected_at`（逐条 evidence） | 2026-11-23T06:30:00+08:00 | 2026-11-23T06:30:00+08:00（**事实采集时刻，不改**） |
| zip | `SQLRUSTGO-V410-REVIEW-PACK-DRAFT.zip`（104822 B） | `SQLRUSTGO-V410-REVIEW-PACK-R2-DRAFT.zip`（122883 B，sha256 `ea3611eee5da3ab02876a8cd5dadab54e250f2448dd53d5cfe20eaae240bc386`） |

### 3.1 O8 授权的 4 类修正

| 设计稿 finding | 修正 | 实测（修正前 → 修正后） |
|---|---|---|
| **R1-F-001** 计数漂移 | `EVIDENCE-COVERAGE.md` 的统计由 `evidence_index.json` 单向生成 | 68 条 21/41/6 → **71 条 24/41/6**（补 E-069/E-070/E-071 三行，三行均 VERIFIED） |
| **R1-F-002** 计数漂移 | `01-EXECUTIVE-SUMMARY.md` §5 的统计同上 | 64 条 13/45/6 → **71 条 24/41/6** |
| **R1-F-003** dead 口径 | 删除「61 道死门禁全部有文档引用」这一**错误断言**（编目里 `dead`=61 的定义就是「既无执行点**也无正文引用**」；有正文引用的是 `orphan_referenced`=22），全部改指 `04-GATE-GOVERNANCE.md` §2 的唯一口径 | 命中 4 处（`01-EXECUTIVE-SUMMARY.md`、`10-KNOWN-DEFECTS.md`、`EVIDENCE-COVERAGE.md` 的 E-014 行、`evidence_index.json` 的 E-014）→ **0 处** |
| **R1-F-004** dead 口径 | `15-DEAD-GATE-DISPOSITION.md` §1/§3：「134 道」不是「无执行点」，是「`stages` 为空」 | 134 道 = 无执行点 **83**（61 dead + 22 orphan）+ CI 可达 **51** |
| **R1-F-005** 自排除规则 | `MANIFEST.json` 显式声明 `MANIFEST.json` / `SHA256SUMS` 的自排除 | R1：18 / 20（未声明）→ R2：**19 / 21（已声明）** |

> **R2 不重新采集 71 条事实。** 除上表 5 项修正外，包内事实仍是 2026-11-23 在
> `cd8cefe623` 上的采集结果；`evidence_index.json` 的 `revision.not_recollected` 字段与
> `MANIFEST.json` 的 `source_commit_semantics` 都明写了这一点。

### 3.2 三套统计：实测值与差额来源

| 统计集 | R1 实测 | R2 实测 | 差额来源 |
|---|---|---|---|
| `EVIDENCE-INDEX.json` | 71 条 24/41/6 | **71 条 24/41/6** | ——（本来就是 SSOT） |
| `EVIDENCE-COVERAGE.md` | 68 条 21/41/6 | **71 条 24/41/6** | R1 的表格**止于 E-068**，缺 E-069/E-070/E-071（3 条，均 VERIFIED）⇒ 24−21=3 |
| `01-EXECUTIVE-SUMMARY.md` §5 | 64 条 13/45/6 | **71 条 24/41/6** | R1 的摘要带的是**更早一版**的分布（13/45/6，合计 64，既非 68 也非 71）⇒ 与 SSOT 的差 = 24−13=11 VERIFIED、41−45=−4 REPORTED |

**R2 实测三套全部为 71 条 / 24-41-6 —— 与目标一致，差额来自上面两条已定位的漂移，
不是靠改数字凑出来的**：两个 markdown 的统计由 `evidence_index.json` **计算**后写入
（`r2-build/gen_r2_package.py`），不是手抄。

### 3.3 `source_commit` 的两个值：为什么并存（显式声明，不合并）

- **包级 `source_commit` = BASE**（`fd849ae205…`）= 本包的**评审对象树**（R64 强制）。
- **逐条 evidence 的 `source_commit` = `cd8cefe623`** = 该条事实**实际被读取时**的采集树。

把逐条也改成 BASE，等于宣称「41 条 REPORTED 与 6 条 NOT_RUN 已在 BASE 上复跑」——
那是本计划要消灭的那类漂移。因此逐条值**原样保留**，两个值在
`EVIDENCE-INDEX.json` 的 `source_commit_semantics` 与 `fact_collection_commit`、
`MANIFEST.json` 的 `source_commit_semantics`、以及
`EVIDENCE-COVERAGE.md` / `01-EXECUTIVE-SUMMARY.md` / `00-CLAUDE-REVIEW-PROMPT.md` 的表头
**显式命名并区分**。⚠️ 这是实施者的判断，若属主认为应统一，属主可推翻（见裁决记录的待裁决项）。

## 4. MANIFEST / SHA256SUMS 的自排除规则（显式声明）

R2 的 `MANIFEST.json` 新增 `self_exclusion` 块：

```json
"self_exclusion": {
  "rule": "MANIFEST.json and SHA256SUMS are NOT listed in MANIFEST.files. MANIFEST.json hashes every content file and is itself hashed into SHA256SUMS; SHA256SUMS hashes nothing and is therefore excluded from its own listing.",
  "excluded_from_manifest_files": ["MANIFEST.json", "SHA256SUMS"],
  "sha256sums_lists": "every file in MANIFEST.files + MANIFEST.json",
  "sha256sums_excludes": ["SHA256SUMS"],
  "file_count_in_manifest_files": 19,
  "file_count_in_zip": 21,
  "r1_note": "R1 (2026-11-23) had 18 / 20 without declaring this difference (R1-F-005)."
}
```

计数口径（R2 实测）：

| 集合 | 文件数 | 内容 |
|---|---|---|
| `MANIFEST.files` | **19** | 17 份正文文档（含 `EVIDENCE-COVERAGE.md`）+ `EVIDENCE-INDEX.json` + `CLAUDE-REVIEW-PACK.md` |
| `package/SHA256SUMS` 行数 | **20** | `MANIFEST.files` 的 19 项 + `MANIFEST.json` 自身 |
| ZIP 内文件数 | **21** | 上一行的 20 项 + `SHA256SUMS` |
| `package/` 磁盘文件数 | **22** | 上一行的 21 项 + `FREEZE.json`（**不入 ZIP**：冻结记录要记录 zip 的哈希，只能后写） |

> R2 相对 R1 的一个额外变化：`EVIDENCE-COVERAGE.md` **移进 `package/`**。R1 里它在冻结范围**之外**，
> 于是三套统计里有一套根本不受冻结保护（R1-F-001 的根因之一）。R2 起它在 `package/` 内，
> 进 `MANIFEST.files`、进 `SHA256SUMS`、进 ZIP、进 `FREEZE.json`。

## 5. 生成步骤（可复算）

```bash
bash …/DSH-EXTERNAL-REVIEW-001/r2-build/build_r2.sh
```

该脚本按顺序做 7 件事，每一步失败即非零退出：

| # | 步骤 | 产物 |
|---|---|---|
| 0 | R1 `package/` 逐文件 sha256 快照 | `/tmp/t9-r1-snapshots/before.txt` |
| 1 | `cp -a` R1 → `sqlrustgo-v4.1.0-r2/`（R1 只读） | R2 目录 |
| 2 | `patch_r2_tools.py`：改 R2 身份 + MANIFEST 自排除块 + `DOC_ORDER` 加 `EVIDENCE-COVERAGE.md` | 副本里的工具 |
| 3 | `gen_r2_package.py`：由 `evidence_index.json` 派生统计、修正 dead 口径、扫 R2 身份 | 包内文档 + SSOT |
| 4 | `patch_r2_dirlevel.py`：把 `RUNBOOK.md` / `OWNER-ACTION-SHEET.md` / `G5-AIOPS…` / `REVIEW-PLAN.md` / `DATA-CLASSIFICATION.md` / `ROUND-1-SUMMARY.md` 指向 R2；把 `EVIDENCE-COVERAGE.md` 移入 `package/` | 目录级操作文档 |
| 5 | `run_r2_generation.sh`：`finalize_package.py` → `build_r2_zip.py` → `freeze_package.py freeze` → `gen_export_approval.py` → `freeze_package.py verify` | `package/`、zip、`FREEZE.json`、批准申请 |
| 6 | `verify_r2_package.py`：A5 #10 / #11 + 包完整性（23 项） | 实测输出 |
| 7 | R1 `package/` 逐文件复核 | `BYTE-IDENTICAL` |

## 6. R2 的实测自校验结果

`python3 verify_r2_package.py` → **23 项全 PASS**（详见 `V410-READINESS-VERDICT.json` 判据 10/11）：

- 三套统计一致：71 条 / 24-41-6（逐项与 SSOT 比对，非相等判断）；
- dead 口径：R2 全文检索「全部有文档引用 / 全部仍有文档引用 / 全部有正文引用」→ **0 命中**；
- 唯一口径陈述在 `04-GATE-GOVERNANCE.md` §2（83 = 61 无引用 + 22 有引用）；
- `MANIFEST.json` 自排除声明与实际计数一致；
- `SHA256SUMS` 20 项逐项复算通过；ZIP 21 项清单 == MANIFEST + 2；
- `FREEZE.json` 的 `zip_sha256` == 实际 zip 哈希；`freeze_package.py verify` → PASS；
- `owner/approvals/` **为空**。

## 7. 签署位：谁签、签什么

| 文件 | 谁写 | 状态 |
|---|---|---|
| `package/MANIFEST.json` | `finalize_package.py`（工具） | `status=DRAFT`、`approval_state=NOT_APPROVED` |
| `package/FREEZE.json` | `freeze_package.py freeze`（工具，`--frozen-by TASK9-EXECUTOR`） | 冻结 R2 内容 |
| `EXPORT-APPROVAL-REQUEST.md` | `gen_export_approval.py`（工具） | 批准栏**留空**（`批准人 | ______________（Owner）`、`"approved_by": null`） |
| `owner/approvals/<review_id>-v2.json` | **只能由属主**用 `record_approval.py record` 生成 | **空 —— 实施者没有、也不会代签** |

> `record_approval.py` 在本次任务中**一次都没有运行**。它写 `owner/approvals/`，
> 那是属主的签署位（C7）。`verify_r2_package.py` 把「`owner/approvals/` 无审批文件」
> 作为一项断言来守。

## 8. 实施者知道的取舍（不是缺陷，是需要属主知道的边界）

1. **R2 没有重新采集事实。** 71 条事实仍是 2026-11-23 的采集结果；R2 只是把它修正到自洽。
   包内关于门禁夹具数、ALPHA 7 道门禁「0 道有可证伪夹具」等陈述，描述的是**采集时点**
   （`cd8cefe623`）的状态；Task 8 新造的 7 道夹具在 `base/v4.1.0-frozen` 上、**不在 BASE 内**。
   外部评审须知：R2 不是 BASE 的重新验收。
2. **R1 的 `FREEZE.json` 有一个工具不产生的字段**（`approval_tool`）—— 冻结记录被手工加过一行。
   R2 的 `FREEZE.json` 是 `freeze_package.py` 的**纯输出**，没有手工改动。
3. **`gen_export_approval.py` 的文件数公式 R1 算错**（`len(rows)+2`，多算 1）。R2 副本里已修正为
   `len(rows)+1`，批准申请现在写的 21 个文件与 ZIP 实际一致。
4. **R2 目录里带 6 个构建脚本**（`gen_r2_package.py` 等）。它们的 docstring/锚点里保留了被删除的
   旧断言字符串（作为替换锚点），因此「R2 全文检索」的口径必须排除这 4 个脚本
   （`verify_r2_package.py` 已显式排除并注明）。**`package/` 内是干净的** —— 判据 11 的对象是包。
5. **DSH 仓库里 R2 与 `r2-build/` 尚未提交**（`git status` 显示为 untracked）。提交与否由控制者决定；
   本次未 push。

## 9. 复算入口

```bash
# 1) 重建 R2（幂等：R1 只读，重跑得到同样的 R2 内容）
bash …/DSH-EXTERNAL-REVIEW-001/r2-build/build_r2.sh

# 2) 单独复核判据 10 / 11 与包完整性
python3 …/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2/verify_r2_package.py

# 3) 复核 R1 未被触碰
shasum -a 256 …/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0/package/FREEZE.json
#   期望 8363343618a6e659b9e94a52d2ba2f16cd5d9c3d9f3eb94dbe7dcfa514b85e36
```
