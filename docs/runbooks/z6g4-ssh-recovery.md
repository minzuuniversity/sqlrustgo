# Z6G4 SSH Recovery Runbook

> 恢复 Z6G4 (192.168.0.252) 容器的 SSH Git 访问
> 适用场景：`ssh -p 222 git@192.168.0.252` 报 `Permission denied (publickey)`

## 症状

```
$ ssh -T -p 222 git@192.168.0.252
git@192.168.0.252: Permission denied (publickey).
```

- Web UI（端口 3000）正常
- 容器端口 222 (sshd) 在 listen
- 容器内 `/data/git/.ssh/authorized_keys` 缺失或为空

## 根本原因

Gitea 1.26.x 在容器内的 git 用户（uid 1000）需要：
1. 拥有 `/data/git/.ssh/` 目录（700，git:git）
2. 拥有 `/data/git/.ssh/authorized_keys` 文件
3. 文件由 `gitea admin regenerate keys` 从 DB `public_key` 表自动生成

当 Gitea 容器因 OOM、磁盘满、误操作等导致 `.ssh` 目录属主变更为 root 时：
- `regenerate keys` 写入失败（permission denied）
- 所有 client key 均无法认证

## 恢复流程（已验证，2026-06-08）

### Step 0：备份

```bash
# 备份容器内 .ssh
docker cp devstack-gitea-1:/data/git/.ssh /tmp/z6g4_gitea_ssh_backup_$(date +%Y%m%d_%H%M%S)

# 备份主机 .ssh
cp -r ~/.ssh ~/.ssh.full.backup.$(date +%Y%m%d_%H%M%S)

# 备份 authorized_keys
cp ~/.ssh/authorized_keys ~/.ssh/authorized_keys.backup.$(date +%Y%m%d_%H%M%S)
```

### Step 1：修复目录属主

```bash
docker exec devstack-gitea-1 chown git:git /data/git/.ssh
docker exec devstack-gitea-1 chmod 700 /data/git/.ssh
```

### Step 2：重新生成 authorized_keys

```bash
docker exec -u git devstack-gitea-1 sh -c "gitea admin regenerate keys 2>&1"
```

### Step 3：添加 client 公钥到 Gitea 用户

#### 3a: 通过 Gitea admin token（推荐，自动化）

```bash
# 容器内生成 admin token (scopes=all)
ADMIN_TOK=$(docker exec -u git devstack-gitea-1 sh -c \
  "gitea admin user generate-access-token \
   --username openclaw --token-name 'admin-restore' \
   --scopes 'all' --raw" | tail -1)

# 用 admin token 添加公钥
curl -X POST "http://192.168.0.252:3000/api/v1/user/keys" \
  -H "Authorization: token $ADMIN_TOK" \
  -H "Content-Type: application/json" \
  -d "{\"title\": \"Z6G4-id_ed25519\", \"key\": \"$(cat ~/.ssh/id_ed25519.pub)\"}"

# 添加完后再次 regenerate 让其生效
docker exec -u git devstack-gitea-1 sh -c "gitea admin regenerate keys 2>&1"
```

#### 3b: Web UI 手动

1. 浏览器打开 http://192.168.0.252:3000/user/settings/keys
2. 登录 openclaw 账户
3. 粘贴 `cat ~/.ssh/id_ed25519.pub` 的内容
4. 标题：`Z6G4-id_ed25519`
5. 添加
6. 容器内再次 `gitea admin regenerate keys`

### Step 4：验证

```bash
# SSH 协议认证
ssh -T -p 222 -i ~/.ssh/id_ed25519 git@192.168.0.252
# 期望: Hi there, openclaw! You've successfully authenticated ...

# Git 协议 ls-remote
git ls-remote ssh://git@192.168.0.252:222/openclaw/sqlrustgo HEAD
# 期望返回 commit SHA
```

## 回滚

如恢复失败导致 Gitea 不可用：

```bash
# 恢复容器 .ssh
docker exec devstack-gitea-1 rm -rf /data/git/.ssh
docker cp /tmp/z6g4_gitea_ssh_backup_YYYYMMDD_HHMMSS/. devstack-gitea-1:/data/git/.ssh/
docker exec devstack-gitea-1 chown root:root /data/git/.ssh
docker exec devstack-gitea-1 chmod 700 /data/git/.ssh

# 恢复主机 .ssh
rm -rf ~/.ssh
cp -r ~/.ssh.full.backup.YYYYMMDD_HHMMSS ~/.ssh
```

## 预防

1. **避免在容器内以 root 写入 `/data/git/.ssh/`** — 写入会导致属主变更
2. **定期备份 `/data/git/.ssh/authorized_keys`** — 自动化脚本
3. **监控 SSH 端点健康** — 定期 `ssh -T -p 222 git@192.168.0.252` 测试
4. **容器启动后立即检查属主** — entrypoint hook

## 已知陷阱

- Gitea 1.26 admin CLI **无 `add-key` 子命令** — 必须用 API
- API 需要 `write:user` scope — 现有 token `04bcda86...` 只有 `write:issue,write:repository`
- 解决方案：容器内 `gitea admin user generate-access-token --scopes all`

## 参考

- Issue: Gitea #3263
- 容器: `devstack-gitea-1` (192.168.0.252:5000/gitea:1.26.1)
- 主机: HP Z6G4 (192.168.0.252)
- 用户: openclaw (uid 1000, Gitea admin)
