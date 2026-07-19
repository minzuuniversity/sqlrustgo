# v3.11.0 Post-GA Plan

> **Version**: v3.11.0
> **Status**: Post-GA (2026-07-19)
> **Owner**: @openclaw

Post-GA tasks and next steps.

---

## 1. v3.11.0 Post-GA Immediate Tasks

| Task | Description | Status |
|------|-------------|--------|
| Tag v3.11.0 | Create git tag and release | Pending |
| Binary builds | Build release binaries for all platforms | Pending |
| Publish crates | `cargo publish` for all workspace crates | Pending |
| 168h SOAK | 168-hour stress test post-release | Pending |

---

## 2. v3.12.0 Planning

| Feature | Description | Priority |
|---------|-------------|----------|
| Distributed query execution | Multi-node query planning | P1 |
| Cypher compatibility | Graph traversal syntax | P2 |
| SIMD vectorization | Batch processing optimization | P2 |

---

## 3. Coverage Improvement Roadmap

| Crate | Current | v3.12.0 Target | Notes |
|-------|---------|----------------|-------|
| sqlrustgo-mysql-client | 43.79% | 60% | Integration tests |
| sqlrustgo-tools | 63.84% | 75% | CLI integration tests |
| sqlrustgo-mysql-server | 51.53% | 70% | Protocol tests |
| sqlrustgo-parser | 71.22% | 80% | Parser edge cases |
| sqlrustgo-executor | 76.45% | 82% | Executor edge cases |

---

## 4. Technical Debt

| Item | Description | Track Issue |
|------|-------------|-------------|
| Extension crate cleanup | Finalize V311-19 decisions | #3612 |
| API drift tests | Fix 12 broken integration tests | #3421 |
| BufferPool refactor | Remove legacy BufferPoolWithClock | #3136 |
