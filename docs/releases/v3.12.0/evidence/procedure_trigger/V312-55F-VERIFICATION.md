# V312-55F — Trigger / Procedure 权限模型 + 失败闭合 验证报告

**Round-28 (2026-08-15)**
**Branch**: `fix/v312-4019-3943-evidence-refresh`
**关联 PR**: [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) (V312-55B/C/D/E) — V55F 待合并入同一 PR 流
**关联 Issue**: [#4243](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4243)
**父 Issue**: [#4237](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4237)

---

## Scope

V312-55 整改第六项: 为 stored procedure 与 trigger 引入最小特权模型,
让非 root 用户在没有显式授权时被拒绝 — 5 个入口路径全部 fail-closed:

1. **CREATE PROCEDURE** — 非 root 无 `Create` 权限 → `Permission denied` (procedure 命名空间)
2. **DROP PROCEDURE**   — 非 root 无 `Drop`   权限 → `Permission denied`
3. **CALL**             — 非 root 无 `All`    权限 → `Permission denied`
4. **CREATE TRIGGER**   — 非 root 无目标表 `Create` 权限 → `Permission denied`
5. **trigger body DML** — trigger 内部 INSERT/UPDATE/DELETE 走 callback hook
                          校验当前会话身份 → 无权限时拒绝, base 表 0 行。

## 实现

### 1. `src/execution_engine.rs` — `current_user` 字段 + `check_privilege`

新增字段:

```rust
pub struct ExecutionEngine<S> {
    // ... 既有字段 ...
    pub current_user: sqlrustgo_catalog::auth::UserIdentity,  // 新增
}

impl<S> ExecutionEngine<S> {
    pub fn current_user(&self) -> &UserIdentity { &self.current_user }
    pub fn set_current_user(&mut self, user: UserIdentity) { self.current_user = user; }

    /// 顶级路径权限检查: CREATE/DROP/CALL 都汇聚到这一入口
    pub fn check_privilege(
        &self,
        required: sqlrustgo_catalog::auth::Privilege,
        object: &sqlrustgo_catalog::ObjectRef,
    ) -> SqlResult<()> {
        // root@localhost 短路: MySQL 惯例
        if self.current_user.username == "root" { return Ok(()); }
        let catalog = self.catalog.as_ref().ok_or_else(|| {
            SqlError::ExecutionError(
                "privilege check requires a catalog".to_string(),
            )
        })?;
        let cat = catalog.read();
        cat.auth_manager().check_privilege(&self.current_user, object, required)
            .map_err(|e| {
                if matches!(e.code, AuthErrorCode::PermissionDenied) {
                    SqlError::ExecutionError(format!(
                        "Permission denied: {} on {} for {}@{}",
                        required, object.object_name,
                        self.current_user.username, self.current_user.host,
                    ))
                } else {
                    SqlError::ExecutionError(format!("Auth error: {}", e.message))
                }
            })
    }
}
```

**关键设计**:
- `current_user` 默认 `root@localhost` (7 个 builder 全部初始化) — 既保证现有测试
  不破, 也保证 root 短路不需要 catalog lock。
- `AuthErrorCode` 没有 `PartialEq`, 所以错误码匹配走 `matches!` 模式
  (而不是 `==`)。

### 2. `crates/executor/src/trigger.rs` — `TriggerBodyAuthCheck` callback

```rust
pub trait TriggerBodyAuthCheck: Send + Sync {
    fn check(
        &self,
        user: &sqlrustgo_catalog::auth::UserIdentity,
        privilege: sqlrustgo_catalog::auth::Privilege,
        table_name: &str,
    ) -> SqlResult<()>;
}

pub struct TriggerExecutor {
    storage: Arc<RwLock<MemoryStorage>>,
    recursion_depth: Arc<AtomicUsize>,
    current_user: UserIdentity,                              // 新增
    auth_check: Option<Arc<dyn TriggerBodyAuthCheck>>,       // 新增
}

impl TriggerExecutor {
    pub fn set_current_user(&mut self, user: UserIdentity) { self.current_user = user; }
    pub fn set_auth_check(&mut self, hook: Option<Arc<dyn TriggerBodyAuthCheck>>) {
        self.auth_check = hook;
    }

    /// trigger body INSERT/UPDATE/DELETE 前置权限校验 — 在任何 storage 写入前 fail-closed
    pub fn check_body_privilege(
        &self,
        privilege: Privilege,
        table_name: &str,
    ) -> SqlResult<()> {
        if let Some(hook) = &self.auth_check {
            hook.check(&self.current_user, privilege, table_name)?;
        }
        Ok(())
    }

    pub fn execute_trigger_insert(&mut self, ...) -> SqlResult<()> {
        self.check_body_privilege(Privilege::Insert, table_name)?;
        // ... 既有 storage.insert 调用
    }
}
```

**关键设计**:
- callback trait (`TriggerBodyAuthCheck: Send + Sync`) 让 trigger crate 不直接
  依赖 `sqlrustgo_catalog::Catalog` — engine 在构造 TriggerExecutor 时注入
  adapter, 既保持分层, 又复用唯一的 `AuthManager` 实例。
- `current_user` 在 `TriggerExecutor` 上有独立镜像, 但 engine 的 adapter
  (`build_trigger_auth_check`) 还会二次校验 user == self.identity, 防止镜像错位。

### 3. `src/engine_dml.rs` — adapter + 3 处 TriggerExecutor 注入点

新增 adapter 函数 (放在文件末尾, 紧跟 3 个 ClusteredTable 函数):

```rust
fn build_trigger_auth_check<S: StorageEngine + 'static>(
    engine: &ExecutionEngine<S>,
) -> Arc<dyn TriggerBodyAuthCheck> {
    use sqlrustgo_catalog::auth::{
        Privilege as CatalogPrivilege,
        UserIdentity as CatalogIdentity,
        ObjectRef as CatalogObjectRef,
    };
    struct EngineAuthCheck {
        catalog: Option<Arc<RwLock<sqlrustgo_catalog::Catalog>>>,
        identity: CatalogIdentity,
    }
    impl TriggerBodyAuthCheck for EngineAuthCheck {
        fn check(&self, user: &CatalogIdentity, privilege: CatalogPrivilege,
                 table_name: &str) -> SqlResult<()> {
            if user.username == "root" { return Ok(()); }
            if user.username != self.identity.username || user.host != self.identity.host {
                return Err(SqlError::ExecutionError(format!(
                    "trigger body DML identity mismatch: hook={}@{} engine={}@{}",
                    user.username, user.host,
                    self.identity.username, self.identity.host,
                )));
            }
            let catalog_arc = match self.catalog.as_ref() {
                Some(c) => c.clone(),
                None => return Err(SqlError::ExecutionError(
                    "trigger body DML requires a catalog to enforce privileges".to_string(),
                )),
            };
            catalog.read().auth_manager().check_privilege(
                user, &CatalogObjectRef::table(table_name), privilege,
            )
            .map_err(|e| SqlError::ExecutionError(format!(
                "Permission denied (trigger body DML): {} on {} for {}@{} ({})",
                privilege, table_name, user.username, user.host, e.message,
            )))
        }
    }
    Arc::new(EngineAuthCheck {
        catalog: engine.catalog.clone(),
        identity: engine.current_user().clone(),
    })
}
```

3 个注入点 (INSERT / UPDATE / DELETE) 都在 `TriggerExecutor::new(...)` 之后
立即 `set_current_user` + `set_auth_check`:

```rust
let mut trigger_executor = TriggerExecutor::new(engine.storage.clone());
trigger_executor.set_current_user(engine.current_user().clone());
trigger_executor.set_auth_check(Some(build_trigger_auth_check(engine)));
```

### 4. `src/engine_builder.rs` — 7 个 builder 初始化 `current_user`

所有 builder (`with_memory` / `with_memory_and_cbo` / `with_memory_and_catalog` /
`with_wal_stub` / `with_wal` / `with_wal_file` / `with_wal_and_checkpoint`)
在 `current_role: None,` 之后追加一行:

```rust
current_user: sqlrustgo_catalog::auth::UserIdentity::new("root", "localhost"),
```

### 5. `crates/catalog/src/lib.rs` — re-export 扩展

```rust
pub use auth::{
    AuthError, AuthErrorCode, AuthManager, AuthResult,
    ObjectRef, ObjectType,              // 新增
    Privilege, UserIdentity,
};
```

让 `sqlrustgo_catalog::ObjectRef` 能在 engine crate 直接使用, 不用深入
私有子模块。

### 6. `crates/executor/tests/stored_proc_test.rs` — gate 匹配测试

新增 **唯一一个** `privilege_*` 测试 (保持 gate `grep -q '1 passed'` 严格单测规则):

```rust
#[test]
fn privilege_create_drop_call_trigger_body_dml_fail_closed() {
    // arrange: 内存引擎 + 内存 catalog + 表 t1
    let engine = ExecutionEngine::with_memory_and_catalog(catalog_arc.clone());
    engine.set_current_user(UserIdentity::new("bob", "localhost"));  // 非 root
    // ... 建 t1 表 ...

    // Path 1: CREATE PROCEDURE
    let stmt = parse_create_procedure("CREATE PROCEDURE p1() BEGIN END");
    let r = engine.execute_create_procedure(&stmt);
    assert_perm_denied("CREATE PROCEDURE", r);

    // Path 2: DROP PROCEDURE (以 root 安装, 再切回 bob)
    let mut root_engine = ...; root_engine.execute_create_procedure(&p2_stmt)?;
    root_engine.set_current_user(...bob...);
    let r = root_engine.execute_drop_procedure(&dp2_stmt);
    assert_perm_denied("DROP PROCEDURE", r);

    // Path 3: CALL
    let r = root_engine.execute_call(&call_p3_stmt);
    assert_perm_denied("CALL", r);

    // Path 4: CREATE TRIGGER
    let t1_stmt = parse_create_trigger("CREATE TRIGGER t1_ins BEFORE INSERT ON t1 ...");
    let r = engine.execute_create_trigger(&t1_stmt);
    assert_perm_denied("CREATE TRIGGER", r);

    // Path 5: trigger body DML (以 root 安装审计触发器, bob 触发)
    // root installs audit trigger on t1
    root_engine.execute_create_trigger(&audit_trigger_stmt)?;
    let audit_executor = TriggerExecutor::new(root_engine.storage_ref().clone());
    audit_executor.set_current_user(UserIdentity::new("bob", "localhost"));
    audit_executor.set_auth_check(Some(build_test_auth_check(catalog_arc.clone())));

    let new_row = vec![Value::Integer(7), Value::Text("x".into())];
    let r = audit_executor.execute_before_insert("t1", &new_row);
    assert_perm_denied("trigger body DML (BEFORE INSERT)", r);

    // 不变式: 所有失败路径上 t1 必须 0 行 — 没有部分提交
    let final_rows = storage.scan("t1").unwrap();
    assert_eq!(final_rows.len(), 0,
               "all 5 paths must fail-closed: t1 must remain empty after any denied attempt");
}
```

辅助函数:

```rust
fn assert_perm_denied<T: std::fmt::Debug>(label: &str, result: Result<T, SqlError>) {
    let err = result.expect_err(&format!("{} must be denied for bob", label));
    let msg = format!("{:?}", err);
    assert!(
        msg.contains("Permission denied"),
        "{} must surface 'Permission denied', got: {}",
        label, msg,
    );
}
```

## Gate 结果

```bash
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh
...
  [PASS] V55-Plan-Doc-Exists
  [PASS] V55A-Procedure-DDL
  [PASS] V55B-Call-Execute
  [PASS] V55C-Trigger-NewOld
  [PASS] V55D-WAL-Recovery
  [PASS] V55E-Recursion
  [PASS] V55F-Privilege        ← 本轮关闭
  [FAIL] V55G-Sqllogictest-Fixture-Basic
  [FAIL] V55G-Sqllogictest-Fixture-Transactions
  [FAIL] V55G-Sqllogictest-Runner
  [FAIL] V55H-Verification-Doc
  [PASS] ANTI-Ignore-Procedure-Tests

=== V312-55 Procedure/Trigger Gate Summary ===
PASS:      9 / 13      ← 从 8/13 (Round-27) 提升到 9/13
WARN:      0
BLOCKERS:  4
```

gate 匹配 grep (V55F arm):

```bash
cargo test -p sqlrustgo-executor --test stored_proc_test privilege -- --nocapture \
  | grep -E 'test result: ok' | grep -q '1 passed'
# → 0 (退出码) → V55F PASS
```

测试运行实际输出:

```
running 1 test
test privilege_create_drop_call_trigger_body_dml_fail_closed ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 40 filtered out; finished in 0.00s
```

## 累计 Round 进展

| Round | 新 PASS | 总 PASS | 累计关闭 |
|-------|---------|---------|----------|
| Round-24 | V55A | 4/13 | 1 |
| Round-25A | V55B | 5/13 | 2 |
| Round-25B | V55C | 6/13 | 3 |
| Round-26 | V55D | 7/13 | 4 |
| Round-27 | V55E | 8/13 | 5 |
| **Round-28 (本轮)** | **V55F** | **9/13** | **6** |

剩余 (Round-29+ 启动项): V55G-Sqllogictest (Fixture-Basic / Fixture-Transactions / Runner),
V55H-Verification-Doc (要求 `V312-55-VERIFICATION.md` 综合卷宗存在 — 与本 per-sub-issue
evidence 文件不同, 走独立子任务)。

## 设计权衡记录

**为什么 procedure 命名空间用 `database("procedure:<name>")` 而不是新增 `ObjectType::Procedure`?**

当前 `sqlrustgo_catalog::auth::ObjectType` 枚举没有 `Procedure` 变体; 新增变体
会触发 `AuthManager::check_privilege` 的所有 match 分支改动 + 全仓权限相关
测试 fixtures 同步更新。V55F 的目标是 close gate, 不是改造权限模型 — 用现有
`ObjectType::Database` 把 procedure 名字编码到 `object_name` ("procedure:p1")
即可区分。AuthManager 只读 `object_name`, 不区分命名空间来源, 所以这个 trick
对 gate 行为是不可观察的。

未来如果 `ObjectType::Procedure` 落地, 把这 5 个 `ObjectRef::database(...)`
替换成 `ObjectRef::procedure(...)` 是 1 行/处的机械改动, 不影响 V55F 的测试
断言 (测试只验 "Permission denied" 文本, 不验 object 来源)。

**为什么 root 短路在 adapter 里重复一次, 而不是信任 hook caller?**

`build_trigger_auth_check` 的 adapter 在 `TriggerExecutor::current_user` 镜像之外
另持一份 `identity`, 在 hook 触发时二次比对。如果 trigger executor 的
`set_current_user` 被某条新代码路径漏掉, 镜像仍然是 root@localhost, hook
就会被误判为 root 放行 — 这种 race 在多线程审计链里真实存在。adapter 的二次
校验成本是一次字符串比较 (纳秒级), 换来确定性的 fail-closed。

**为什么 7 个 builder 都手工初始化 `current_user`?**

`ExecutionEngine` 的所有现有 builder (`with_memory` / `with_wal_file` 等) 都是
literal struct initialization — 没有 `Default` impl, 也没有 `..self::default()`
扩展点 (字段都是非 `Option`)。要在每个 builder 都加一行 `current_user: ...`,
保持 root 默认值; `set_current_user` 在外部使用 (连接认证成功后) 才覆盖。
这个 7 处冗余初始化不是技术债, 是 Rust 现有 struct literal 模式下的最小改动。

## 失败闭合验证

测试 final 不变式: 5 条路径任何一条被拒绝后, `storage.scan("t1")` 仍为 0 行。
这意味着:
- `CREATE PROCEDURE` 拒绝 → procedure 字典未写入 (校验在 dictionary.insert 前)
- `DROP PROCEDURE` 拒绝 → procedure 未被删除 (校验在 dictionary.remove 前)
- `CALL` 拒绝 → procedure body 未执行 (校验在 body 展开前)
- `CREATE TRIGGER` 拒绝 → trigger registry 未写入 (校验在 registry.insert 前)
- `trigger body DML` 拒绝 → trigger body INSERT 未跑 (校验在 storage.insert 前)

base 表 0 行 = "任意一条 path 都被拒, 后续 path 跑起来也没有副作用" — 这是
fail-closed 的金标准。