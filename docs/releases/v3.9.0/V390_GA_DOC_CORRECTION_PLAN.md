# 文档改正计划 (v3.9.0 GA 准备)

> **执行时间**: 2026-06-13
> **执行人**: Hermes Agent
> **依据**: `docs/governance/DOC_CHECK_CORRECTION_RULES.md` v1.0.0 (5 步标准流程)
> **最小修改原则**: 只修改事实性错误，不修改技术内容

---

## 一、问题清单 (从 Step 1)

| # | 文件 | 问题 | 位置 | 依据 |
|---|------|------|------|------|
| 1 | `docs/releases/v3.9.0/CHANGELOG.md` | Header 标注 "RC2 (form-only)"，实际有 rc4-rc7 | line 8 | git tag list 2026-06-12 |
| 2 | `docs/releases/v3.9.0/CHANGELOG.md` | 版本表缺 rc4/rc5/rc6/rc7 | "版本表" section | 实际 tags |
| 3 | `docs/releases/v3.9.0/README.md` | GA 目标 2026-09-23 需更新为 2026-12-15 | line 8 | Hermes audit #3252 (已关闭) 提到 |
| 4 | `docs/releases/v3.9.0/RELEASE_NOTES.md` | Header 无当前阶段 | top | 当前 rc7 |
| 5 | `/CHANGELOG.md` (root) | 缺 v3.9.0-rc4..rc7 entries | "v3.9.0" section | 实际 tags |
| 6 | `/ROADMAP.md` | v3.9.0 计划项 "MVCC 完整实现" 已完成 (实际 v3.8.0 完成) | section 4.9 | 实际状态 |
| 7 | `/README.md` | "Latest stable: v3.7.0" 实际应为 v3.8.0-GA | line 5 | v3.8.0-GA 已发布 |
| 8 | `docs/releases/v3.9.0/GA_GATE_REPORT.md` | 状态信息需补充 rc5-rc7 进展 | section 1 | 实际 tags |

---

## 二、修改操作

### 2.1 `docs/releases/v3.9.0/CHANGELOG.md`

**操作 1**: Header 阶段从 "RC2 (form-only)" → "RC7"

```
oldString: > **当前阶段**: **RC2 (form-only)** → RC3 待启动 (参考 V390_COMPREHENSIVE_ASSESSMENT.md)
newString: > **当前阶段**: **RC7** (awaiting 24h/72h/168h soak completion, see GA_GATE_REPORT.md)
```

**操作 2**: 版本表补全

```
oldString: | v3.9.0-rc3 | (planned) | after P0 issues closed (server perf + L3 + TX/WAL) |
newString: | v3.9.0-rc3 | 2026-06-12 | All 5 RC3 P0 blockers closed, G1-G16 PASS |
newString: | v3.9.0-rc4 | 2026-06-12 | RC4 gate PASS (G1/G7/G8/G9/G13), SHA-256 + QPS baseline, un-ignore tests |
newString: | v3.9.0-rc5 | 2026-06-12 | G2 substance + Z6G4 QPS + cross-version upgrade chain |
newString: | v3.9.0-rc6 | 2026-06-12 | INT-2/INT-3 full substance tests (Issues #3146, #3108) |
newString: | v3.9.0-rc7 | 2026-06-12 | Performance docs + MariaDB comparison (PR #3363) |
```

### 2.2 `docs/releases/v3.9.0/README.md`

**操作 3**: GA 目标日期更新

```
oldString: > **GA 目标**: 2026-09-23 (12 周 / 6 Phase)
newString: > **GA 目标**: 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)
```

### 2.3 `docs/releases/v3.9.0/RELEASE_NOTES.md`

**操作 4**: Header 添加当前阶段

```
oldString: # Release Notes — SQLRustGo v3.9.0

> Comprehensive list of changes from v3.8.0 → v3.9.0.
newString: # Release Notes — SQLRustGo v3.9.0

> **当前阶段**: RC7 (2026-06-12) — awaiting 24h/72h/168h soak for GA cut
> Comprehensive list of changes from v3.8.0 → v3.9.0.
```

### 2.4 `/CHANGELOG.md` (root)

**操作 5**: 在 "v3.9.0 Production Readiness Release 启动" 下方添加 RC 进展

```
oldString: ### v3.9.0 Production Readiness Release 启动
newString: ### v3.9.0 Production Readiness Release 启动
```
(保持不变, 启动日期已记录)
然后在 "v3.9.0" section 下添加：
```
### v3.9.0-rc4..rc7 (2026-06-12)
- rc4: G1-G13 gate PASS, SHA-256 + QPS baseline
- rc5: G2 substance + Z6G4 QPS + cross-version upgrade (closes #3224, #3230, #3270)
- rc6: INT-2/INT-3 full substance (closes #3108, #3146)
- rc7: Performance docs + MariaDB comparison
```

### 2.5 `/ROADMAP.md`

**操作 6**: v3.9.0 计划项更新

```
oldString: ### 4.9 v3.9 - MVCC 完整实现 ⏳ 计划中
newString: ### 4.9 v3.9 - Production Readiness ✅ RC7 (awaiting soak)
- Status: v3.9.0-rc7 (2026-06-12), GA target 2026-12-15
- Gates G1-G16 PASS, 36 substance tests PASS
- Open: 5 soak-related issues (#3264/#3265/#3266, #3225, #3229)
- See docs/releases/v3.9.0/GA_GATE_REPORT.md
```

### 2.6 `/README.md`

**操作 7**: 最新稳定版本

```
oldString: > **Latest stable**: v3.7.0 (GA, 2026-05-31)
newString: > **Latest stable**: v3.8.0 (GA, 2026-06-08) | v3.9.0-rc7 (in soak, GA target 2026-12-15)
```

### 2.7 `docs/releases/v3.9.0/GA_GATE_REPORT.md`

**操作 8**: 添加 RC4-RC7 进展 + 250 24h soak 当前状态

```
oldString: > **Tag**: `v3.9.0-rc4` at `450d8b736` (cut); GA pending 24h/72h/168h soak completion
newString: > **Latest tag**: `v3.9.0-rc7` at `0868910f1` (2026-06-12)
> **GA pending**: 24h/72h/168h soak completion (250 24h running, 671 samples, 0 errors)
```

---

## 三、复核审查 Checklist

- [ ] 操作 1: CHANGELOG 阶段从 RC2 → RC7
- [ ] 操作 2: CHANGELOG 版本表添加 rc4/rc5/rc6/rc7
- [ ] 操作 3: README GA 目标 2026-12-15
- [ ] 操作 4: RELEASE_NOTES Header 阶段标注
- [ ] 操作 5: root CHANGELOG 添加 RC4-7 entries
- [ ] 操作 6: ROADMAP 4.9 更新为 RC7 状态
- [ ] 操作 7: README Latest stable v3.8.0
- [ ] 操作 8: GA_GATE_REPORT latest tag/soak status
- [ ] 无 commit 日志内容修改
- [ ] 无功能描述/架构设计修改
- [ ] 所有引用 .md 文件存在
- [ ] git diff 干净

---

## 四、预期结果

完成 8 项修改后，文档状态将：
- 反映 v3.9.0 实际进度（rc7）
- 标注 GA 真实目标（2026-12-15）
- 移除过期信息（v3.7.0 latest, MVCC 待实现）
- 保持最小修改原则（不改技术内容）
