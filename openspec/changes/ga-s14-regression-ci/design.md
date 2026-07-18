## Overview

回归测试套件确保版本质量。

## Components

1. **TPC-H Baseline** — 22/22 queries at SF=1
2. **Chaos Soak** — 2h 随机故障注入
3. **Upgrade Test** — v3.10.0 → v3.11.0
4. **Performance Regression** — 相比 baseline 无退化

## Gate Criteria

- All tests must pass
- No performance regression > 10%
