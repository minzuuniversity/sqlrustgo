# v3.4.0 快速入门

## 5 分钟快速上手

### 1. 安装（30秒）

```bash
curl -Lo sqlrustgo https://github.com/minzuuniversity/sqlrustgo/releases/v3.4.0/sqlrustgo
chmod +x sqlrustgo
sudo -S -p '' mv sqlrustgo /usr/local/bin/
sqlrustgo --version
```

### 2. 启动服务器（10秒）

```bash
sqlrustgo serve --port 5432
```

### 3. 连接数据库（10秒）

```bash
psql -h localhost -p 5432 -U postgres -d sqlrustgo
```

### 4. 执行第一个查询

```sql
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO users VALUES (1, 'Alice');
SELECT * FROM users;
```

## 常用命令

| 命令 | 说明 |
|------|------|
| `sqlrustgo serve` | 启动服务器 |
| `sqlrustgo shell` | 交互式 SQL |
| `sqlrustgo bench` | 运行基准测试 |
| `sqlrustgo health-check` | 健康检查 |

## 连接示例

### Python

```python
import psycopg2
conn = psycopg2.connect(host="localhost", port=5432, database="sqlrustgo", user="postgres")
cur = conn.cursor()
cur.execute("SELECT 1 + 1")
print(cur.fetchone())  # (2,)
```
