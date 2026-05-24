# E7 — GMP-Platform v1.2.0 GA 发布

> **类型**: Episode
> **日期**: 2026-05-24
> **结果**: ✅ GA 发布成功

## 事件时间线

| 时间 | 事件 |
|------|------|
| 2026-05-24 03:09 | Z6G4 Hermes Agent 开始执行存储层 trait 抽象 (Plan D) |
| 2026-05-24 上午 | PR #109 (Plan D) 合入 develop/v1.2.0 |
| 2026-05-24 中午 | PR #111 (完全移除 rusqlite) 合入 |
| 2026-05-24 12:38 | PR #114 (dead code) 合入 |
| 2026-05-24 12:46 | PR #115 (clippy too_many_arguments) 合入 |
| 2026-05-24 12:55 | PR #116 (cargo fmt) 合入 |
| 2026-05-24 12:56 | git tag `v1.2.0` + Gitea Release 发布 |

## 关键决策

### 1. Plan D: 存储层 trait 抽象

**问题**: `FourChannelEngine::new` 期望 `StorageEngine`，但 `sqlrustgo_storage` 导出不同类型。

**解决方案**:
- 定义 `GmpStorage` trait（9 个方法）
- `StorageEngine` 实现 `GmpStorage` trait
- 支持 `Arc<dyn GmpStorage>` 动态分发
- VectorChannel / FtsChannel / FourChannelEngine 全部改为 `Arc<dyn GmpStorage>`

### 2. 移除 rusqlite 依赖

**问题**: gmp-cli 依赖 rusqlite，但用户明确拒绝此依赖。

**解决方案**:
- 完全移除 rusqlite from Cargo.toml
- Populate 命令（Layer 0 population）替换为 `anyhow::bail!` stub
- 切换到 sqlrustgo_storage 统一存储

### 3. gmp-cli Populate block 清理

**问题**: Populate block 包含 `use gmp_llm::LlmClient` 等未使用导入，导致 clippy 失败。

**解决方案**:
- 删除整个 Populate body（358 行）
- 替换为 6 行 bail! stub
- 字段名加 `_` 前缀消除 unused variable 警告

## 门禁结果

```
L1-Beta Gate: 14/14 PASSED ✅
L1-RC Gate: 10/10 PASSED ✅
Scenario: 96.08% pass rate
```

## 提交记录

| Commit | PR | 描述 |
|--------|-----|------|
| cbeea44 | #109 | feat(gmp-storage): 存储层 trait 抽象 (Plan D) |
| eab953e | #111 | refactor: 完全移除 rusqlite 依赖 |
| f5eee7f | #114 | fix: 移除 dead code warnings |
| 8f63d50 | #115 | fix: add clippy allow for too_many_arguments |
| fae5ae7 | #116 | style: auto-format code (cargo fmt) |

## 技术细节

### GmpStorage Trait (9 个方法)

```rust
pub trait GmpStorage: Send + Sync {
    fn insert(&self, ...) -> Result<()>;
    fn scan(&self, ...) -> Result<Vec<Record>>;
    fn search_fts(&self, ...) -> Result<Vec<SearchResult>>;
    fn search_vectors(&self, ...) -> Result<Vec<SearchResult>>;
    fn get_document_by_id(&self, ...) -> Result<Option<Document>>;
    fn get_conn(&self) -> Arc<Mutex<dyn ConnectionPool>>;
    fn reload(&self, data_dir: PathBuf) -> Result<()>;
    fn in_memory(&self) -> Result<Self>
    where Self: Sized;
}
```

### 架构图

```
┌─────────────────────────────────────┐
│           gmp-cli / gmp-api         │
├─────────────────────────────────────┤
│         FourChannelEngine            │
│   (Arc<dyn GmpStorage>)             │
├──────────┬──────────┬───────────────┤
│ VectorCh │ FtsChannel│ RuleChannel  │
├──────────┴──────────┴───────────────┤
│        StorageEngine                │
│   (impl GmpStorage)                 │
├─────────────────────────────────────┤
│    sqlrustgo_storage (统一存储)      │
│  RDBMS + Vector + FTS + Graph      │
└─────────────────────────────────────┘
```

## 经验教训

1. **大规模重构前先建分支**: Plan D trait 抽象在独立分支 `feature/plan-d-storage-trait-v1` 开发，避免破坏 main
2. **subagent 结果需验证**: Z6G4 Hermes 修复的文件内容需逐项确认
3. **门禁脚本加 timeout**: `cargo test --all-features` 集成测试需 SQLRustGo server，门禁应加 timeout 防止挂起
4. **populated stub 清理**: 替换函数 body 时，brace 计数需处理 string 内容中的 `}` 字符

## 下一步

- v1.3.0 开发分支: `develop/v1.3.0`
- MQTT 设备集成
- 向量索引生成流水线
- cargo audit 安全审计

## 相关页面

- [[F13-gmp-platform-v120-release]] — v1.2.0 发布状态
- [[F12-gmp-platform-version]] — 历史版本
- [[sqlrustgo/v3.4.0 Overview]] — SQLRustGo v3.4.0 GA
