# v3.4.0 快速入门

## 5 分钟快速上手

### 1. 安装（30秒）

```bash
# Linux/macOS
curl -Lo sqlrustgo https://github.com/minzuuniversity/sqlrustgo/releases/v3.4.0/sqlrustgo
chmod +x sqlrustgo
sudo -S -p '' mv sqlrustgo /usr/local/bin/

# 验证
sqlrustgo --version
```

### 2. 启动服务器（10秒）

```bash
# 启动 SQL 服务器
sqlrustgo serve --port 5432

# 输出示例：
# SQLRustGo v3.4.0
# Listening on 0.0.0.0:5432
```

### 3. 连接数据库（10秒）

```bash
# 使用 psql 连接
psql -h localhost -p 5432 -U postgres -d sqlrustgo

# 或使用 sqlrustgo CLI
sqlrustgo shell
```

### 4. 执行第一个查询（1分钟）

```sql
-- 创建表
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE
);

-- 插入数据
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');

-- 查询
SELECT * FROM users;

-- 输出：
--  id | name |      email
-- ----+------+------------------
--   1 | Alice | alice@example.com
--   2 |   Bob |   bob@example.com
```

### 5. 运行基准测试（2分钟）

```bash
# TPC-H 基准测试
sqlrustgo bench --test tpch --scale-factor 1

# 输出示例：
# TPC-H SF=1 Results:
# Q1: 16.0s  Q2: 3.2s  ... Q22: 3.0s
# Total: 292s (22/22 PASS)
```

## 常用命令

| 命令 | 说明 |
|------|------|
| `sqlrustgo serve` | 启动服务器 |
| `sqlrustgo shell` | 交互式 SQL |
| `sqlrustgo bench` | 运行基准测试 |
| `sqlrustgo backup` | 备份/恢复 |
| `sqlrustgo health-check` | 健康检查 |

## 连接示例

### Python

```python
import psycopg2

conn = psycopg2.connect(
    host="localhost",
    port=5432,
    database="sqlrustgo",
    user="postgres"
)
cur = conn.cursor()
cur.execute("SELECT 1 + 1")
print(cur.fetchone())  # (2,)
```

### Go

```go
import (
    "database/sql"
    _ "github.com/lib/pq"
)

func main() {
    db, _ := sql.Open("postgres", "host=localhost port=5432 user=postgres dbname=sqlrustgo")
    defer db.Close()
    
    var result int
    db.QueryRow("SELECT 1 + 1").Scan(&result)
    println(result)  // 2
}
```

### Java

```java
import java.sql.*;

public class Example {
    public static void main(String[] args) throws SQLException {
        Connection conn = DriverManager.getConnection(
            "jdbc:postgresql://localhost:5432/sqlrustgo");
        Statement stmt = conn.createStatement();
        ResultSet rs = stmt.executeQuery("SELECT 1 + 1");
        while (rs.next()) {
            System.out.println(rs.getInt(1));  // 2
        }
    }
}
```

## 下一步

- 📖 [完整文档](../README.md)
- 📦 [安装指南](INSTALL.md)
- 🔧 [配置参考](../CONFIG.md)
- 🧪 [测试套件](../TESTING.md)
- 🏗️ [架构设计](../ARCHITECTURE.md)

## 常见问题

### Q: 连接被拒绝？

```bash
# 检查服务是否运行
ps aux | grep sqlrustgo

# 检查端口
sudo -S -p '' netstat -tlnp | grep 5432
```

### Q: 内存不足？

```bash
# 减小缓冲池
sqlrustgo serve --buffer-pool 1GB
```

### Q: 需要帮助？

- Issue: https://github.com/minzuuniversity/sqlrustgo/issues
- 文档: https://github.com/minzuuniversity/sqlrustgo/docs