## Context

Issue #4670: TRUNCATE/TRUNC parse error + MD5/SHA1 返回空。两个独立的小改动，各自只需改一处文件。

## Goals / Non-Goals

**Goals:**
- TRUNCATE(expr, digits) 和 TRUNC(expr, digits) 正确解析并返回截断结果
- MD5(string) 返回 32 字符十六进制 MD5 哈希
- SHA1(string) 返回 40 字符十六进制 SHA-1 哈希

**Non-Goals:**
- SHA2 (已有框架但返回空，本次不修)
- TRUNCATE 的负数digits行为（按 std::f64::trunc 语义即可）

## Decisions

### 1. TRUNCATE/TRUNC: parser 直接识别为函数

当前 parser 把 `TRUNCATE`/`TRUNC` 当未知关键字处理。最简修复：在 `parse_function_call` 的关键字识别分支加入这两个 token。

实现：`f64::trunc(x * 10^digits) / 10^digits`（处理负数digits）。

### 2. MD5/SHA1: executor 调用已有 crate

`Cargo.toml` 已有 `md-5` (md5) 和 `sha1` crate。executor 的 `eval_function` 分支中已有 MD5/SHA1 case，但 return Null。需要填入真实计算逻辑。

MD5: `md5::compute(input).to_hex()` → 32 字符
SHA1: `sha1::Sha1::digest(input).to_hex()` → 40 字符

## Risks / Trade-offs

- SHA2 的 SHA2-256/384/512 同样返回空，但 SHA2 需要 extra 参数（算法名+位数），放在下次修
