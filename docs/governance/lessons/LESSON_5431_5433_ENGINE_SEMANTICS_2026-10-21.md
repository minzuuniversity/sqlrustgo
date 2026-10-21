# LESSON — 迭代 5 批次 1（#5431 / #5432 / #5433）：引擎语义修复的三条归因纪律

> **日期**：2026-10-21 · **分支**：`fix/5431-5433-engine-semantics` · **基线**：`005b6c18f31a`
> **PR**：`http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/5470`
> **证据**：`docs/releases/v4.1.0/evidence/ENGINE_SEMANTICS_5431_5433_2026-10-21.md`
> 本文件由 mavis (MiniMax Code) 编写。所有结论均附可复现命令与真实输出。

---

## Lesson-38：归因的第一个问题不是「哪一行错了」，而是「**它跑了几行项目代码**」

`#5431` 的标题写的是「REPLACE INTO 的 4 个语义用例失败（引擎缺陷）」。
实测下来，**引擎一行都没跑过**：

```rust
// tests/integration/dml/replace_test.rs（改造前）
let out = run_sql("…").unwrap_or_default();     // ← Err 被吞成空串
assert!(out.contains("apple"), "should have apple: {}", out);
```

`run_sql` 的第一件事是读一个环境变量：

```rust
.current_dir(
    std::env::var("SQLRUSTGO_REPO_ROOT")
        .map_err(|_| "SQLRUSTGO_REPO_ROOT env var must be set to repo root".to_string())?,
)
```

而这个变量**全仓 + CI 0 处设置**：

```console
$ grep -rl "SQLRUSTGO_REPO_ROOT" tests/
tests/integration/dml/replace_test.rs          # 唯一命中
$ grep -rn "SQLRUSTGO_REPO_ROOT" --include=*.yml --include=*.yaml --include=*.sh --include=*.toml .
（0 命中）
```

⇒ 每次 CI 运行，4 个用例都拿到**空串**，断言报的是**症状**
（`should have apple: `）而不是**原因**（`env var must be set`）。
⇒ 带上手工设置的环境变量，4 个用例**全绿**（35.72 s）⇒ 引擎没有缺陷。

**纪律**：一个用例失败时，先回答三个问题，再看报错行：

1. **它跑了几行项目代码？**（0 行 ⇒ 先查前置条件，不要读业务代码）
2. **失败信息指向的是症状还是原因？**（`should have apple: ` 是症状；
   `env var must be set` 才是原因）
3. **报错里有没有「被吞掉的错误」？**（`.unwrap_or_default()` / `|| true` /
   `if let Ok(..) = ..` / `let _ = ..` —— 这四种写法会把错误变成沉默）

**可复用检查**：

```bash
grep -rn "unwrap_or_default\|unwrap_or(\"\"\|unwrap_or_else(|_| \"\")" tests/ | head
grep -rn "std::env::var(" tests/ | while read -r l; do
  v=$(printf '%s' "$l" | grep -o 'var("[A-Z_]*")' | head -1 | tr -d '"' | sed 's/var(//;s/)//')
  [ -n "$v" ] && printf '%-40s 在 CI/脚本里被设置过吗: ' "$v" \
    && (grep -rn "$v" .gitea/ scripts/ *.toml 2>/dev/null | head -1 || echo "❌ 没有")
done
```

---

## Lesson-39：**`debug_assert!` 会把一种缺陷在 debug 里炸出来，在 release 里变成沉默**

`#5432` 的宽度检查是 `crates/executor/src/executor.rs:70` 的 **`debug_assert!`**：

```rust
pub fn with_columns(columns: Vec<ResultColumn>, rows: Vec<Vec<Value>>, affected_rows: usize) -> Self {
    debug_assert!(
        columns.is_empty() || rows.iter().all(|r| r.len() == columns.len()),
        "row width {} does not match the {} declared columns", ...,
    );
    Self { rows, affected_rows, columns }     // ← 行宽原样保留，不截断、不补齐
}
```

⇒ **debug 构建**：panic ⇒ CI 变红 ⇒ 看得见。
⇒ **release 构建**：断言被编译掉，行**原样**存下 ⇒ **结果集声明的列数与行的实际宽度不一致**。

**基线 `005b6c18f3` 的 release 构建实测**：

```console
$ ./target/release/sqlrustgo-mysql-server exec "… SELECT count(*) FROM orders GROUP BY cust; …"
col_1 | col_2 | col_3 | col_4
Text("bob")   | Integer(1) | Text("bob")   | Integer(200)
Text("alice") | Integer(2) | Text("alice") | Integer(100)
Text("carol") | Integer(1) | Text("carol") | Integer(300)
(3 rows)
```

⇒ **声明 1 列，打印 4 个 cell；第一个 cell 是分组键，计数被挤到第 2 位。**
⇒ **不 panic。读第一列拿到的是分组值，不是计数。**

**纪律**：

- **CI 只跑 debug ⇒ 生产形态不可见。** 任何「debug 下会 panic」的断言，
  都要问一句「release 下它会做什么」—— 答案通常是「什么都不做」，
  而**什么都不做**在数据流的下游就是**错答案**。
- **release 的实测成本很低**：只改一个文件时
  `cargo build --release -p <binary>` 只需 ~1m20s（依赖已缓存），
  却能拿到**生产形态**的证据。**凡是断言是 `debug_assert!` 的，都该跑一次 release。**
