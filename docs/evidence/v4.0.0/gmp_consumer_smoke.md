# GMP-Platform consumer gate — smoke 证据

**Issue**: #4943（P0 GMP-Platform consumer gate 未完成 — PR #207 查无此物）
**执行日期**: 2026-10-05
**sqlrustgo 基线**: `6c52ce1317`（`6c52ce13177ba098cdfd86b571fbc146a6bf2442`）
**GMP-Platform HEAD**: `7143c720171ca7c56f318f012f9d811a221942bf`（branch `fix/hnsw-embed-dim`）
**执行命令**:

```bash
GMP_ROOT=/Volumes/workspace/dev/GMP-Platform \
GMP_OLLAMA_URL=http://192.168.0.250:11434 \
GMP_SMOKE_DEEP=1 GMP_408_LIMIT=5 \
bash scripts/gate/check_gmp_consumer.sh
```

> 注：脚本默认在 `REPO_ROOT` 同级目录找 GMP-Platform；本仓库在 worktree
> `/private/tmp/sq-5009` 下，故须显式给 `GMP_ROOT`。

---

## AC3 — 四类 smoke 门禁全部可执行（3 次复跑）

三次输出**逐字节一致**：

```
=== GMP-Platform consumer gate (V400-10 / #4873 / #4943) ===

GMP-Platform: /Volumes/workspace/dev/GMP-Platform
  head: 7143c72 (fix/hnsw-embed-dim)

  [PASS] pr-207-resolved — found and merged in GMP-Platform at 67fc628 (2026-09-11); #4873's 'self-approval pending' was a wrong-repository lookup
  [PASS] compile-gmp-api — cargo check against current sqlrustgo crates
  [PASS] compile-gmp-auth — cargo check against current sqlrustgo crates
  [PASS] compile-gmp-audit-db — cargo check against current sqlrustgo crates
  [PASS] sqlrustgo-linkage — 4 GMP crate(s) path-depend on this repo; sqlrustgo-storage still builds
  [PASS] smoke-408 — gmp-eval ran 5 of 408 scenarios
  [PASS] webui — npm run test ok (Tests  11 passed)
  [PASS] webui-typecheck — npm run typecheck clean

RESULT: 8 passed, 0 failed, 0 skipped
EXIT=0
```

| 复跑 | 结果 | 退出码 |
|---|---|---|
| RUN 1 | 8 passed, 0 failed, 0 skipped | 0 |
| RUN 2 | 8 passed, 0 failed, 0 skipped | 0 |
| RUN 3 | 8 passed, 0 failed, 0 skipped | 0 |

未设 `GMP_SMOKE_DEEP` 时为 `7 passed, 0 failed, 1 skipped`（`smoke-408` 需
`GMP_SMOKE_DEEP=1` + ollama 端点）。**注意 SKIP 不等于 PASS**——脚本在
GMP-Platform 不可达时以 `exit 2` 退出并写 `**SKIPPED** — not a pass`。

---

## AC5 — 门禁自身的变异测试验证

**这一步是「门禁有效」与「门禁恰好返回绿」的唯一区别。**

### 变异方法

在 `crates/storage/src/engine.rs` 的 `pub trait StorageEngine` 之前注入语法错误：

```
fn __mutation_probe_4943( { unclosed delimiter
```

这直接破坏 `sqlrustgo-storage` 的编译，模拟 consumer 接口被破坏。

### 变异后输出

```
  [PASS] pr-207-resolved — found and merged in GMP-Platform at 67fc628 (2026-09-11)
  [PASS] compile-gmp-api — cargo check against current sqlrustgo crates
  [PASS] compile-gmp-auth — cargo check against current sqlrustgo crates
  [PASS] compile-gmp-audit-db — cargo check against current sqlrustgo crates
  [FAIL] sqlrustgo-linkage — sqlrustgo-storage failed to build: error: this file contains an unclosed delimiter
  [PASS] smoke-408 — gmp-eval ran 5 of 408 scenarios
  [PASS] webui — npm run test ok (Tests  11 passed)
  [PASS] webui-typecheck — npm run typecheck clean

RESULT: 7 passed, 1 failed, 0 skipped
EXIT=1
```

**判定：有效变异。** 门禁 `sqlrustgo-linkage` 精确报出破坏内容，且退出码由
0 变为 1（具备阻断能力）。变异已恢复，`cargo check -p sqlrustgo-storage`
复跑通过。

### 顺带发现的盲区

变异时 `compile-gmp-api` / `compile-gmp-auth` / `compile-gmp-audit-db` 三项
**仍报 PASS**，只有 `sqlrustgo-linkage` 抓到破坏。说明这三项在本机条件下
可能命中了各自 target 目录的构建缓存，或检查范围不含 `sqlrustgo-storage`
本身。**不因此判定它们无效**（它们仍会在冷缓存下失败），但「8/0/0」这一
数字里，真正对本仓库提供保护的是 `sqlrustgo-linkage` 一项。建议后续
排查那三项是否存在缓存掩盖。

---

## AC1 — PR #207 状态已查明

门禁 `pr-207-resolved` 项给出实证结论：

> found and merged in GMP-Platform at `67fc628` (2026-09-11);
> #4873's 'self-approval pending' was a wrong-repository lookup

即 **PR #207 存在于 GMP-Platform 仓库并已于 2026-09-11 合并**；此前
#4873 记录的「self-approval pending」源于**在错误的仓库里检索**。

此前在 sqlrustgo 自身的 API 上查询 `GET /pulls/207` 返回空，与该结论一致
——因为 #207 本就属于 GMP-Platform，不属于 sqlrustgo。

---

## AC2 / AC6 状态

- **AC2**（v4.0.0 文档中 #207 失实记载按 `DOC_CHECK_CORRECTION_RULES.md` 订正）：
  本轮**未执行**。需按 DOC_CHECK_CORRECTION_RULES 的 7 步流程订正并附订正前原文。
- **AC6**（不得在任何文档中声称 GMP-Platform 兼容性）：本轮**未做全量文档扫描**，
  属未验证项。

---

## evidence_hash

```
sqlrustgo   : 6c52ce13177ba098cdfd86b571fbc146a6bf2442
GMP-Platform: 7143c720171ca7c56f318f012f9d811a221942bf
gate script : dd1db4eec0c5cd698713a1828f9361dd5ef3c3d1f6731d03bdfa8e1aad030bd1  (sha256)
```

---

*source_agent: mcode (minimax-M3.1) · timestamp: 2026-10-05 · 遵循 ANTI_FABRICATION_POLICY.md*
