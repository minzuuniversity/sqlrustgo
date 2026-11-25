# PR-GPT-R1 ACCEPTANCE — 门禁执行证据

> **source_agent**: DSH-B (opencode / MiniMax-M3) · **source_run**: SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123
> **timestamp**: 2026-11-23T11:30:00+08:00 · **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见 `docs/releases/v4.1.0/refactoring/gpt-r1/manifest.json#files_digest`
> **conflict_resolution**: 每条结论标注 VERIFIED / REPORTED / NOT_RUN / UNKNOWN / STALE；未实跑的一律 NOT_RUN。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T11:30:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/evidence/gpt-r1/

## 1. 提交与推送（VERIFIED）

```
$ git rev-parse HEAD
fb84bc5096b419b9827c82a5f2f159679ac0aad1
$ git ls-remote origin dsh-b/v410-g0-baseline-reconcile-20261123
fb84bc5096b419b9827c82a5f2f159679ac0aad1	refs/heads/dsh-b/v410-g0-baseline-reconcile-20261123
```

未推送任何受保护分支；未合并任何 PR。

## 2. 行为契约套件（VERIFIED）

```
$ cargo test --test gate_behavior_contract_test
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.00s
```

## 3. 鉴别力验证（VERIFIED —— 关键）

同一套件跑在**整改前**的 `scripts/gate/`（`8cbde5b6d3`）上：

```
test result: FAILED. 12 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.66s
```

失败的 8 条 ⇔ 8 条已证明的行为缺陷：

```
architecture_freeze_detects_real_ad002_bypass
architecture_freeze_missing_detector_is_incomplete
drift_gate_zero_scanned_scripts_must_not_pass
evidence_binding_heading_marker_does_not_excuse_claim
evidence_binding_negated_statement_is_not_a_violation
anti_fabrication_stale_sqlancer_report_must_not_be_certified
drift_gate_missing_baseline_must_not_pass
gate_test_integrity_missing_test_reference_must_not_pass
```

## 4. 受影响门禁在权威基线上的实跑（VERIFIED）

| 门禁 | rc | 输出要点 |
|---|---|---|
| `check_drift_not_pass.sh` | **0** | `✅ PASS — P14 satisfied`（14 条历史 finding 仍由基线吸收，语义未变） |
| `check_gate_test_integrity.sh` | **0** | `48 gate tests, 0 NEW #[ignore]`（2 条 ADR-008 基线豁免） |
| `check_architecture_freeze.sh` | **0** | `PASS: 5/5`，A7-1 = `PASS (0 production storage DML bypass in server/network crates)` |
| `check_gate_script_refs.sh` | **0** | 未受影响 |
| `check_evidence_binding.sh v4.1.0` | **1** | `PASS=268 WARN=303 FAIL=29 UNVERIFIED=28`（整改前 `PASS=286 WARN=301 FAIL=85 UNVERIFIED=84`） |

**未制造新红。** evidence-binding 的 FAIL 从 85 降到 29：56 条是假阳性
（45 条明确否认通过声明的句子 + 11 条定义/规格句），新增 1 条真实违规，
**29 条既有发现的语义未被本轮改动**。

## 5. 交付包校验（VERIFIED，服务端复算）

```
$ cd docs/releases/v4.1.0/refactoring/gpt-r1 && shasum -a 256 -c SHA256SUMS
BASELINE-RECONCILIATION.md: OK
CONTROL-PLANE-GAP-MATRIX.json: OK
EXECUTOR-CAPABILITY-PLAN.md: OK
EXTERNAL-REVIEW-PACK-V2-CANDIDATE.md: OK
HARNESS-P0-FINDINGS.md: OK
TEST-FACTORY-RECOVERY-PLAN.md: OK
manifest.json: OK
```

`files_digest` 独立复算 = `2bc8ac623bf2c29d52b95350cc1349c7a81c81a0a45444e87da01cd8774bee0e`（与 `manifest.json` 一致）。
**服务端复算同样通过**（从 Gitea raw 接口取回 7 份文件后重算）。

## 6. 工具链（VERIFIED）

| 检查 | 结果 |
|---|---|
| `cargo fmt --check --all` | 本 PR 新增的 Rust 文件 **0 违规**（权威 tip 既有 **14 文件** fmt 红，与本 PR 无关，未擅自扩大改动面去修） |
| `cargo clippy --all-features --all-targets` | 本 PR 新增文件 **0 warning**（权威 tip 既有 6 warning，全在 `tests/integration/test_factory/ca01_*.rs`） |
| `bash -n` 全部 5 个改动的门禁脚本 | 语法 OK |

## 7. NOT_RUN（明确列出，不当作已验证）

| # | 项 | 原因 |
|---|---|---|
| 1 | `check_anti_fabrication.sh` 在权威树上的**全量**实跑 | 需要 `cargo build --release -p sqlancer` 与 test-runner 产物；本轮用桩环境证明并修复了其过期证据通道 |
| 2 | 75 场景矩阵重跑 | 大规模执行未获授权；矩阵证据绑定的量化结论见 `TEST-FACTORY-RECOVERY-PLAN.md` §1.1 |
| 3 | 30 个试点用例 | 同上 |
| 4 | 全量 `cargo test --all-features` | 与本 PR 无关的既有失败会混淆归因 |
| 5 | 交接包 `files_digest` 独立复算 | 规范化口径未定义 |
| 6 | Swift 能力项 B / C / E | 执行准入与控制面准入未就绪 |
| 7 | Swift 能力项 D 的**分段**复核 | 整份 33 KB diff 尺度实测不可用（§2.3 D2）；分段路径未实测 |

## 8. 边界声明

本 PR 未触碰：生产库、生产服务器、门禁阈值、STAGE 配置、任何 `tests/baseline/*.json`、
任何 `crates/**` 生产源码、任何**既有**文档、任何受保护分支。
未通过模型切换绕过 LOCAL_ONLY（Swift 只做只读调用，发送内容仅为仓库内公开源码与
本 PR 自产 diff；无任何仓库内容被发往仓库外）。