- **宽度 / 长度 / 计数这类「形状契约」应该是显式的运行时校验**，
  而不是 `debug_assert!`：`debug_assert!` 只保护**开发期**，
  形状契约在**生产期**同样需要成立。

**可复用检查**：

```bash
# 找出所有 debug_assert：它们在 release 里是不存在的
grep -rn "debug_assert" src/ crates/*/src/ | head -30
# 对每一条问：release 下这行代码被跳过后，调用方会拿到什么？
```

---

## Lesson-40：同一个缺陷的**孪生副本**要靠「找同一段手写常量」来找

`#5433` 的视图不可见，根因是 `MemoryStorage::list_views` 剥的是手写前缀
`"{db}\0"`，而 `scoped_key` 用的是 `DB_SEP = '\u{1}'`：

```rust
const DB_SEP: char = '\u{1}';                                  // 行 968
pub(crate) fn scoped_key(db: &str, table: &str) -> String {
    format!("{}{}{}", db.to_lowercase(), DB_SEP, table.to_lowercase())
}
```

**关键发现**：`#5350` **已经修过同一处缺陷** —— 在 `FileStorage::list_views` 里，
而且**注释里把原因写清楚了**：

```rust
// crates/storage/src/file_storage.rs:6894-6897
// #5350: this stripped a hard-coded `"{db}\0"` prefix while
// `scoped_key` joins with `DB_SEP` (`\u{1}`), so the strip never
// matched and `list_views` always returned an empty list. Split on
// the real separator instead of hand-spelling it.
```

⇒ **#5350 修了 `FileStorage`，漏了 `MemoryStorage`。**
⇒ 失败是**静默**的：`CREATE VIEW` 成功、`has_view` / `get_view` 都找得到视图，
只有 introspection 列表是空的 —— **没有任何一个 API 报错**。

**纪律**：

- 修一个「同族缺陷」时，**先把同族全部找出来**：搜**那段手写常量**
  （不是搜函数名）—— `grep -rn 'format!("{}\\\\0"' crates/ src/`。
- **同一个分隔符在同一个模块里出现两次，就一定会漂移一次。**
  正确做法是**只留一个拼装点**（`scoped_key` / `split_scoped_key`），
  让第二个地方**不可能写错**。
- 找「同族缺陷」的最快路径是**搜注释**：写清楚了根因的注释会告诉你
  「还有谁没改」。

**可复用检查**：

```bash
# 同一个模块里，键的拼装与键的拆分是否都走同一个 helper？
grep -rn 'scoped_key\|split_scoped_key' crates/storage/src/ | head
# 手写的分隔符字面量（漂移高发点）
grep -rn 'format!("{}\\\\\0"\|format!("{}\\\\u{1}"' crates/ src/ | head
```

---

## 附：本轮形成的可复用检查

| 工具 / 做法 | 作用 | 复现命令 |
|---|---|---|
| **两轮全量回归的差分** | 找出**残留态**缺陷（同一份代码，第一轮绿、第二轮红） | 同一 worktree 连跑两次 `cargo test --no-fail-fast --all-features`，对比失败目标集合 |
| **基线对照（回退源文件，不必新开 worktree）** | 判定「这个红是不是本 PR 引入」的**唯一硬证据** | `git checkout <base> -- <changed files>` → 跑同一目标 → `cp` 回来 → 重跑 |
| **release 形态实测** | 拿到生产构建的行为（`debug_assert!` 背后） | `git checkout <base> -- <file> && cargo build --release -p <bin>` → 跑 → 恢复并重建 |
| **「它跑了几行项目代码」体检** | 区分「代码红」与「前置条件没满足」 | `grep -rn "unwrap_or_default\|unwrap_or(\"\")" tests/` + `grep -rn "std::env::var(" tests/` 逐个查谁设置 |
| **外部依赖体检** | 找出「测试依赖一个没人设置的环境变量 / 绝对路径 / 共享目录」 | 对每个 `std::env::var("X")`，在 `.gitea/` `scripts/` `*.toml` 里查 `X` 是否有设置点 |
| **共享默认目录体检** | 找出「测试拉起子进程时不传 `--data-dir` ⇒ 共用 `/tmp/sqlrustgo-data`」 | `grep -rn -A6 '\.arg("serve")' tests/ \| grep -c '\-\-data-dir'` 与 `.arg("serve")` 出现次数对比 |
| **绝对耗时阈值体检** | 找出「无判别力的时序断言」（与 #5398 同族） | `grep -rn "Duration::from_millis\|Duration::from_secs" tests/ \| grep -i "assert\|elapsed"` |
| **跑测试后的工作区体检** | 防止测试副产物被当成自己的改动提交 | `cargo test … && git status --short` → 逐条判断归属 → `git checkout -- <被改写的目录>` |
| **注释与实现漂移体检** | 找出「注释说会 lowercase，代码没有」这类 | 读每个 `normalize_*` / `parse_*` 的注释，逐条核对代码 |

> 特别提醒最后一条：**跑完全量测试后一定要 `git status`**。
> 本轮跑完一轮全量，`docs/releases/v4.1.0/evidence/a04-5226/wire/` 下 **81 个**
> 已入库证据文件被测试改写（其中若干被写成 `ENGINE_UNAVAILABLE: …`，
> 即**真实证据被占位文本覆盖**）—— 已全部还原（#5389 跟踪根因）。

---

*本文件由 mavis (MiniMax Code) 编写于迭代 5 批次 1。所有结论均附可复现命令与真实输出；
未实测的判断一律标注为"待验证"。*
