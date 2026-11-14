# v4.1.0 audit manifest 红项处置清单（草案，2026-11-14）

> **性质**：执行者整理的**处置提案**，不是裁定。每条最终路径需 Owner 按看板 P0-6 / IL-5 / #5596 决定。
> **实测命令**：`bash scripts/gate/check_audit_manifest.sh`（本分支 HEAD `1b36ef4dcc`）
> **实测结果**：rc=1，**10 条红**；其中 #8（`20261105-lb1-006-mcode`）已由 PR #5787
> 修复（canonical_run_id），修复后实测 **9 条红**（见 §1 第 8 行）。本会话 4 条新 run
> （002/3-001/3b-001/2-001）全部 `verify OK`。
> **门禁定位（重要）**：`GATE_CATALOG.jsonl` 把本门禁登记为 `orphan_referenced`
> （`stages: []`，`needs-execution-point`）——它**不在任何阶段的 required_gates 里**，
> 因此不会机械阻断 GA；但它是审计证据完整性的唯一检查器，`#5596` 未处置前无法挂执行点。
> 不得用「不在 required_gates」来淡化：GA 证据集要求“归档 run 可复核”。
> **回滚纪律**：任何处置都保留原始失败记录，不把历史改写成 PASS。

---

## 1. 红项清单

| # | run / 路径 | 实测失败原因 | 当前分类 | 建议处置（提案） | 需谁裁定 |
|---|---|---|---|---|---|
| 1 | `audit/v4.1.0/iaia410-a06-r1-20261011T073325Z-01df92a5/` | 目录无 `manifest.yaml`（只有 `findings/`） | 无清单 | ① 按 A01 契约重跑 A06 并补 manifest；或 ② 标 `retired` 并移入 `archive/`（保留原始 findings + 说明）。**不得**补一个事后 manifest 冒充当时证据 | Owner（#5596） |
| 2 | `iaia410-5112-evidence-20261015` | 6/6 条命令缺 `cmd` ⇒ exit_code 声明不可复核 | hollow | 重跑并记录命令行；若原始命令无法还原，标 `retired` + 说明，不得引用其 exit_code | Owner |
| 3 | `iaia410-a01-20261011T062229Z-609a8e81` | 7/7 条命令缺 `cmd` | hollow | 同上；A01 是契约本身，优先重跑 | Owner |
| 4 | `iaia410-a02-20261011T070228Z-609a8e81` | 8/8 条命令缺 `cmd` | hollow | 同上 | Owner |
| 5 | `iaia410-a03-20261011T070845Z-omp` | `commands` 为空 | empty | ① 重跑；或 ② 退役。空记录不得被任何结论引用 | Owner |
| 6 | `iaia410-a04-20261011T180837Z-609a8e81` | 5/5 条命令缺 `cmd` | hollow | 同上 | Owner |
| 7 | `iaia410-a05-20261012T004948Z-bf48818c` | 9/9 条命令缺 `cmd` | hollow | 同上 | Owner |
| 8 | `20261105-lb1-006-mcode` | ~~`evidence_hash mismatch`~~ **已修复**：canonical 含 run_id；该 run 改名后 hash 永久假红 | 已修（PR #5787） | `evidence_hash_of` 支持 `canonical_run_id`；manifest 补记原名，**不改** results/artifacts/frozen_commit/evidence_hash；夹具 M7/M8 承重；实测 verify OK，红项 10→9 | 待 PR 合并 |
| 9 | `20261112-ab08-core3` | manual-wrapper（非 `audit_run.py`）：无 `commands` | manual-wrapper | 二选一：① 用 `audit_run.py` 重跑同一 bench 集；② 认定它**不是 audit run**，移出 `runs/` 到 `audit/v4.1.0/attachments/` 并在原位置留指针 + 原始 artifacts sha256 | Owner（#5596） |
| 10 | `20261112-soak-ab003` | manual-wrapper：无 `results/evidence_hash`（`run it first`） | manual-wrapper | 同 #9；SOAK 报告本身可作为附件，但不得冒充 audit run | Owner |

## 2. 判据边界

- 本清单**不**删除任何历史目录，也**不**修改任何已提交 manifest。
- “重跑”必须在冻结 commit 上执行，并产出新的 run id + `evidence_hash`；旧 run 保留。
- “退役/移出 runs/”必须有 Owner 书面记录（Issue 评论或 STAGE.yaml 裁决块），且在原路径留
  说明文件（`RETIRED.md`）指向新位置与原因。
- `20261105-lb1-006-mcode` 的 hash mismatch 与 #5698 的 canonical 缺陷高度相关：先确认
  `evidence_hash_of` 是否排除 `run_id`，再决定是工具修复还是证据失效。

## 3. 与阶段的关系

| 阶段 | 是否被本门禁机械阻断 | 说明 |
|---|---|---|
| ALPHA | 否（非 required_gates） | 但看板 P0-6 / IL-5 仍 OPEN |
| BETA | 否（非 required_gates） | 证据集要求仍适用 |
| RC | 否（非 required_gates） | RC 的 `check_full_gate_verification` / `check_rc_ga_gate` 可能间接引用证据完整性 |
| GA | 否（非 required_gates） | GA 证据集明确要求“归档 run 可复核”；本清单是前置清理，不是可选装饰 |

## 4. 下一步（最小动作）

1. ~~先处理 #8（canonical 缺陷）~~ **已由 PR #5787 修复并实测转绿**（红项 10→9）。
2. 再处理 #9/#10（manual-wrapper）：它们数量少、边界清楚，适合先定为
   “附件而非 audit run”，给其余 hollow 记录一个可复制的处置模板。
3. 最后处理 #2–#7（IAIA-410 hollow 记录）：逐条重跑成本高，建议按 A01–A06 的
   当前工具链一次性重跑，产出新 run，旧记录标 `superseded` 并保留。
4. 全部处置完再给 `check_audit_manifest.sh` 挂执行点（看板 P0-2/P0-6 同源）。
