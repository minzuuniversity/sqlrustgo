# PR-GPT-R1 TEST REVIEW — 独立审核记录

> **source_agent**: DSH-B (opencode / MiniMax-M3) · **source_run**: SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123
> **timestamp**: 2026-11-23T11:30:00+08:00 · **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见 `docs/releases/v4.1.0/refactoring/gpt-r1/manifest.json#files_digest`
> **conflict_resolution**: 执行者不自签（ADR-014 / `docs/releases/v4.1.0/STAGE.yaml:221-223`）。本文件如实标注：**独立终审尚未执行**。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T11:30:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/evidence/gpt-r1/

## 1. 独立性声明（逐项，不含糊）

| 维度 | 状态 | 依据 |
|---|---|---|
| 过程独立 | ❌ **未满足** | 本轮全部取证、修复、验证都由 DSH-B 一个执行者完成 |
| 血统独立 | ❌ **未满足** | 执行侧为 MiniMax-M3（`minimax-cn-coding-plan`）；**只读复核侧**为 Swift-27B（250:8021，本地 llama.cpp）。两者**不是**异血统终审 —— Swift 在本轮只做只读探针，未参与任何写入路径的判断 |
| Oracle 独立 | ✅ 满足 | 所有断言都绑定**外部可核对的工件**：权威 tip 的 SHA、门禁的真实退出码、源码行号、Issue 标题、SPEC-011 条文。没有任何一条断言依赖模型的自我评价 |
| 属主终审 | ❌ **未执行** | Owner / Codex Reviewer 尚未签字 |

**结论：本包不构成「已通过独立审核」。** 它是一份**自证 + 自审**的交付，等独立复核者接手。

## 2. 自审发现（执行者查自己的问题，已修）

| # | 发现 | 处置 |
|---|---|---|
| 1 | 初稿在 `check_drift_not_pass.sh` 里引用了 `#5913` 作为本缺陷的编号 | **#5913 是 BinaryTableStorageV2 桩静默成功，与本缺陷无关** —— 已在提交 `94e04d2507` 中删掉该错误引用，改为描述性表述 |
| 2 | 交付包把「门禁文件数」统一写成 217、「文档数」写成 424 | 三个口径被混用（顶层 196 / 递归 217 / 漂移门禁管辖 194；`.md` 254 / git 追踪全类型 424）—— 已在提交 `fb84bc5096` 中逐处订正 |
| 3 | 初稿的 `evidence_hash` 写了未经计算的占位串 | **那是伪造** —— 改为指向同目录 `manifest.json#files_digest`（真实计算值） |
| 4 | `CONTROL-PLANE-GAP-MATRIX.json` 里 `probe_ci_roots.py` 的路径写成 `audit/v44.1.0/...` | 已订正为 `audit/v4.1.0/...` |
| 5 | G2 矩阵里引用了子代理的结论而未独立复核 | 已**独立复核三条关键结论**（`check_audit_manifest.sh` 无执行点、`openclaw_endpoints.rs` 未被声明为 module、`audit_run.py` 的 `cmd_run` 无「已封存」守卫），全部属实 |
| 6 | 初稿的 A7-1 修复用朴素的 `grep -E 'storage\.(insert|update|delete\|scan)'` | 会把**未编译**的孤儿文件（`crates/server/src/openclaw_endpoints.rs`）与**字符串字面量**（`crates/storage/src/vtu_guard.rs:240` 的诊断文案）判成违规 —— 改为独立实现 `architecture_freeze_bypass_detector.py`，只扫 cargo 真正编译的源文件、排除测试代码与字面量 |

## 3. 请独立复核者重点检查的 7 点

1. **缺基线 ⇒ FAIL** 是否与 #5113 对兄弟门禁的既有修法口径一致；`--init-baseline` 的语义是否可接受。
2. **`check_evidence_binding.sh` 的 FAIL 85 → 29** 是否可接受 —— 56 条消失（45 条否认句 + 11 条定义/规格句），1 条新增（`P15_CLAIM_RUN_BINDING` 的标题行）。剩余 29 条既有发现的**语义未被本轮改动**，但「什么算声明 vs 什么算定义」是属主裁定项。
3. **A7-1 的判据换源**是否真的符合 SPEC-011 §3.1/§3.3（原文要求 `storage\.(insert|update|delete|scan)`），以及「只扫 cargo 编译产物」这一收紧是否过度（可能漏掉用 `include!`/构建脚本注入的文件）。
4. **ADVISORY 不阻断**是否与 SPEC-011 §5 的风险裁定一致（SPEC 说真发现 bypass 时保持 INFO / 非阻断）。
5. **sqlancer 每次必跑**给 CI 增加的耗时是否可接受（`--duration 5` + 可能的一次 release 构建）。
6. `scripts/gate/architecture_freeze_bypass_detector.py` 的「编译产物」判定是**自实现的启发式**（读 crate root 的 `mod` 声明），**不是** cargo 的解析结果 —— 是否有更权威的做法（例如 `cargo metadata` 的 target 列表，或 `rustc --emit=dep-info`）。
7. 交付包的 `manifest.json` 里 `final_head_sha: null`、`status: CANDIDATE_NOT_ISSUED` —— 是否应由 dsh-ops 在签发时补齐。

## 4. 明确未做、也没声称做过的事

- 未推进 STAGE、未关闭任何 Issue、未合并任何 PR、未启动 SOAK。
- 未改任何 `crates/**` 生产源码、任何门禁阈值、任何 `tests/baseline/*.json`。
- 未改任何**既有**文档（交接包、发布线文档、台账）。
- 未声称 75 场景矩阵已刷新（未重跑）。
- 未声称 Swift 具备 B/C/D/E 能力（B/C/E 未跑；D 在整份 diff 尺度实测不可用）。
