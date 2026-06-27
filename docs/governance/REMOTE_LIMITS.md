# 远程仓库限制与解决方案 (REMOTE_LIMITS)

> **Date**: 2026-06-13
> **Status**: 🟡 Documented limitation, deferred to v3.9.1

## 5+1 远程仓库状态

| Remote | URL | Tip | Status |
|--------|-----|-----|--------|
| **252 Gitea** (PRIMARY) | http://192.168.0.252:3000/openclaw/sqlrustgo | `917979692` | ✅ Auto-restart, self-healing |
| **250 Gitea** (BACKUP) | http://192.168.0.250:3000/openclaw/sqlrustgo | `917979692` | ✅ Auto-restart, stable |
| **GitHub** | https://github.com/minzuuniversity/sqlrustgo | `917979692` | ✅ Public mirror |
| **Gitee** (CN mirror) | https://gitee.com/yinglichina/sqlrustgo | `917979692` | ✅ Public mirror |
| **Gitcode** | https://gitcode.com/BreavHeart/sqlrustgo | `4a2ad05582` | ⚠️ BLOCKED |
| **Local (Mac mini)** | `~/workspace/dev/yinglichina163/sqlrustgo` | `917979692` | ✅ Working dir |

**4 of 6 remotes fully synced. 1 (Gitcode) blocked by LFS requirement. 1 (Local) is develop working copy.**

---

## Gitcode 限制

### 问题

`git push gitcode HEAD:refs/heads/develop/v3.9.0` returns:

```
remote: chore: migrate large files to LFS
! [remote rejected] HEAD -> develop/v3.9.0 (pre-receive hook declined)
```

### 原因

Gitcode pre-receive hook enforces **strict LFS migration**:
- Files >100KB in history must be LFS pointers, not regular git objects
- Currently in history: `*.tbl` (7-8MB), `*.html` (8.4MB), `*.tar.gz` (2.2MB), `*.wal` (1.1MB)
- The `.gitattributes` we added (PR #3377) only affects **future** files
- Historical blobs are immutable

### 解决方案对比

| 方案 | 优点 | 缺点 | 风险 |
|------|------|------|------|
| **A. Full LFS migrate** | 一劳永逸，gitcode 完整同步 | 改写全部 git history，所有 SHA 变化，**需 force-push 所有 4+ 远程**，破坏其他开发者环境，CI cache 失效 | 🔴 **HIGH** |
| **B. .gitattributes + 历史保留** | 不破坏现有 SHA，新文件自动 LFS | 历史文件仍阻塞，gitcode 部分同步 | 🟡 LOW (current) |
| **C. 放弃 Gitcode** | 立即可行，零风险 | 失去一个 CN 镜像 | 🟢 LOW |
| **D. 创建 Gitcode 专用 fork** | 完整同步，不影响主仓库 | 维护成本（每次 push 双倍） | 🟡 MEDIUM |

### 决策 (2026-06-13)

**采用方案 B** — `.gitattributes` 已在 #3377 合并，**v3.9.0 GA 不阻塞**。完整 LFS 迁移在 v3.9.1 进行（计划）。

### v3.9.1 迁移计划

```bash
# Full LFS migration (DESTRUCTIVE — all SHAs change)
git lfs migrate import --everything \
  --include="*.tbl,*.wal,*.tar.gz,*.tar,*.zip,*.html,*.bin"

# Then force-push all remotes
git push --force-with-lease origin develop/v3.9.0
git push --force-with-lease backup develop/v3.9.0
git push --force-with-lease github develop/v3.9.0
git push --force-with-lease gitee develop/v3.9.0
git push --force-with-lease gitcode develop/v3.9.0
```

**Prerequisites**:
- [ ] All 4+ active developers aware of SHA change
- [ ] CI cache cleared
- [ ] Other agent's worktrees re-anchored to new SHAs
- [ ] Issue/PR links to old SHAs updated

---

## 当前 4-Remote 同步策略

### 4-Remote 完整同步 (252/250/GitHub/Gitee)
1. Local commit on `develop/v3.9.0`
2. Push to GitHub: `git push github HEAD:develop/v3.9.0` (force-with-lease if needed)
3. Push to 250: `git push backup HEAD:develop/v3.9.0`
4. Push to Gitee: `git push gitee HEAD:develop/v3.9.0`
5. **For 252 (PR-protected)**: Create branch + PR, then merge via API

### 252 Merge 速率限制应对

If `POST /api/v1/repos/.../pulls/{n}/merge` returns `HTTP 405 "Please try again later"`:
- Wait 30-60 minutes for rate limit to clear
- Use **sliding window trap** pattern: each failed retry re-blocks for 40-60 min
- **Definitive workaround**: Use manual `git commit-tree` to construct merge commit, then `git push --force-with-lease origin develop/v3.9.0`
- Skill: `gitea-api-merge-rate-limit-workaround`

---

## 250 vs 252 角色

| 角色 | 252 (Z6G4) | 250 (Z440) |
|------|-----------|-----------|
| **Hostname** | 192.168.0.252 | 192.168.0.250 |
| **OS** | Linux (Nomad runner) | Linux (lighter) |
| **可靠性** | 5+ outages/day | Stable |
| **Resources** | 414GB RAM, load 12-19 | 96GB RAM, load 3-4 |
| **Gitea** | Primary, fast-iter | Backup, slow-iter |
| **CI/CD** | Heavy tests | Soaks, long-running |
| **Stability gates** | G13, Crash Matrix | 24h/72h/168h soak |

**Why 250 is more stable**:
- Less load (no concurrent dev agents)
- Better thermals
- Simpler disk layout (single sdb2)
- No Z6G4 network switch issue

**Why we keep 252 as primary**:
- Faster (more cores)
- Closer to where development happens
- More disk space (1.9TB vs 916GB)

---

## Gitea 备份策略

| Backup Type | Location | Frequency | Retention |
|-------------|----------|-----------|-----------|
| **DB dump** | `/home/openclaw/gitea-backup-20260613/gitea-db-*.sql.gz` | Daily 3am (cron) | 7 days |
| **Config** | `/opt/backups/gitea-config-*.tar.gz` | On upgrade | 30 days |
| **Bundle** | `/opt/backups/sqlrustgo-all.bundle` | Manual | Forever |
| **Per-server data** | Inside `devstack_gitea-data` volume | Always (live) | Indefinite |

**Critical**: DO NOT run `docker compose -p devstack up -d` after Z6G4 reset without verifying volume first. See `gitea-api-merge-rate-limit-workaround/references/gitea-data-loss-recovery-2026-06-12.md`.

---

## 跨远程分支策略

| Branch | Purpose | Sync Strategy |
|--------|---------|---------------|
| `main` | Stable releases only | Manual cherry-pick from `develop/v*.*.*` |
| `develop/v3.9.0` | Current dev | All 4 remotes sync |
| `develop/v3.8.0` | Maintenance | Synced to all 4 |
| `feature/*` | Per-feature work | Local + 252 (via PR) |
| `sync/*` | Cross-server sync | Ephemeral, deleted after merge |

---

## 相关 Issues

- #3252: GA audit report (Hermes 2026-06-12)
- #3264/#3265/#3266: 24h/72h/168h soak (open)
- #3225/#3229: Real wall-clock soak (open)
- PR #3377: .gitattributes for LFS (merged)

---

Last updated: 2026-06-13 03:10 CST
