# SQLRustGo v4.0.0 — GMP-Platform Consumer Requirements

> **版本**: v4.0.0 draft
> **日期**: 2026-09-11
> **来源**: GMP-Platform `develop/v1.5.0`, `feat/issue-187-185`, `feat/eval-408-ui`, `feat/release-v1.6.0`
> **用途**: 将 GMP-Platform 对 SQLRustGo 的已知要求和新增要求转成 v4.0.0 可执行验收项

---

## 1. 判定原则

SQLRustGo v4.0.0 不能只证明自身 SQL/vector/graph 能力,还必须证明 GMP-Platform 作为下游项目可以稳定使用这些能力。本文档是 consumer contract:

- 每个要求必须能追溯到 GMP-Platform 的运行路径、报告或发布摘要。
- 每个要求必须映射到 SQLRustGo v4.0.0 work package 或显式标为非 SQLRustGo 责任。
- GA 前不能以单次编译通过替代 GMP-Platform 回归证据。
- 如果 GMP-Platform 侧通过客户端规避了 SQLRustGo 缺陷,该缺陷仍需进入本表并标注处理决策。

---

## 2. 要求矩阵

| ID | GMP-Platform 要求 | SQLRustGo 责任 | v4.0.0 处理 | 验收证据 |
|---|---|---|---|---|
| GMP-C01 | `gmp-storage`, `gmp-server`, `gmp-graph`, `gmp-kg` 可统一指向同一 SQLRustGo worktree 编译 | crate path/API 稳定 | V400-10 | `cargo build -p gmp-storage -p gmp-server` 0 error |
| GMP-C02 | `sqlrustgo-graph` 支持 GMP Cypher read path 和 GraphStore write path | graph API + WAL/snapshot | V400-03, V400-04 | GMP `cypher_engine` 8/8 pass; graph WAL recovery pass |
| GMP-C03 | `sqlrustgo-rag` 中文 tokenizer + inverted index 可支撑 GMP FTS/RAG | RAG API 稳定/CJK | V400-08, V400-10 | GMP search/rag smoke non-empty; CJK query regression |
| GMP-C04 | 12,531 个 bge-m3 1024d embedding 可加载、可检索、启动不退化 | vector storage/index/rebuild | V400-01, V400-02, V400-09 | embedding count 12,531; rebuild < 60s; query p95 recorded |
| GMP-C05 | 408 场景回归维持高精度 | SQL/vector/graph/RAG 综合 | V400-10 | GMP 408 run >= 99.0% 或失败归因为语料缺失 |
| GMP-C06 | 408 场景 LLM rerank 支持 OpenAI-compatible endpoint | SQLRustGo 不托管 LLM;需确保 DB path 不阻塞 | V400-10 boundary | report records LLM endpoint, failures separated from DB failures |
| GMP-C07 | WebUI/REST/API stats: docs/chunks/embeddings 计数稳定 | MySQL wire + table scan/query correctness | V400-10 | `/api/stats` docs ~=1760, chunks ~=12531, embeddings ~=12531 |
| GMP-C08 | 上传 markdown/txt 后立即可搜 | transaction + index refresh | V400-02, V400-05, V400-10 | upload fixture -> search fixture hit without server restart |
| GMP-C09 | CJK tokenization/LIKE/SUBSTRING 不因 UTF-8 边界 panic | string functions + CJK safety | WP-A, WP-B, GMP-CJK gate | CJK LIKE/SUBSTRING tests, no char-boundary panic |
| GMP-C10 | Batch-record compliance audit 可在 DB/RAG degraded mode 下运行 | DB calls must timeout/fail predictably | V400-07, V400-10 | audit no-rag/no-llm + rag mode both documented |
| GMP-C11 | Audit hash chain / Part 11 后续需要 WAL 层支持 | audit event WAL + tamper evidence | V400-05, V400-07 | hash-chain tamper fail-closed test |
| GMP-C12 | gmp-server 高 CPU / contention 不应拖垮审计系统 | wire/load stability, backpressure | V400-09 | mixed workload CPU/latency report; no runaway process |
| GMP-C13 | Explicit transaction semantics 不得破坏 multi-model atomicity | transaction correctness | WP-E, V400-05 | #4847/#4626 regressions + cross-model rollback tests |
| GMP-C14 | Schema migration 支持下游升级 | `ALTER TABLE RENAME COLUMN` | WP-F | #4848 regression pass |
| GMP-C15 | CHAR/TEXT comparison 与 MySQL/SQLite compatible subset 一致 | type/comparison correctness | WP-G | #4846 regression pass |

