# PR-870: VTU Merge — 测试设计报告

> **版本**: v1.0
> **日期**: 2026-06-01
> **PR 系列**: PR-870
> **状态**: STUB - 未完成

---

## 1. 测试目标

### 1.1 主要目标

| 目标 | 描述 | 优先级 | 当前状态 |
|------|------|--------|----------|
| T1 | Parser 支持 MERGE 语法 | P0 | ❌ 未实现 |
| T2 | execute_merge 被实际调用 | P0 | ❌ 未实现 |
| T3 | MERGE 正确执行 UPDATE/INSERT/DELETE | P0 | ❌ 未实现 |

### 1.2 次要目标

| 目标 | 描述 | 优先级 | 当前状态 |
|------|------|--------|----------|
| T4 | WHEN MATCHED THEN DELETE | P1 | ❌ 未实现 |
| T5 | 多行 MERGE | P2 | ❌ 未实现 |

---

## 2. 测试用例

### 2.1 T1: Parser MERGE 语法

**目的**: 验证 Parser 能解析 MERGE SQL

**测试用例**:
```sql
MERGE INTO target USING source ON target.id = source.id
WHEN MATCHED THEN UPDATE SET target.val = source.val
WHEN NOT MATCHED THEN INSERT (id, val) VALUES (source.id, source.val)
```

**验收标准**:
```rust
let result = parse("MERGE INTO ...");
assert!(result.is_ok());
let stmt = result.unwrap();
assert!(matches!(stmt, Statement::Merge(_)));
```

### 2.2 T2: execute_merge 调用

**目的**: 验证 execute_merge 被实际调用（不再返回错误）

**测试步骤**:
1. 调用 execute("MERGE INTO ...")
2. 验证返回不是 "MERGE via ExecutionEngine: wired but needs parser support" 错误

**验收标准**:
```rust
let result = engine.execute("MERGE INTO ...");
assert!(!result.is_err() || !result.unwrap_err().to_string().contains("wired but needs parser"));
```

### 2.3 T3: MERGE 基本功能

**目的**: 验证 MERGE 正确执行 UPDATE 和 INSERT

**前置条件**:
- target 表存在，包含 (id=1, val='old')
- source 表存在，包含 (id=1, val='new') 和 (id=2, val='insert')

**测试步骤**:
1. 执行 MERGE：
   ```sql
   MERGE INTO target USING source ON target.id = source.id
   WHEN MATCHED THEN UPDATE SET target.val = source.val
   WHEN NOT MATCHED THEN INSERT (id, val) VALUES (source.id, source.val)
   ```
2. 验证 target 表：
   - id=1 的行 val='new' (UPDATE)
   - id=2 的行 val='insert' (INSERT)

**验收标准**:
```rust
let result = engine.execute("SELECT * FROM target ORDER BY id");
// row 0: [1, "new"]
// row 1: [2, "insert"]
assert_eq!(result.rows[0][1], Value::Text("new".to_string()));
assert_eq!(result.rows[1][1], Value::Text("insert".to_string()));
```

### 2.4 T4: WHEN MATCHED DELETE

**目的**: 验证 WHEN MATCHED THEN DELETE

**测试步骤**:
1. 创建 target 表 (id=1)
2. 创建 source 表 (id=1)
3. 执行 MERGE：
   ```sql
   MERGE INTO target USING source ON target.id = source.id
   WHEN MATCHED THEN DELETE
   ```
4. 验证 target 表为空

**验收标准**:
```rust
let result = engine.execute("SELECT COUNT(*) FROM target");
assert_eq!(result.rows[0][0], Value::Integer(0));
```

### 2.5 T5: 多行 MERGE

**目的**: 验证多行同时 MERGE

**测试步骤**:
1. 创建 target 表 (id=1,2,3)
2. 创建 source 表 (id=1,4)
3. 执行 MERGE
4. 验证：id=1 UPDATE, id=2 DELETE, id=3 不变, id=4 INSERT

---

## 3. 测试位置

### 3.1 建议的测试文件

```
crates/executor/tests/merge_vtu_test.rs  # 扩展现有测试
```

### 3.2 需要的测试函数

```rust
#[test]
fn test_merge_parse() { ... }

#[test]
fn test_merge_execute_called() { ... }

#[test]
fn test_merge_basic_update_insert() { ... }

#[test]
fn test_merge_when_matched_delete() { ... }

#[test]
fn test_merge_multi_row() { ... }

#[test]
fn test_merge_complex_conditions() { ... }
```

---

## 4. 覆盖率目标

### 4.1 代码覆盖率

| 模块 | 当前覆盖率 | 目标 |
|------|-----------|------|
| merge.rs | ~30% | ≥80% |
| local_executor.rs (merge 路径) | 0% | ≥70% |
| parser (MERGE) | 0% | ≥80% |

### 4.2 功能覆盖率

| 功能 | 覆盖状态 |
|------|----------|
| MERGE 解析 | ❌ 未覆盖 |
| WHEN MATCHED UPDATE | ❌ 未覆盖 |
| WHEN MATCHED DELETE | ❌ 未覆盖 |
| WHEN NOT MATCHED INSERT | ❌ 未覆盖 |
| ON 条件表达式 | ❌ 未覆盖 |

---

## 5. 边界测试

### 5.1 边界条件

| 场景 | 输入 | 预期行为 |
|------|------|----------|
| 无匹配行 | MERGE 空 source | target 不变 |
| 多行匹配 | source 有重复 key | 错误或未定义 |
| NULL 条件 | ON target.id = source.id (NULL) | 正确处理 |
| 空表 MERGE | MERGE 空表 | 正常执行 |

### 5.2 异常测试

| 场景 | 预期行为 |
|------|----------|
| 语法错误 | ParseError |
| 不存在的表 | SqlError |
| 类型不匹配 | SqlError |

---

## 6. 测试限制

### 6.1 当前限制

1. **Parser 不支持 MERGE**: 无法解析
2. **execute_merge 未被调用**: 代码只返回错误
3. **无完整执行路径**: MergeExecutor 未正确接入

### 6.2 前置条件

测试需要先完成：
1. Parser 添加 MERGE 语法
2. LocalExecutor 暴露 Arc<Mutex<dyn ExecutionEngine>>
3. execute_merge 实际调用 MergeExecutor

---

## 7. 测试执行

### 7.1 运行命令

```bash
# 运行当前测试（会失败）
cargo test --test merge_vtu_test

# 运行所有 merge 相关测试
cargo test merge
```

### 7.2 预期结果 (Phase 完成后)

```
test test_merge_parse ... ok
test test_merge_execute_called ... ok
test test_merge_basic_update_insert ... ok
test test_merge_when_matched_delete ... ok
test test_merge_multi_row ... ok
6 passed; 0 failed
```

---

*本文档为 PR-870 测试设计报告 v1.0*
