# PR-5537 TEST_DESIGN — v4.1.0 ALPHA 阶段门禁阻塞项修复

> Issue: #5537 · 分支: `opencode/fix-alpha-gate-blockers` · 基线: `6f26128890`
> 配套: `PR-5537_SPEC.md` / `PR-5537_TEST_PLAN.md` / `PR-5537_TEST_REVIEW.md` / `PR-5537_ACCEPTANCE.md`

---

## 1. 测试设计原则

### 1.1 断言契约，不断言实现

Rust 回归套件断言的是**可观察的行为**（脚本里有没有某个能力、
某个文件有没有某个门控），而不是脚本的内部结构。唯一的例外是
「手写清单必须不存在」—— 那条本身就是契约（清单会静默过期）。

### 1.2 检查代码，不检查散文

`env!("CARGO_TARGET_DIR")` 会出现在**注释里**（helper 的文档注释解释
自己为什么不用它）。朴素的 `contains` 会把那段解释当成违规。

所以断言前先剥离注释：

```rust
fn code_only(text: &str) -> String   // 去掉整行注释与行尾注释
```

### 1.3 `option_env!` 包含 `env!` 作为子串

这是本 PR 实际踩到的坑：`option_env!("CARGO_TARGET_DIR")` 里**含有**
`env!("CARGO_TARGET_DIR")`。朴素的 `contains` 会把**正确**的代码判为违规。

```rust
let bare_env = code.replace("option_env!", "");
assert!(!bare_env.contains("env!(\"CARGO_TARGET_DIR\")"));
```

### 1.4 双向验证 cfg 门控

只验证「无 feature 时测试不存在」是不够的 —— 一个写错的 `#![cfg(...)]`
可能让测试**永远**消失，那比原来更糟（测试从此永不运行，且没人发现）。

所以两个方向都验：
- 无 feature ⇒ `cargo test -p <crate> --no-run` 里没有该 target
- 有 feature ⇒ `Executable tests/<name>.rs` 存在

### 1.5 用真实 gate 脚本做变异

变异测试的 harness 跑的是**真实的 shell 脚本**，不是 mock。变异点用
**内容定位**（不是行号），所以脚本改动后 harness 仍然有效。

---

## 2. 关键实现片段

### 2.1 feature 推导（`check_coverage_v312.sh`）

```bash
derive_crate_features() {
    local crate="$1"
    ...
    python3 - "$crate_dir" <<'PYEOF'
# 找 #[cfg(... feature = "X" ...)]，且紧跟其后的是
#   `pub mod` / `pub use`（模块级公共项 —— 集成测试能 use 的东西）
# 只门控方法的 feature 不算：测试 import 不到它。
PYEOF
}
```

**为什么只认模块级**：`v313_3_profile` 只门控 `binary_storage_v2.rs` 里的
方法（`use_in_place_encoding_for_test` 等），集成测试 import 不到它们，
所以开着它没有意义。早期版本把 `not(feature = "X")` 也算成门控，
导致 16 个文件被**过度门控** —— 那些文件根本不需要这个 feature。

### 2.2 排除清单也自动提取

```bash
excluded_test_names() {
    for file in "${EXCLUDED_TEST_FILES[@]}" "${PERF_BOUND_TEST_FILES[@]}"; do
        python3 - "$file" <<'PYEOF'
# 从 #[test] / #[sqlrustgo_test] 属性后提取函数名
PYEOF
    done
}
```

cargo 没有「排除整个 test target」的选项（只有 package 级 `--exclude`），
而 libtest 的 `--skip` 按**函数名**过滤 —— 所以先提取函数名。

### 2.3 `--skip` 必须放在 `--` 之后

`cargo llvm-cov test [OPTIONS] [TESTNAME] [-- <args>...]` ——
`--skip` 是 **libtest** 参数，放在 `--` 之前会让 cargo 拒绝整个调用。
第一次尝试就踩了这个坑（8 个 crate 全部 exit=1）。

---

## 3. 取舍

| 取舍 | 选择 | 理由 |
|---|---|---|
| feature 清单手写 vs 推导 | **推导** | 手写清单静默过期，失效表现是覆盖率数字悄悄变错 |
| `#![cfg]` vs `#[ignore]` | **`#![cfg]`** | `#[ignore]` 让门禁能在测试从未运行时报 PASS（ADR-008 Policy 2）|
| 并行 vs 串行 | **串行** | 插桩下并行导致同一 checkout 两次测量不同 —— 那不是测量 |
| 排除整个文件 vs 排除单个函数 | **整个文件** | wall-clock 测量在 helper 里，函数级检测看不到 |
| 改门禁语义（测试失败降为 WARN）vs 排除测试 | **排除测试** | 门禁语义是 SSOT 事实；降级会让真回归也被吞 |
| 用 `option_env!` vs 运行时 `std::env::var` | **`option_env!` + 回退** | 编译期已知值更快；回退保证变量缺失时不崩 |

---

## 4. 已知的设计局限（明确声明）

| 局限 | 影响 | 缓解 |
|---|---|---|
| 排除清单是**文件级**手工维护 | 新增崩溃测试文件需手工加入 | 清单里的**函数名**是自动提取的；文件级可由 `check_g14_real_crash.sh` 的清单交叉检查 |
| 推导假设 crate 目录名可从包名推出 | 非标准命名的 crate 会走 fallback 扫描 Cargo.toml | fallback 存在且已实现 |
| `code_only` 不处理字符串里的 `//` | 理论上可能误剥离 | 被检查的文件里没有这种情况；已在代码注释里写明 |
