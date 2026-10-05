# GMP-Platform consumer gate — 2026-10-05（#4943 / #4873 / V400-10）

**RESULT: 8 passed, 0 failed, 0 skipped** — 全部实跑，无一项 SKIP。

- SQLRustGo: `96946478d` (`develop/v4.1.0`)
- GMP-Platform: `7143c72` (`fix/hnsw-embed-dim`) at `/Volumes/workspace/dev/GMP-Platform`
- ollama: `http://192.168.0.250:11434`（含 `nomic-embed-text`，即门禁默认 embed 模型）

```bash
GMP_SMOKE_DEEP=1 \
GMP_OLLAMA_URL=http://192.168.0.250:11434 \
GMP_408_LIMIT=5 \
bash scripts/gate/check_gmp_consumer.sh
```

| Status | Check | Detail |
|---|---|---|
| PASS | `pr-207-resolved` | GMP-Platform 内 `67fc628`（2026-09-11）已合入；#4873 的「self-approval pending」是查错仓库 |
| PASS | `compile-gmp-api` | 对当前 sqlrustgo crates `cargo check` |
| PASS | `compile-gmp-auth` | 同上 |
| PASS | `compile-gmp-audit-db` | 同上 |
| PASS | `sqlrustgo-linkage` | 4 个 GMP crate path-depend 本仓；`sqlrustgo-storage` 仍可构建 |
| PASS | `smoke-408` | `gmp-eval` 实跑 408 场景中的 5 条 |
| PASS | `webui` | `npm run test` → Tests 11 passed |
| PASS | `webui-typecheck` | `npm run typecheck` clean |

## 关于「PR #207 查无此物」

`GET /pulls/207` 在本实例返回 404 —— 该 PR 不在 SQLRustGo 仓库，而在 GMP-Platform：

```
67fc628 Merge PR#207: M5 sqlrustgo-graph migration
         (resolve cypher_engine.rs conflict)          ai <ai@z440>
```

经 PR #210 forward 到 `develop/v1.5.0`，由 `develop/v1.6.0` 继续演进（`crates/gmp-storage/src/cypher_engine.rs` 在位）。

## 前一版证据的 FAIL 已排除

同日另一份证据（基线 `6c52ce1317`）记录 `sqlrustgo-linkage` FAIL：

```
sqlrustgo-storage failed to build: error: this file contains an unclosed delimiter
```

**该 FAIL 无法复现。** 在当前 HEAD 实跑 `cargo build -p sqlrustgo-storage`：

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 16.80s
```

那份证据指向的是提交前的未提交中间态，不是仓库缺陷。

## 两处此前的 SKIP 已消除

### 408：阻断是 ollama 端点，不是「缺 LLM 凭据」

早前记 SKIP，理由「需 LLM 凭据」——**这是错的**。`gmp-eval` 的评估后端是本地 ollama（`--ollama-url`，默认 `http://localhost:11434`）加一个 embedding 模型，**根本不用 LLM API key**。本机未装 ollama，但 192.168.0.250 上有可用的。

门禁据此改为**探测 ollama 端点**（`curl $ollama_url/api/tags`）而非查环境变量。

### WebUI：build 此前一直是坏的

`npm run build` 第一步是 `tsc --noEmit`，而它因 `vite.config.ts` 使用 `node:path` / `__dirname` 却无 node 类型而失败——**vite 根本没被调用**。单个 tsconfig 同时管浏览器（`src`）与 Node（`vite.config.ts`）两套环境，无法兼顾。

已在 GMP-Platform 修（commit `1795ebb`，基于 `95bf551`，仅动 `web/` 4 文件）：按 vite 官方惯例拆 `tsconfig.node.json`、加 `@types/node`、`typecheck` 串行校验两个 project。

验证时**移除了 `@types/node`** 以排除「本机恰好装了所以过」的假阳性：

```
基线: vite.config.ts(3,18): error TS2307: Cannot find module 'node:path'
      vite.config.ts(20,25): error TS2304: Cannot find name '__dirname'.
修复后: ✓ typecheck clean   ✓ build 51 modules (gzip 63.08 kB)   ✓ test 11 passed
```

## 附带完成（同一轮）

`npm run lint` 此前也不可能通过——ESLint 9 移除了 `--ext`，且仓库**从无任何 ESLint 配置文件**。已建 flat config（含类型感知规则）并修掉它暴露的 10 个真实代码问题（未处理 Promise、`onClick` 挂 async 函数、与 enum 比裸字符串等）。现 `lint` 0 problems。

## 遗留：408 检索层全零（数据资产问题，非门禁缺陷）

5/5 passed，但检索指标全零：

```
Avg Recall@K: 0.667 | Avg MRR: 0.000 | Avg Precision@5: 0.000 | Avg nDCG@10: 0.000
[init] HNSW index: 0 vectors loaded (dim=768)
FTS passed: 5 | Vector fallback: 0 | LIKE fallback used: 10
```

完整根因见 GMP-Platform `docs/eval/408_RETRIEVAL_ZERO_ROOTCAUSE.md`：

1. `gmp-eval` 从不调 `index_embeddings`，HNSW 索引必然为空
2. **仓库语料与 408 场景集不同源** —— 场景期望 `"审计计划"`（字符串），语料存 `12443`（数字），`doc_id` 命中率 **0/830**
3. 附带缺陷 `embedding_dim` 硬编码 1024（`nomic-embed-text` 实为 768）—— 已修为按模型查表，未知模型直接拒绝运行

门禁验的是 408 流程能否跑通，不是检索质量，故判定 PASS。语料对齐需确认权威源，属数据决策。
