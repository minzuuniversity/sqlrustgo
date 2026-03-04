# Changelog

All notable changes to SQLRustGo will be documented in this file.

<<<<<<< HEAD
## [1.1.0] - 2026-03-03

### Added

- LogicalPlan/PhysicalPlan 分离架构
- ExecutionEngine 插件化接口
- Client-Server 架构支持
- 异步网络层 (Tokio)
- HashJoin 实现 (Inner/Left Join)
- Hash trait for Value (支持 Float NaN 特殊处理)
- 性能基准测试框架 (Criterion)
- 安全审计报告 (SECURITY_AUDIT.md)
- 代码质量审计报告 (CODE_QUALITY_AUDIT.md)
- 发布门禁检查清单 (RELEASE_GATE_CHECKLIST.md)
- API 文档 (API_DOCUMENTATION.md)
- 升级迁移指南 (UPGRADE_GUIDE.md)
- 日志规范 (LOGGING_SPECIFICATION.md)
- 监控规范 (MONITORING_SPECIFICATION.md)
- 健康检查规范 (HEALTH_CHECK_SPECIFICATION.md)
- 性能测试报告 (PERFORMANCE_REPORT.md)
- 成熟度评估报告 (MATURITY_ASSESSMENT.md)

### Changed

- Executor 重构为插件化架构
- 网络层升级为异步模式
- 测试覆盖率提升至 94.18% (目标 ≥90%)
- 代码质量门禁通过 (Clippy/Format)
- 成熟度等级从 L2 升级到 L3 (产品级)

### Fixed

- 修复 Clippy 警告 (11 个错误)
- 修复格式检查问题
- 修复测试代码中的逻辑问题
- 修复文档注释中的 HTML 标签警告

### Security

- 依赖审计通过
- 无高危安全问题
- 无敏感信息泄露
- SQL 注入防护验证

### Documentation

- 添加 v1.1.0 Release Notes
- 更新 CHANGELOG
- 添加门禁检查清单
- 更新 README.md 版本信息
- 添加 CODEOWNERS 文件

## [1.0.0-rc.1] - 2026-02-20

### Bug Fixes

- apply cargo fmt
- remove let chains for Rust 2021 compatibility

### Documentation

- add v1.0.0-rc1 security scan report
- add dependency check results to security report
- add RC branch protection guide
- generate CHANGELOG from v1.0.0-beta.0

### Features

- setup engineering system rules for 2.0 development

## 1.0.0-rc.1

### Bug Fixes

- resolve clippy warnings in aggregate functions
- resolve clippy warnings in executor aggregate functions
- implement FromStr trait for Role to satisfy clippy
- implement FromStr trait for Role to satisfy clippy
- replace unwrap with proper error handling
- update version flow and fix code issues
- 修复代码格式问题
- 修复 clippy 警告
- 修复 clippy 警告

### Documentation

