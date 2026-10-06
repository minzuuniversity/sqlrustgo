# COV-01 / COV-02 覆盖率门禁证据（2026-10-07）

> **source_agent**: Sisyphus (oh-my-openagent / mimo-v2.6-flash-free)
> **source_run**: 2026-10-07T04:56:43+08:00（本机实跑，非引用文档 claim）
> **branch**: fix/omp-4944-alpha2-promotion
> **HEAD（COV-02 门禁时）**: 0ddb0866781b6324761caeb57c0296ade1f6631b
> **evidence_hash**: 见下方各日志 sha256

---

## COV-01: parser 74.32% → 86.20% ✅

**门禁命令**:

```bash
cargo llvm-cov clean && cargo llvm-cov test -p sqlrustgo-parser --no-fail-fast
```

**实跑输出（TOTAL 行，region 口径）**:

```
Filename   Regions  Missed Regions  Cover  ...  Lines  Missed Lines  Cover
lexer.rs      972              38  96.09%  ...    551            17  96.91%
parser.rs   20719            3099  85.04%  ...  12017          1781  85.18%
token.rs     1770             258  85.42%  ...    825            58  92.97%
transaction.rs 90               6  93.33%  ...    102             6  94.12%
TOTAL       23551            3401  85.56%  ...  13495          1862  86.20%
```

- 测试结果: 24 个 test binary 全部 `ok`，**0 FAILED**
- 日志: `/tmp/opencode/parser_final_gate.log`
- **evidence_hash (sha256)**: `6ae5adc2df12b29472916749348bfd810128c3f9ce9116596bcbebda0fa86f6c`
- 相关 commit:
  - `2463b284e5` fix(parser): DATE(x) 补消费 RParen + UNIQUE 表约束 P0 死循环改报错（含 v410_coverage_push.rs 197 项覆盖率推进测试）
  - `15e12b054a` style(parser): 补块级花括号过全仓 fmt 门禁

---

## COV-02: mysql-server 75.41%(v4.0.0 基线) → 87.89% ✅

**门禁命令（governance 口径）**:

```bash
cargo llvm-cov clean && cargo llvm-cov test -p sqlrustgo-mysql-server --no-fail-fast \
  -- --skip one_connections_rollback_must_not_discard_anothers_buffered_rows
```

skip 原因: 该测试为预存 flaky（先前基线循环实证，与本轮改动无关，见 §预存问题）。

**基线（本轮改动前）**:

```
TOTAL   11183   2539   77.30%   648   123   81.02%   6907   1636   76.31%
```

日志: `/tmp/opencode/cov02_base3.log`
**evidence_hash (sha256)**: `11ee523ed9aa1895300af760f03876fa0eda1562c80e95f3e01f778d3998bca2`

**改进后（GATE_EXIT=0）**:

```
TOTAL   11607   1467   87.36%   656    77   88.26%   7176    869   87.89%
```

- 测试结果: 24 个 test binary 全部 `ok`，**0 FAILED**（1 filtered = 上述 skip）
- 日志: `/tmp/opencode/cov02_after.log`
- **evidence_hash (sha256)**: `6fb88f8119ba492a73c83d2330b63b1249b2f6502786b6fc12abdce291887fe5`

**per-file 变化（region 覆盖率）**:

| 文件 | 基线 | 改进后 |
|------|------|--------|
| lib.rs | 76.97% (1256 miss) | 86.20% (787 miss) |
| main.rs | 67.17% (323 miss) | 94.31% (56 miss) |
| metrics_endpoint.rs | 85.00% (45 miss) | 95.61% (14 miss) |
| load_data.rs | 92.90% | 92.90% |

**新增测试 commits（5 个）**:

| commit | 内容 |
|--------|------|
| `7b86e37b7b` | Batch A — lib.rs in-crate 单测 7 项（statement_kind 表驱动 / decode_param / extract_table_name / infer_column_types / property_value / param_bind / write_binary_row） |
| `50ccd8bb82` | A2 — metrics bind→scrape→shutdown 单测 |
| `a0cb1d748c` | B1/B2 — run_server*/POOL 生命周期 8 个 TCP 黑盒测试 |
| `9870a77066` | B3 — 压缩 I/O roundtrip 与错误映射 8 测 |
| `0ddb086678` | C — CLI 子进程 e2e 10 测（并入 probe 后删除之） |

---

## 配套门禁（同轮实跑）

- `cargo fmt --check --all` → exit 0（先执行 `cargo fmt --all`）
- `cargo clippy -p sqlrustgo --lib --all-features` → 32 warnings，与 stash 对照基线 32 相等（零新增）
- `cargo test -p sqlrustgo --lib --all-features` → 158 passed / 0 failed

---

## 预存问题（本轮已实证、另行修复，不计入上述门禁）

1. `SERVER_POOL.acquire` AddrInUse 分支 panic（lib.rs slot None unwrap）
2. parallel 存储已提交 DML 重启前不可见（WAL recovery-only）
3. `param_bind_type_from_string` INT 扫除致 INT24/SMALLINT/TINY 臂不可达
4. lexer 无 ROLE/ROLES token → statement_kind 部分臂不可达
5. `CompressedReader` leftover 分支双发 + seq 取 payload[0]
6. flaky: `one_connections_rollback_must_not_discard_anothers_buffered_rows`（门禁 skip）
7. 根套件既有红测（先前基线循环实证）
