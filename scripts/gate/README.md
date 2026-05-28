# Gate Scripts Index

> SQLRustGo 门禁脚本索引文档
> 分支: `origin/develop/v1.9.0`
> 更新: 2026-05-29

---

## 一、门禁阶段说明

| 阶段 | 名称 | 说明 |
|------|------|------|
| **Alpha** | 内部开发门禁 | 编译、单元测试、Clippy、格式化、覆盖率、L0 测试 |
| **Beta** | 特性冻结门禁 | Alpha + TPC-H SF=1、SQL 兼容性、安全扫描、文档链接 |
| **RC** | 发布候选门禁 | Beta + 全面测试、Mutants 变异测试、性能基线 |
| **GA** | 最终发布门禁 | RC + 审计验证、生产就绪确认 |

---

## 二、版本相关脚本索引

### 2.1 v3.6.0 (当前分支)
> 暂无专用门禁脚本 (develop/v1.9.0 分支正在开发)

### 2.2 v3.5.0
| 脚本 | 用途 |
|------|------|
| `check_alpha_v350.sh` | v3.5.0 Alpha Gate |
| `check_beta_v350.sh` | v3.5.0 Beta Gate |
| `check_rc_v350.sh` | v3.5.0 RC Gate |
| `check_ga_v350.sh` | v3.5.0 GA Gate |

### 2.3 v3.4.0
| 脚本 | 用途 |
|------|------|
| `check_alpha_v340.sh` | v3.4.0 Alpha Gate |
| `check_beta_v340.sh` | v3.4.0 Beta Gate |
| `check_rc_v340.sh` | v3.4.0 RC Gate |
| `check_ga_v340.sh` | v3.4.0 GA Gate |

### 2.4 v3.3.0
| 脚本 | 用途 |
|------|------|
| `check_alpha_v330.sh` | v3.3.0 Alpha Gate |
| `check_beta_v330.sh` | v3.3.0 Beta Gate |
| `check_rc_v330.sh` | v3.3.0 RC Gate |
| `check_ga_v330.sh` | v3.3.0 GA Gate |

### 2.5 v3.2.0
| 脚本 | 用途 |
|------|------|
| `check_alpha_v320.sh` | v3.2.0 Alpha Gate |
| `check_beta_v320.sh` | v3.2.0 Beta Gate |
| `check_rc_v320.sh` | v3.2.0 RC Gate |
| `check_ga_v320.sh` | v3.2.0 GA Gate |
| `check_trust_gate_v320.sh` | v3.2.0 Trust Gate |

### 2.6 v3.1.0
| 脚本 | 用途 |
|------|------|
| `check_alpha_v310.sh` | v3.1.0 Alpha Gate |
| `check_beta_v310.sh` | v3.1.0 Beta Gate |
| `check_rc_v310.sh` | v3.1.0 RC Gate |
| `check_ga_v310.sh` | v3.1.0 GA Gate |

### 2.7 v3.0.0
| 脚本 | 用途 |
|------|------|
| `check_beta_v300.sh` | v3.0.0 Beta Gate |
| `check_beta_v300_phase2.sh` | v3.0.0 Beta Phase 2 |
| `check_rc_v300.sh` | v3.0.0 RC Gate |
| `check_ga_v300.sh` | v3.0.0 GA Gate |

---

## 三、通用脚本索引

### 3.1 门禁执行器
| 脚本 | 用途 |
|------|------|
| `check_gate.sh` | 通用门禁执行器 (参数: VERSION PHASE) |
| `gate.sh` | 门禁主入口 |
| `hermes_gate.sh` | Hermes Agent 门禁接口 |
| `run_hermes_gate.sh` | Hermes 门禁运行器 |
| `check_gate_sync.sh` | 门禁同步检查 |

### 3.2 覆盖率检查
| 脚本 | 用途 |
|------|------|
| `check_coverage.sh` | 覆盖率检查 (L1 ≥75%, GA ≥85%) |
| `check_coverage_comprehensive.sh` | 全面覆盖率检查 |
| `aggregate_coverage.sh` | 覆盖率聚合 |

### 3.3 性能测试
| 脚本 | 用途 |
|------|------|
| `check_tpch.sh` | TPC-H SF=1 基准测试 |
| `setup_tpch_env.sh` | TPC-H 环境配置 |
| `check_sysbench.sh` | Sysbench 性能测试 |
| `check_perf_baseline.sh` | 性能基线检查 |
| `check_benchmark.sh` | 基准测试检查 |
| `check_performance.sh` | 性能检查 |

### 3.4 安全与合规
| 脚本 | 用途 |
|------|------|
| `check_security.sh` | 安全扫描 |
| `check_attack_surface.sh` | 攻击面检查 |
| `check_hsm.sh` | HSM 检查 |
| `check_mobile.sh` | 移动端兼容性 |
| `check_four_eyes.sh` | 四眼原则检查 |

