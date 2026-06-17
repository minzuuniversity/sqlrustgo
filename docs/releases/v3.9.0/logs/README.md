# Gate 执行日志目录

> **版本**: v3.9.0
> **更新日期**: 2026-06-17

## 日志命名规范

每次 Gate 执行必须保存日志，命名格式：

```
gate_<阶段>_<commit>_<timestamp>.log
```

### 示例

```
gate_alpha_642ff9cf9_20260617_221500.log
gate_beta_642ff9cf9_20260617_221500.log
gate_rc_642ff9cf9_20260617_221500.log
gate_ga_642ff9cf9_20260617_221500.log
```

## 日志保存要求

1. **每个 Gate 执行必须保存日志**
2. **日志必须来自实际命令输出**，禁止文档审查替代
3. **日志文件必须可验证**：脚本退出码 0=PASS, 1=FAIL
4. **保存路径**：`docs/releases/v{VERSION}/logs/`

## 目录结构

```
logs/
├── README.md                          # 本文件
├── gate_alpha_<commit>_<ts>.log       # Alpha Gate 日志
├── gate_beta_<commit>_<ts>.log       # Beta Gate 日志
├── gate_rc_<commit>_<ts>.log          # RC Gate 日志
├── gate_ga_<commit>_<ts>.log          # GA Gate 日志
└── soak_1h_<host>_<ts>.log            # 1h 浸泡测试日志
    soak_24h_<host>_<ts>.log           # 24h 浸泡测试日志
    soak_72h_<host>_<ts>.log           # 72h 浸泡测试日志
    soak_168h_<host>_<ts>.log          # 168h 浸泡测试日志
```

## 关联要求

- GATE_CONDITIONS.md §门禁执行要求 - 日志保存
- RELEASE_LIFECYCLE.md - 版本阶段定义
