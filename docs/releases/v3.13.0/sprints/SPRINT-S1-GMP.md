# SPRINT-S1: GMP 治理 (Cluster A)

## §1 Scope

闭合 #4225 (V312-52 GMP vector/retrieval) + #4226 (V312-53 GMP compliance/access-control),把 v3.12.0 阶段 DEFERRED 的 production wiring 任务在 v3.13.0 production code path 上落地。

## §2 Entry 标准

- [x] SPRINT-S0 三个 blocker 全部完成 (commit `314cc35afe`)
- [x] V313-STRICT-CLOSE-STANDARDS.md 已合并 (`a1940a9fdc`)
- [x] V313-FOLLOWUP-INDEX.md Cluster A 已列出 (`0190762b71`)

## §3 上游基线 (v3.12.0 已完成部分)

| Issue | 上游 commit | 已完成 | DEFERRED → v3.13 |
|---|---|---|---|
| #4225 (V312-52) | `6c717ed671` | Flat index + hybrid retrieval | fixture determinism + rebuild persistence + fail-closed + model-name 一致性 + (model_name, dimension) 唯一索引 |
| #4226 (V312-53) | `0bf1520e95` + `5fa7a4fed8` + `8b4365b5a7` | CRUD audit chain + deterministic audit fixture | compliance-op chain (Import/Export/Approve/Review/Backup/Restore) + tamper integration + ACL 5x12 全矩阵 + production wiring |

## §4 入口任务

### §4.1 #4225 V312-52 GMP vector/retrieval production wiring

- [ ] **deterministic top-k fixture** — SHA-256 锚定 fixture,跨进程/跨重启同输入返同 top-k
- [ ] **rebuild persistence** — 关闭重开 index,数据一致 (count + 每条 ID + distance)
- [ ] **dimension drift fail-closed** — loaded model dim != index dim → ERROR 而非 silent fallback
- [ ] **empty index fail-closed** — 空 index 查询 → 错误而非返全 0
- [ ] **model-name consistency** — catalog model_name 与 graph 一致 (无 orphan graph)
- [ ] **(model_name, dimension) 唯一索引** — 防重复创建,违反 → constraint error

### §4.2 #4226 V312-53 GMP compliance/access-control production wiring

- [ ] **AuditAction 6 类型实现** — Import / Export / Approve / Review / Backup / Restore 全部触发 audit log
- [ ] **hash-chain tamper integration** — WAL log 哈希链验证,断链 → ERROR + panic recovery 路径
- [ ] **embedding/graph tamper** — ML model 加载时 SHA-256 校验,失配 → ERROR
- [ ] **ACL 5x12 全矩阵** — 5 role × 12 action = 60 case,每 case 期望 pass/deny
- [ ] **production wiring** — 从 sandbox demo 提升到 production code path (no `#[cfg(test)]` only)

## §5 退出标准

- [ ] #4225 + #4226 各自有 merged PR 到 `develop/v3.13.0`
- [ ] `bash scripts/gate/check_v313_gmp_gate.sh` exit=0
- [ ] §4.1 六项 + §4.2 五项全部有真证据 (命令 + exit code + 输出摘要 + SHA-256)
- [ ] `docs/releases/v3.13.0/evidence/V313-S1-GMP-VERIFICATION.md` 已生成
- [ ] Anti-Fabrication-Policy-v1.0 验证:任何 gate fail 必须诚实披露

## §6 风险与依赖

| Risk | 影响 | Mitigation |
|---|---|---|
| production wiring 工作量超 2 周 | S1 延期 | 拆 2-3 个 sub-issue,每 PR gate PASS |
| ACL 60 case matrix 测出未实现路径 | 返工 | Phase 0 先写 failing test 摸底,再补实现 |
| tamper integration 触及 WAL 内部 | 影响其他模块 | 用 trait 抽象 hash-chain,backend 可替换 |

## §7 预估工作量

约 2-3 周 (per memory `v312-round24-chatgpt-remediation.md` 备注 "GMP vector/retrieval requires substantial code work beyond v3.12 timeframe")。

## §8 References

- Round-24 evidence manifest: `V313-ROUND24-EVIDENCE-MANIFEST.md` (per memory `v312-round24-chatgpt-remediation.md`)
- Cluster A 索引: `V313-FOLLOWUP-INDEX.md` Cluster A
- 上游 DEFERRED 提交: `6c717ed671` (#4225), `0bf1520e95` + `5fa7a4fed8` + `8b4365b5a7` (#4226)
- 严格关闭标准: `V313-STRICT-CLOSE-STANDARDS.md`

## Evidence Hash

`sha256=481d0046fc74c0eb26b0d21a96c31c578c212f70d23612a993857e38a26be560` (computed on file content at HEAD)