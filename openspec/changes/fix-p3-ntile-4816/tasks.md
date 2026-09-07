# Tasks — Issue #4816: NTILE(N) balanced bucketing

## 1. 现有实现审查

- [ ] 1.1 在 `src/executor/window.rs` 找到 NTILE 现有实现,记录当前 bug。
- [ ] 1.2 验证 NTILE 在 `WindowFunction` 枚举中的 signature (`Ntile(i64)` 或 `Ntile(usize)`)。
- [ ] 1.3 验证 parser 已经正确解析 `NTILE(n)` 中的 arg。
- [ ] 1.4 写一个 anchor failing 测试:10 行 / 4 桶 → 期望
      `(1,1,1,2,2,2,3,3,4,4)`,确认 FAIL。

## 2. NTILE 算法实现

- [ ] 2.1 在 `src/executor/window.rs` 的 NTILE 分支,替换为 balanced 算法:
      ```rust
      let n = partition_rows.len();
      let k = n_buckets as usize;
      if k == 0 { return Err(...); }
      if n == 0 { return Ok(vec![]); }
      
      let rows_per_big_bucket = (n + k - 1) / k;
      let big_bucket_count = n % k;
      
      let mut assignments = Vec::with_capacity(n);
      let mut idx = 0;
      for b in 0..k {
          let size = if b < big_bucket_count {
              rows_per_big_bucket
          } else {
              rows_per_big_bucket - 1
          };
          let actual_size = (n - idx).min(size);
          for _ in 0..actual_size {
              assignments.push(b + 1);  // 1-based
              idx += 1;
          }
          if idx >= n { break; }
      }
      ```
- [ ] 2.2 验证算法:
      - 10 行 / 4 桶 → 大桶 = 2,每桶 3 行;小桶 = 2,每桶 2 行
        → 分配 = (1,1,1,2,2,2,3,3,4,4) ✓
      - 8 行 / 4 桶 → remainder = 0,每桶 2 行
        → (1,1,2,2,3,3,4,4) ✓
      - 5 行 / 4 桶 → 大桶 = 1 (3 行), 小桶 = 3 (2 行)
        → (1,1,1,2,2,3,3,4,4)? Wait, 5 行 + 1 大桶(3) + 3 小桶(各2)
        = 3 + 6 = 9 > 5。重新计算:rows_per_big_bucket = ceil(5/4) = 2,
        big_bucket_count = 1 → 大桶 2 行,小桶 4 桶 (剩 3 桶) 各 1 行
        → 分配 = (1,1,2,3,4) ✓
      - 3 行 / 5 桶 → big_bucket_count = 3, 大桶 3 个 (各 ceil(3/5)=1 行)
        → 分配 = (1,2,3) ✓

## 3. Window Executor 集成

- [ ] 3.1 验证 NTILE 已经在 `evaluate_window_function` 的 dispatch 表中。
- [ ] 3.2 验证 partition 边界正确处理(每 partition 独立计算)。
- [ ] 3.3 验证 ORDER BY 顺序与 NTILE bucket 一致(同一 partition 内)。
- [ ] 3.4 验证 NULLS FIRST/LAST 影响 bucket 编号(因为 idx 包含 NULLs)。

## 4. Tests (8 tests)

- [ ] 4.1 `ntile_4_of_10_rows` — issue anchor。
- [ ] 4.2 `ntile_exact_division` — 8 / 4。
- [ ] 4.3 `ntile_more_buckets_than_rows` — 3 / 5。
- [ ] 4.4 `ntile_single_bucket` — 5 / 1 = (1,1,1,1,1)。
- [ ] 4.5 `ntile_with_partition` — `PARTITION BY cat` 时每 partition 独立。
- [ ] 4.6 `ntile_zero_buckets_error` — `NTILE(0)` 报错。
- [ ] 4.7 `ntile_negative_buckets_error` — `NTILE(-1)` 报错。
- [ ] 4.8 `ntile_large_n` — 100 / 7,验证 remainder。

## 5. Verification

- [ ] 5.1 `cargo build --all-features` clean。
- [ ] 5.2 `cargo test --test p3_ntile_4816_test` 8/8 PASS。
- [ ] 5.3 `cargo test --all-features --lib` no regression。
- [ ] 5.4 `cargo test --test window_function_test --test partition_test` no regression。
- [ ] 5.5 `openspec validate fix-p3-ntile-4816 --strict` valid。

## 6. Commit + Memory

- [ ] 6.1 Commit message:
      `fix(P3 / #4816): NTILE bucketing is balanced (big buckets first)`。
- [ ] 6.2 在 `memory/` 新增 `p3-ntile-4816.md` 记录 balanced algorithm
      与 PostgreSQL 语义对照。
- [ ] 6.3 `openspec archive fix-p3-ntile-4816` after merge。

## 7. Out of Scope

- [ ] 7.1 WIDTH_BUCKET — v3.14。
- [ ] 7.2 NTILE with frames — 不支持。
- [ ] 7.3 NTILE inside aggregate — 不支持。