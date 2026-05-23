# Web Dashboard 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 完成 Issue #78 Web Dashboard 的 full integration —— 修复路径硬编码、集成真实审计数据源、更新版本号

**Architecture:** 混合方案：dashboard/server.py 通过 subprocess 调用 gmp-retrieve CLI；CLI 内嵌调用 gmp-storage lib 直接查询审计表；前端动态渲染真实数据

**Tech Stack:** Python stdlib HTTP server, gmp-storage (FileStorage), gmp-cli (clap), native HTML/JS

---

## 前置：拉取最新代码

**Step 1: 拉取 develop/v1.2.0 最新提交**

```bash
git checkout develop/v1.2.0 && git pull origin develop/v1.2.0
```

预期：无冲突，fast-forward

---

## Task 1: gmp-storage 增加审计表 + demo 数据

**Files:**
- Modify: `crates/gmp-storage/src/sqlrustgo_adapter.rs:55-100`

**Step 1: 在 `SqlrustgoAdapter::init_gmp_schema` 中添加审计表**

在 `chunks_table` 创建后插入：

```rust
// Create audit chain table
let audit_table = TableInfo {
    name: "gmp_audit_chain".to_string(),
    columns: vec![
        ColumnDefinition::new("id", "INTEGER"),
        ColumnDefinition::new("audit_id", "TEXT"),
        ColumnDefinition::new("work_order_id", "TEXT"),
        ColumnDefinition::new("timestamp", "INTEGER"),
        ColumnDefinition::new("action", "TEXT"),
        ColumnDefinition::new("actor", "TEXT"),
        ColumnDefinition::new("detail", "TEXT"),
        ColumnDefinition::new("signature", "TEXT"),
        ColumnDefinition::new("result", "TEXT"),
    ],
    ..Default::default()
};
engine.create_table(&audit_table).context("Failed to create gmp_audit_chain table")?;
```

**Step 2: 添加审计记录插入方法**

在 `impl SqlrustgoAdapter` 末尾（flush 方法后）添加：

```rust
/// Insert an audit chain record
pub fn insert_audit_record(
    &self,
    audit_id: &str,
    work_order_id: &str,
    timestamp: i64,
    action: &str,
    actor: &str,
    detail: &str,
    signature: Option<&str>,
    result: &str,
) -> Result<()> {
    let record = vec![
        Value::Integer(0), // id auto
        Value::Text(audit_id.to_string()),
        Value::Text(work_order_id.to_string()),
        Value::Integer(timestamp),
        Value::Text(action.to_string()),
        Value::Text(actor.to_string()),
        Value::Text(detail.to_string()),
        signature.map(|s| Value::Text(s.to_string())).unwrap_or(Value::Null),
        Value::Text(result.to_string()),
    ];
    let mut engine = self.engine.lock().unwrap();
    engine.insert("gmp_audit_chain", vec![record])?;
    Ok(())
}

/// Scan all audit records, sorted by timestamp descending
pub fn scan_audit_chain(&self) -> Result<Vec<Record>> {
    let engine = self.engine.lock().unwrap();
    Ok(engine.scan("gmp_audit_chain")?)
}

/// Insert demo audit data if table is empty
pub fn insert_demo_audit_data(&self) -> Result<()> {
    let records = self.scan_audit_chain()?;
    if !records.is_empty() {
        return Ok(()); // already has data
    }

    let demos = vec![
        ("WE-20260522-001", 1747888338000i64, "工单创建", "operator_zhang", r#"{"task":"称量任务 #WE-001","operator":"张三"}"#, Some("ZhangSan#sha256:abc123"), "通过"),
        ("WE-20260522-001", 1747888385000i64, "OCR识别", "system", r#"{"device":"BAL-001","result":"识别成功"}"#, None, "通过"),
        ("WE-20260522-001", 1747888392000i64, "规则校验", "system", r#"{"dsl":"weigh_material","result":"校验通过"}"#, None, "通过"),
        ("WE-20260522-001", 1747888425000i64, "主管审批", "supervisor_wang", r#"{"level":1,"result":"审批通过"}"#, Some("WangWu#sha256:def456"), "通过"),
        ("WE-20260522-001", 1747888442000i64, "质量部审批", "quality_li", r#"{"level":2,"result":"第二级审批通过"}"#, Some("LiLiu#sha256:ghi789"), "通过"),
    ];

    for (i, (wo, ts, action, actor, detail, sig, result)) in demos.into_iter().enumerate() {
        self.insert_audit_record(
            &format!("AUDIT-{:03}", i + 1),
            wo,
            ts,
            action,
            actor,
            detail,
            sig,
            result,
        )?;
    }
    Ok(())
}
```