- add L3 upgrade and governance framework
- add architecture governance and plugin design
- create v2.0 planning directory with architecture documents
- add engineering-level design documents for v2.0
- 重组文档目录结构，创建v1.0和v2.0分离结构
- v2.0 目录按子目录分类整理
- 重构 README 为项目索引和演进指南
- 恢复项目故事，人性化重写 README
- 修正文档时间顺序，增加版本专属阅读指南
- 重命名对话记录为 Claude对话记录，强调项目起源
- 项目演进说明增加项目起源章节
- 添加 AI 协作开发教程
- 重组 AI 相关文档到 AI增强软件工程 目录
- 添加高小原对话记录文档
- 更新 AI增强软件工程 README，添加 AI 规划与分析报告索引
- 重写 AI增强软件工程 README，提升宏观视野
- 修正 VERSION_PROMOTION_SOP Beta 阶段定义
- v1.1.0-beta documentation (#31)
- reorganize v1.1.0-beta documentation structure
- establish engineering governance standards for beta phase
- update version roadmap and flow diagram
- update engineering governance for v1.0 freeze
- 移除 RC-1 阶段，简化版本流程
- add comprehensive version planning documentation

### Features

- add complete engineering governance framework
- implement aggregate functions (COUNT/SUM/AVG/MIN/MAX) (#34)
- implement aggregate functions (COUNT/SUM/AVG/MIN/MAX)
- add benchmark framework with Criterion.rs (#40)
- 实现基础认证机制
- 实现基础认证机制

### Refactor

- replace unwrap with explicit error handling (16 fixes) (#29)

### Testing

- network coverage improvement to 90.94% (#33)
- add network integration tests for coverage improvement
- add more network integration tests for 90%+ coverage
- add more aggregate and executor tests
- add executor tests for coverage improvement
- add parser tests for coverage improvement

### Merge

- resolve conflicts with feature/v1.0.0-beta
- resolve conflicts with feature/v1.0.0-beta
- resolve conflicts with feature/v1.0.0-beta

### Style

- fix rustfmt formatting for v1.0.0-beta

## 1.0.0-beta.0

### Documentation

- correct alpha→beta→release→baseline workflow
- add VERSION_PROMOTION_SOP

## 1.0.0-alpha.1

### Documentation

- 添加Alpha版本文档补全计划
- Phase 2 准备工作 - 文档完善
- 添加高小药与李哥聊天记录 (2026-02-18)
- add documentation reading guide and project evolution
- add Alpha to Beta migration workflow plan

### Features

- add network integration tests

### Testing

- 添加network模块集成测试 - Mock TcpStream

## 1.0.0

### Bug Fixes

- 统一项目名称为 sqlrustgo，版本号改为 1.0.0
- 更新项目测试以匹配当前 API
- 修复 .exit 命令 bug 和编译警告

### Documentation

- 添加对话记录文档
- 更新对话记录，添加阶段一实施详情
- 添加实施完成状态和测试统计
- 更新阶段六、七完成状态
- Task 11 完成，更新最终统计
- 添加并行开发协作指南
- 更新协作指南，添加文档位置说明和 GitHub 访问链接
- 更新文档链接指向 baseline 分支
- 添加并行开发协作指南
- enhance README with features and usage
- Task 11 完成，更新最终统计
- 补全对话记录和添加 AI 协作开发指南
- 创建 v1.0.0 版本文档目录
- 添加 v1.0.0 评估改进报告 (教学阶段)
- add v1.0.0 comprehensive evaluation reports
- add comprehensive improvement plan for v1.0.0
- add version evolution plan
- update version plan for v1.0.0 official release

### Features

- 初始化项目，添加设计文档和实施计划
- 初始化 Rust 项目结构 (Task 1)
- 配置 AI 工具链和 CI/CD (Task 2)
- 定义核心类型系统 (Task 3)
- 实现词法分析器 (Task 4)
- 实现语法分析器 (Task 5)
- 实现存储引擎 (Task 6)
- 实现查询执行器 (Task 8)
- 实现 REPL 和 CLI (Task 9)
- 实现事务管理 (Task 9)
- 实现网络协议层 (Task 10)
- 添加集成测试 (Task 11)
- 实现完整的 DML 语句支持 (INSERT/UPDATE/DELETE)
- 完成 DML 操作相关功能
- 实现 B+ Tree 索引持久化和查询优化
- 完善 MySQL 网络协议层

### Miscellaneous Tasks

- 添加 .worktrees 到 gitignore

### Refactor

- 导出 parser 模块
- 导出 executor 模块
- 导出 transaction 模块
- 导出 network 模块，添加 From<io::Error> 实现
- 使用 FileStorage 替代内存 HashMap 实现持久化存储

### Testing

- enhance network and executor test coverage

### Merge

- 解决与 baseline 的冲突

### Release

- promote v1.0.0 evaluation to alpha (#14)

### Style

- 代码格式化修复 (cargo fmt)

<!-- generated by git-cliff -->
=======
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<<<<<<< HEAD
## [Unreleased]

## [1.1.0-draft] - 2026-03-05

### Added

- **Architecture**: LogicalPlan/PhysicalPlan separation for query execution
- **Architecture**: ExecutionEngine trait for pluggable executors
- **Architecture**: Client-Server architecture with async network layer
- **Feature**: HashJoin implementation for efficient join operations
- **Feature**: Connection pool support for multiple clients
- **Feature**: WHERE clause AND/OR logical operators support
- **Feature**: Expression evaluation for BinaryOp (+, -, *, /)
- **Feature**: TEXT column index support (hash-based)
- **Testing**: Performance benchmark framework with Criterion
- **Testing**: Test coverage improved to 90.66%

### Changed

- **Refactor**: Replaced unwrap with proper error propagation in executor
- **Refactor**: Improved error handling with SqlResult<T>
- **Docs**: Updated gate checklist with correct branch workflow
- **Docs**: Reorganized teaching materials (student/TA separation)

### Fixed

- **Fix**: Clippy warnings resolved (zero warnings)
- **Fix**: Rust 2021 compatibility (let chains syntax)
- **Fix**: Code formatting issues

### Security

- **Audit**: Dependency audit passed
- **Audit**: No sensitive information leakage

### Documentation

- **New**: DeepSeek evaluation report
- **New**: Improvement plan for v1.1.0-draft
- **New**: AI-CLI collaboration notice
- **New**: v1.3.0 version plan with observability track
- **New**: 2.0 architecture design documents
- **New**: Distributed interface design (3.0 preview)
- **New**: Teaching practice materials (student/TA handbooks)

## [1.0.0] - 2026-02-22

### Added

- **Core**: SQL parser supporting SELECT, INSERT, UPDATE, DELETE
- **Core**: B+ tree storage engine
- **Core**: Transaction support with WAL
- **Core**: Basic query execution
- **Testing**: Unit test framework
- **Docs**: Initial documentation

### Changed

- Initial release

---

## Version History

| Version | Date | Maturity | Notes |
|---------|------|----------|-------|
| v1.1.0-draft | 2026-03-05 | L3 Draft | Architecture upgrade, Clippy passed |
| v1.0.0 | 2026-02-22 | L3 GA | Initial release |

---

## Roadmap

- **v1.1.0**: Draft → Alpha → Beta → RC → GA
- **v1.2.0**: Performance optimization (vectorization, CBO)
- **v1.3.0**: Enterprise features (observability, MVCC)
- **v2.0**: Distributed architecture

---

*This changelog is maintained by yinglichina8848*
=======
## [1.0.0] - 2026-02-16

### Added

- **SQL-92 子集支持**
  - SELECT 查询语句
  - INSERT 数据插入
  - UPDATE 数据更新
  - DELETE 数据删除
  - CREATE TABLE 表创建
  - DROP TABLE 表删除

- **存储引擎**
  - Buffer Pool 实现 (LRU 缓存)
  - FileStorage 持久化存储
  - 页面管理 (Page)

- **B+ Tree 索引**
  - 索引持久化
  - 查询优化

- **事务支持**
  - Write-Ahead Log (WAL)
  - TransactionManager
  - BEGIN/COMMIT/ROLLBACK

- **网络协议**
  - MySQL 风格协议实现
  - TCP 服务器/客户端
  - 数据包编解码

- **REPL 交互界面**
  - 命令行交互
  - SQL 语句执行

- **测试覆盖**
  - 集成测试
  - 项目结构测试
  - CI 配置验证

### Changed

- 使用 Rust Edition 2024
- 集成 Tokio 异步运行时
- 重构模块导出结构

### Fixed

- 修复 .exit 命令 bug
- 修复编译警告
- 统一项目名称为 sqlrustgo

---

## [0.0.1] - 2026-02-13

### Added

- 项目初始化
- 设计文档
- 实施计划
- AI 工具链配置
- 基础项目结构
>>>>>>> origin/main
>>>>>>> origin/develop-v1.2.0
