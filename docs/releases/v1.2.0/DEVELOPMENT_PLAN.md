# GMP-Platform v1.2.0 开发计划

> **版本**: 1.0
> **日期**: 2026-05-23
> **分支**: develop/v1.2.0

---

## 一、版本目标

在 v1.1.0 GA 基础上，v1.2.0 聚焦于：
1. **Web Dashboard**: 纯 stdlib 无外部依赖的 Web 可视化界面
2. **DiskGraphStore**: 图索引持久化，依赖 SQLRustGo #1326
3. **压测框架**: 72h 持续压测，发现内存泄漏和性能退化
4. **工具集完善**: 移动端 CLI、编排器、DSL 验证器

---

## 二、功能清单

| 功能 | 优先级 | 状态 |
|------|--------|------|
| Web Dashboard | P0 | ✅ 开发中 |
| DiskGraphStore 持久化 | P0 | ⏳ 等待 SQLRustGo #1326 |
| 移动端 CLI 完善 | P1 | ✅ PR #84 |
| 72h 压测框架 | P1 | ✅ 完成 |
| MQTT 设备集成 | P2 | ⏸ 推迟至 v1.3.0 |

---

## 三、组件矩阵

| Crate | v1.1.0 | v1.2.0 变更 |
|-------|--------|-------------|
| gmp-core | GA | 无变化 |
| gmp-retrieval | GA | 无变化 |
| gmp-reranker | GA | 无变化 |
| gmp-llm | GA | 无变化 |
| gmp-storage | GA | 无变化 |
| gmp-cli | GA | 修复 runtime panic |
| gmp-eval | GA | 无变化 |
| gmp-auth | 新增 | JWT + 电子签名 |
| dashboard | 新增 | Web Dashboard |
| gmp_agent | 新增 | 移动端工具集 |

---

## 四、交付物

- `dashboard/`: Web Dashboard (server.py + index.html)
- `gmp_agent/`: 移动端工具集
- `scripts/stress_test_72h.py`: 72h 压测脚本
- `docs/governance/gate_spec_v120.md`: 门禁规范
- `scripts/gate/check_*_v120.sh`: 各级门禁脚本