**Step 3: 更新 `init_gmp_schema` 调用 demo 数据插入**

在 `create_table` 块最后（`chunks_table` 创建后）添加：

```rust
// Seed demo audit data
self.insert_demo_audit_data()?;
```

**Step 4: 验证编译**

```bash
cd /Users/liying/workspace/GMP-Platform && cargo check -p gmp-storage 2>&1 | tail -20
```

预期：编译通过，无 error

**Step 5: 提交**

```bash
git add crates/gmp-storage/src/sqlrustgo_adapter.rs
git commit -m "feat(gmp-storage): add gmp_audit_chain table with demo data"
```

---

## Task 2: gmp-cli 增加 audit 子命令

**Files:**
- Modify: `crates/gmp-cli/src/main.rs:21-50` (添加 Commands enum)
- Modify: `crates/gmp-cli/src/main.rs` (添加 audit handler)

**Step 1: 在 `Commands` enum 中添加 Audit 变体**

在 `Search` 变体后添加：

```rust
    /// Query audit chain records
    Audit {
        /// Work order ID to filter (optional)
        #[arg(long)]
        work_order_id: Option<String>,

        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,
    },
```

**Step 2: 在 `main.rs` 中添加 audit 处理逻辑**

找到 `match &cli.command` 的地方，在 `Commands::Search` 分支后添加：

```rust
Commands::Audit { work_order_id, db } => {
    use gmp_storage::StorageEngine;

    let config = gmp_storage::StorageConfig {
        data_dir: db.clone(),
        db_path: db,
        embedding_dim: 1024,
        use_sqlrustgo_storage: true,
    };
    let storage = gmp_storage::StorageEngine::new(config)
        .context("Failed to init storage")?;

    let records = storage.scan_audit_chain()?;
    let mut filtered: Vec<_> = if let Some(ref wo) = work_order_id {
        records.into_iter().filter(|r| {
            r.get(2).map(|v| v.to_string().contains(wo)).unwrap_or(false)
        }).collect()
    } else {
        records
    };

    // Sort by timestamp descending (newest first)
    filtered.sort_by(|a, b| {
        let ta = a.get(3).and_then(|v| v.as_integer()).unwrap_or(0);
        let tb = b.get(3).and_then(|v| v.as_integer()).unwrap_or(0);
        tb.cmp(&ta)
    });

    for rec in filtered {
        let fields: Vec<String> = rec.iter().map(|v| v.to_string()).collect();
        println!("{}", fields.join("|"));
    }
}
```

**Step 3: 在 StorageEngine impl 中暴露 scan_audit_chain**

检查 `crates/gmp-storage/src/storage.rs` 中 `StorageEngine` impl 是否有 `scan_audit_chain` 公共方法。如果没有，在 `StorageEngine` impl 块中添加：

```rust
/// Scan all audit chain records
pub fn scan_audit_chain(&self) -> Result<Vec<sqlrustgo_types::Value>> {
    self.adapter.scan_audit_chain()
}
```

**Step 4: 验证编译**

```bash
cargo check -p gmp-cli 2>&1 | tail -30
```

预期：编译通过。如果有错误，检查 `scan_audit_chain` 是否在正确位置暴露

**Step 5: 提交**

```bash
git add crates/gmp-cli/src/main.rs crates/gmp-storage/src/storage.rs
git commit -m "feat(gmp-cli): add audit subcommand to query gmp_audit_chain"
```