### 3.5 文档与链接
| 脚本 | 用途 |
|------|------|
| `check_docs_links.sh` | 文档链接检查 |
| `check_docs.sh` | 文档完整性检查 |
| `check_oo_docs.sh` | OO 文档检查 |
| `audit_documentation.sh` | 文档审计 |

### 3.6 SQL 兼容性
| 脚本 | 用途 |
|------|------|
| `check_sql_compat.sh` | SQL 兼容性检查 (SQL Corpus ≥80%) |

### 3.7 测试与回归
| 脚本 | 用途 |
|------|------|
| `check_l0_smoke.sh` | L0 Smoke 测试 |
| `check_regression.sh` | 回归测试 |
| `crash_recovery_loop.sh` | 崩溃恢复循环测试 |
| `check_mutants.sh` | 变异测试 |

### 3.8 审计脚本
| 脚本 | 用途 |
|------|------|
| `audit_development.sh` | 开发流程审计 |
| `audit_process.sh` | 流程审计 |
| `audit_testing.sh` | 测试流程审计 |

### 3.9 高级特性
| 脚本 | 用途 |
|------|------|
| `check_correction_chain.sh` | 纠错链检查 |
| `check_timestamp.sh` | 时间戳检查 |
| `check_immutable_record.sh` | 不可变记录检查 |
| `check_proof.sh` | 证明检查 |
| `check_provenance.sh` | 数据溯源检查 |
| `check_digital_signature.sh` | 数字签名检查 |
| `check_electronic_signature.sh` | 电子签名检查 |
| `check_static_analysis.sh` | 静态分析 |
| `check_workflow.sh` | 工作流检查 |

### 3.10 Webhook 与通知
| 脚本 | 用途 |
|------|------|
| `send_gate_alert.sh` | 门禁告警发送 |
| `send_gate_failure_webhook.sh` | 门禁失败 Webhook |
| `gate_webhook.sh` | Gate Webhook |

### 3.11 其他工具
| 脚本 | 用途 |
|------|------|
| `collect_self_opt_metrics.sh` | 自优化指标收集 |
| `run_sqlite_diff.sh` | SQLite 差异对比 |
| `update_proof_registry.sh` | 证明注册表更新 |
| `gate_lifecycle_check.sh` | 门禁生命周期检查 |
| `check_qa_enhancement.sh` | QA 增强检查 |
| `check_test_plan_sync.sh` | 测试计划同步检查 |
| `pre-receive-hook.sh` | Pre-receive Hook |

---

## 四、使用指南

### 4.1 快速执行

```bash
# 进入项目根目录
cd /home/ai/sqlrustgo/.wt-v360-p1

# 执行 v3.5.0 Alpha Gate
bash scripts/gate/check_alpha_v350.sh

# 使用通用执行器
bash scripts/gate/check_gate.sh v3.5.0 alpha
bash scripts/gate/check_gate.sh v3.5.0 beta
bash scripts/gate/check_gate.sh v3.5.0 rc
bash scripts/gate/check_gate.sh v3.5.0 ga
```

### 4.2 各阶段检查项

**Alpha Gate 检查项:**
- A1: Build - `cargo build --release --workspace`
- A2: Test - `cargo test --lib`
- A3: Clippy - `cargo clippy --all-features -- -D warnings`
- A4: Format - `cargo fmt --all -- --check`
- A5: Coverage - `cargo llvm-cov` (L1 CRATES ≥75%)
- A6: MySQL Protocol - MySQL client 连接测试
- A7: TPC-H SF=1 - 22/22 queries pass

**Beta Gate 检查项:**
- B1: Build - 编译通过
- B2: Test ≥90% - L1 core crates 通过率 ≥90%
- B3: Clippy - 零警告
- B4: Format - `cargo fmt --check` 通过
- B5: Coverage ≥75% - 覆盖率 ≥75% (Beta) / 85% (GA)
- B6: Security - 无已知漏洞
- B7: SQL Compat - SQL Corpus ≥80%
- B8: TPC-H SF=1 - TPC-H 基准可运行
- B9: Proof - 证明文件存在

---

## 五、门禁结果说明

| 标记 | 含义 |
|------|------|
| PASS | 检查通过 |
| FAIL | 检查失败 (阻塞发布) |
| SKIP | 检查跳过 (条件不满足) |

---

## 六、相关文档

- [AGENTS.md](../../AGENTS.md) - Agent 执行规则
- [GATE_SPEC_MASTER.md](../../docs/governance/GATE_SPEC_MASTER.md) - 门禁规范 SSOT
- [GATE_CHECKLIST_TEMPLATE.md](../../docs/governance/GATE_CHECKLIST_TEMPLATE.md) - 门禁清单模版