---

## 3. 新增 v4.0.0 工作包

### V400-10 — GMP-Platform consumer regression

**Phase**: 1 entry, 2/3 repeat, GA blocking

**Owner**: release engineering + GMP integration

**输入基线**:

- GMP-Platform `develop/v1.5.0` for 408 / RAG / REST baseline
- GMP-Platform `feat/release-v1.6.0` or successor for audit / WebUI parity / eval-408 UI

**任务**:

1. 建立可重复 consumer worktree,统一指向当前 SQLRustGo `develop/v4.0.0` checkout。
2. 跑最小编译矩阵: `gmp-storage`, `gmp-server`, `gmp-graph`, `gmp-kg`。
3. 跑 GMP storage unit subset: Cypher, RAG, vector load/rebuild, CJK text regression。
4. 跑 REST/WebUI smoke: `/healthz`, `/api/stats`, `/api/search`, upload-then-search。
5. 跑 408 回归: 记录 pass rate、失败归因、p50/p95、DB/LLM 分离。
6. 跑 audit system degraded-mode smoke: no-rag/no-llm, rag mode, timeout behavior。
7. 生成 `docs/releases/v4.0.0/evidence/gmp-platform-consumer/summary.md`。

**Exit evidence**:

- `GMP_PLATFORM_INTEGRATION_VERIFICATION.md` 更新到最新 GMP commit。
- 408 regression report,失败项分为 SQLRustGo bug / GMP bug / corpus gap / LLM backend。
- WebUI parity 说明包含 Track 0/A/B/C 的覆盖边界。
- 未通过项必须进入 v4.0.0 issue 或明确 scope exclusion。

---

## 4. 当前缺口

| 缺口 | 影响 | 处理 |
|---|---|---|
| 现有 `GMP_PLATFORM_INTEGRATION_VERIFICATION.md` 停留在 2026-09-09 / GMP `c3f479f` | 未覆盖 v1.6 audit/UI/eval-408 要求 | Phase 0 刷新 |
| `TEST_PLAN.md` 只有 V400-G1..G10 粗粒度 gate | 不能证明每个 legacy issue 或 GMP 要求都有测试 | 本轮已扩展测试映射 |
| LLM/embedding 服务稳定性属于外部依赖 | 容易把 Ollama/llama.cpp failure 误判为 DB failure | 408 报告必须分离 DB 和 LLM backend |
| CJK 字符串边界问题已在 GMP 安装文档中作为规避项出现 | SQLRustGo 字符串函数可能仍不安全 | 加入 GMP-CJK gate |
| gmp-server contention / CPU spike 已在 GMP v1.6 open work 中出现 | 影响生产化 SOAK | 加入 V400-09 workload 和 V400-10 smoke |

---

## 5. GA 前禁止声明

在 V400-10 未通过前,不得声明:

- SQLRustGo v4.0.0 已满足 GMP-Platform v1.5/v1.6 全量需求。
- 408 场景高通过率是 SQLRustGo 单方能力,除非报告已分离 corpus / LLM / UI / DB 责任。
- WebUI/REST/audit path 已 production-ready,除非 Track 0/A/B/C 覆盖边界已记录。
- CJK 文本路径安全,除非 CJK LIKE/SUBSTRING/identifier regression 已通过。
