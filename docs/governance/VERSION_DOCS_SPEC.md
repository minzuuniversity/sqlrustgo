# VERSION_DOCS_SPEC — 版本文档规范

> **版本**: 1.0
> **创建日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro
> **适用范围**: v1.0.0+

---

## 一、目的

定义 GMP 内审检索系统每个发布版本必须遵守的文档标准。

---

## 二、元数据要求

每个版本文档头部必须包含以下元数据：

```markdown
> **版本**: X.Y.Z
> **更新日期**: YYYY-MM-DD
> **维护人**: <用户名>
> **阶段状态**: Draft | Alpha | Beta | RC | GA
```

---

## 三、命名规范

| 文档类型 | 文件名 | 位置 |
|----------|--------|------|
| 开发计划 | `DEVELOPMENT_PLAN.md` | `docs/releases/vX.Y.Z/` |
| 测试计划 | `TEST_PLAN.md` | `docs/releases/vX.Y.Z/` |
| 门禁检查清单 | `RELEASE_GATE_CHECKLIST.md` | `docs/releases/vX.Y.Z/` |
| 发布说明 | `RELEASE_NOTES.md` | `docs/releases/vX.Y.Z/` |
| 变更日志 | `CHANGELOG.md` | `docs/releases/vX.Y.Z/` |
| 功能矩阵 | `FEATURE_MATRIX.md` | `docs/releases/vX.Y.Z/` |
| 性能目标 | `PERFORMANCE_TARGETS.md` | `docs/releases/vX.Y.Z/` |
| 集成状态 | `INTEGRATION_STATUS.md` | `docs/releases/vX.Y.Z/` |
| ADR | `ARCHITECTURE_DECISIONS.md` | `docs/releases/vX.Y.Z/` |
| 用户手册 | `USER_MANUAL.md` | `docs/releases/vX.Y.Z/` |
| 部署指南 | `DEPLOYMENT_GUIDE.md` | `docs/releases/vX.Y.Z/` |
| 发布摘要 | `00-release-summary.md` | `docs/releases/vX.Y.Z/` |

---

## 四、最小文档集

### 必选文档

| 文档 | 阶段要求 |
|------|----------|
| `DEVELOPMENT_PLAN.md` | Alpha |
| `TEST_PLAN.md` | Alpha |
| `RELEASE_GATE_CHECKLIST.md` | Beta |
| `RELEASE_NOTES.md` | RC |
| `CHANGELOG.md` | RC |
| `FEATURE_MATRIX.md` | Beta |
| `INTEGRATION_STATUS.md` | Beta |
| `ARCHITECTURE_DECISIONS.md` | RC |
| `USER_MANUAL.md` | RC |
| `DEPLOYMENT_GUIDE.md` | RC |
| `00-release-summary.md` | GA |

### 可选文档

| 文档 | 说明 |
|------|------|
| `PERFORMANCE_TARGETS.md` | 性能基准和目标 |

---

## 五、链接规范

- **必须使用相对路径**: `../governance/GATE_SPEC.md`
- **禁止使用绝对路径**: `http://192.168.0.252:3000/...`
- **跨版本引用**: 明确标注版本号

---

## 六、版本一致性检查

每次 RC 门禁通过前，执行以下检查：

| 检查项 | 方法 |
|--------|------|
| 文档版本号 = 当前版本 | `grep "版本>: X.Y.Z" docs/releases/vX.Y.Z/` |
| 无遗留旧版本号 | 搜索不应出现旧版本号 |
| 日期格式正确 | `YYYY-MM-DD` 格式 |
| 阶段状态正确 | 不得出现 "Draft" 状态的 RC/GA 文档 |