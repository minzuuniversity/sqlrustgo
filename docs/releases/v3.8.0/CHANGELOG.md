# SQLRustGo v3.8.0 Changelog

> **版本**: v3.8.0
> **发布日期**: 2026-06-04
> **分支**: `develop/v3.8.0`
> **前版本**: v3.7.0

---

## v3.8.0 (2026-06-04)

### 重大变更 (Breaking Changes)

| 变更 | 说明 | PR |
|------|------|-----|
| 统一二进制入口 | `sqlrustgo-mysql-server` 整合 serve/exec/repl/bench/gmp/diag 子命令 | #2904 |
| 废弃 sql-cli | `sqlrustgo-sql-cli` 标记 DEPRECATED，请使用 `sqlrustgo-mysql-server repl` | #2905 |
| 废弃 tools | `sqlrustgo-tools` 标记 DEPRECATED，请使用 `sqlrustgo-mysql-server diag` | #2905 |

### 新功能 (Features)

#### 执行引擎
| 功能 | 说明 | PR |
|------|------|-----|
| WAL Recovery Chain | PR-830A~E 完整 WAL 恢复链，22/22 RECOVERY 测试 PASS | #2694, #2704 |
| WAL Lifecycle Controller | PR-830F Checkpoint + Truncation，WAL 磁盘空间管理 | #2697 |
| COM_QUERY AST Routing | PR-800/850 MySQL 协议与 LocalExecutor 统一 | #2696 |
| TransactionalFacade | PR-800F Trait 定义（实现待续） | #2694 |
| CASE WHEN in Aggregates | TPC-H Q8 支持 | #2901 |
| SQL LIKE Pattern | TPC-H Q9 支持 | #2911 |
| Multi-column ON | TPC-H Q9 复合键支持 | #2915 |

#### TPC-H 支持
| Query | 状态 | PR |
|--------|------|-----|
| Q1~Q22 (PART 1/2/3) | 13/22 PASS | #2911, #2915, #2896 |

#### 测试改进
| 改进 | 说明 | PR |
|------|------|-----|
| VTU Predicate/Mutation Pipeline | 22 个新测试 | #2929 |
| E2E Subprocess Test | 15 场景综合测试 | #2919 |
| 38 个 #[ignore] 添加 reason | SPEC-005 G-01 合规 | #2793 |
| 5 个 weak assertion 修复 | SPEC-006 错误上下文 | #2795 |

### Bug 修复
| Bug | 说明 | PR |
|-----|------|-----|
| UPDATE+DELETE mixed transaction | 内存状态错误 | #2792 |
| cross_path test index | Alpha/RC-GA bugfix | #2914 |
| D6 bash 3.2 兼容 | declare -A 替换 | #2914 |

---

## v3.7.0 (2026-04-??)

> 参考 `docs/releases/v3.7.0/`

---

*本文档由 `git log --oneline v3.7.0..HEAD` 自动生成*