---

## Task 3: dashboard/server.py 路径自适应 + 版本更新

**Files:**
- Modify: `dashboard/server.py`

**Step 1: 改为相对路径自动检测**

将文件顶部的硬编码配置改为：

```python
import os, sys

# Auto-detect GMP directory (parent of dashboard/)
_SCRIPT_DIR = Path(__file__).resolve().parent
GMP_DIR = _SCRIPT_DIR.parent
GMP_DB = GMP_DIR / "data" / "gmp.db"
GMP_CLI = GMP_DIR / "target" / "release" / "gmp-retrieve"
```

注意：删除 `from pathlib import Path` 中的 `Path` — 已在 import 中

**Step 2: 更新版本号**

将 `handle_stats` 返回的 `"platform_version": "v1.1.0"` 改为 `"v1.2.0"`，`"target_version": "v1.2.0"` 改为 `"v1.3.0"`

**Step 3: 提交**

```bash
git add dashboard/server.py
git commit -m "fix(dashboard): use relative auto-detect paths, update version to v1.2.0"
```

---

## Task 4: dashboard/index.html 动态渲染审计数据

**Files:**
- Modify: `dashboard/index.html`

**Step 1: 更新 `handle_audit` 调用改为 fetch 真实数据**

在 `<script>` 中找到 `handle_audit` 调用的地方（审计 Tab 被点击时），改为：

```javascript
// 在 switchTab 函数中，审计 tab 被激活时拉取真实数据
const _originalSwitchTab = switchTab;
window.switchTab = function(name) {
    _originalSwitchTab(name);
    if (name === 'audit') {
        loadAuditData();
    }
};

async function loadAuditData() {
    try {
        const resp = await fetch('/api/audit');
        const data = await resp.json();
        renderAuditTimeline(data.entries || []);
    } catch (e) {
        // Fallback to demo
        renderAuditTimeline([]);
    }
}
```

**Step 2: 添加 `renderAuditTimeline` 函数**

在 `renderResults` 函数后添加：

```javascript
function renderAuditTimeline(entries) {
    const timeline = document.getElementById('auditTimeline');
    if (!entries || entries.length === 0) {
        // Keep demo data if no real data
        return;
    }
    timeline.innerHTML = '';
    entries.forEach(entry => {
        const item = document.createElement('div');
        item.className = 'audit-item';
        const time = new Date(entry.time).toLocaleString('zh-CN');
        item.innerHTML = `
            <div class="audit-time">${time}</div>
            <div class="audit-text">${escapeHtml(entry.action)} — ${escapeHtml(entry.detail)}</div>
            <div class="audit-user">${escapeHtml(entry.user)}</div>
        `;
        timeline.appendChild(item);
    });
}
```

**Step 3: 提交**

```bash
git add dashboard/index.html
git commit -m "feat(dashboard): dynamic audit timeline from API"
```

---

## Task 5: 更新 Issue #78 状态

**Step 1: 关闭 Issue #78**

```bash
curl -s -X PATCH "http://192.168.0.252:3000/api/v1/repos/openclaw/GMP-Platform/issues/78" \
  -H "Content-Type: application/json" \
  -u "openclaw:details8848" \
  -d '{"state": "closed"}'
```

**Step 2: 更新 Issue #77 (v1.2.0 开发计划)**

将 #78 状态从 "todo" 标记为 "done"，注明 PR 分支

**Step 3: 创建 PR**

```bash
git checkout develop/v1.2.0 -b fix/issue-78-dashboard-full-integration
git merge --no-edit feature/issue-78-dashboard-phase1 2>&1 || true
git push origin fix/issue-78-dashboard-full-integration
```

---

## 验证清单

- [ ] `cargo check -p gmp-storage` 通过
- [ ] `cargo check -p gmp-cli` 通过
- [ ] `dashboard/server.py` 路径为相对路径
- [ ] `/api/audit` 返回真实 demo 数据
- [ ] Issue #78 关闭
- [ ] PR 创建成功
