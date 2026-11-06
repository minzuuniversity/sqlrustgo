> 这是 4.1.0 阶段看板行 **BIN-1**（源：`docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl`）。
> 本 ISSUE 只做 discover / measure / verify / report（AGENTS.md §2 红线条款）；怎么修、`--storage binary` 4.1.0 要不要继续对外提供，由 Owner 定。

## 一句话

`--storage binary` 下，`CREATE DATABASE` 与 `CREATE TRIGGER` 都返回 OK，但客户端能观测到它们**根本没生效** —— 典型「假成功」。

## 实测（2026-11-06，真 server + 真 mysql 客户端，同日复跑两次，结论一致）

- 被测：`target/debug/sqlrustgo-mysql-server`（debug 构建），`serve --host 127.0.0.1 --storage binary --wal-sync every`
- 客户端：`/opt/homebrew/bin/mysql`（MariaDB 客户端，非 Oracle mysql）
- 探针：`audit/v4.1.0/binmode_probe.py`（本仓新增，逐语句记 rc/stdout/stderr，并把服务端 **info 级**日志落盘后一起判）

| 语句 | 返回码 | 客户端侧可观测结果 |
|---|---|---|
| `CREATE DATABASE IF NOT EXISTS probeb` | 0 | `SHOW DATABASES` 仍只有 `default` ⇒ 库没建成 |
| `CREATE TABLE probeb.t (...)` | 1 | `BinaryTableStorage has no database namespace`（这条至少是**真报错**） |
| `CREATE TABLE pb_t2 (...)`（默认库） | 0 | 之后 INSERT / SELECT 正常 ⇒ 默认库确实可用，说明上一条不是「全存储不可用」 |
| `CREATE TRIGGER pb_trig BEFORE INSERT ON pb_t2 FOR EACH ROW SET NEW.v='fired'` | 0 | 之后 INSERT 存进去的是 `orig`，`fired` 一次都没出现 ⇒ 触发器没执行 |
| `DROP TRIGGER pb_trig` | 1 | `Trigger 'pb_trig' not found` ⇒ 触发器**从未登记**（与上一行的 rc=0 互相矛盾） |

服务端自己在 info 日志里写下的自认行（同一次运行出现 3 次）：

```
ERROR sqlrustgo_executor::trigger: TriggerExecutor::new: storage has no WAL enabled — triggers will be silently skipped (binary mode)
```

「静默跳过」只存在于日志里：语句照返 OK，调用方拿不到任何信号。

## 复现（数字都来自这条命令的 JSON）

```bash
/usr/bin/python3 audit/v4.1.0/binmode_probe.py --out test_results/binmode
```

退出码 0 只表示「观测完成」；缺陷与否看 JSON 里的 `false_success_count` / `false_success`。本轮实测：

```
create_database_rc = 0        create_database_effective = False
create_trigger_rc  = 0        drop_trigger_rc = 1
final_rows = {'1':'orig','2':'orig','3':'orig','4':'orig'}   trigger_actually_fired_rows = []
false_success_count = 3       trigger_self_admission_log_lines = 3 条（见上）
```

另一路独立证据（不同探针，不同代码路径）：

```bash
/usr/bin/python3 audit/v4.1.0/capability_smoke.py run --out test_results/capability_smoke/run-archive
```

其 `arm_nonproduction`（`--storage binary` 判别臂）里 `ddl_error: CREATE DATABASE 报成功但未生效：SHOW DATABASES=['default']`，该臂 findings=11；生产链（`--storage file --wal-sync every`）同一条探针 findings=0。归档见 `audit/v4.1.0/runs/20261106-cap-smoke-001`（首跑）与 `.../20261106-cap-smoke-002`（本轮）。

## 影响

「报成功但没生效」是这套系统里最贵的一类缺陷：CLI、测试、上层集成默认只看返回码，就会把没落地的东西当成已落地。这与 #5525（`restore` 报成功但不写盘）是**同一形状**，只是发生在存储层 DDL / 触发器上；与 #5478（BinaryTableStorage 故障注入）**不是**同一件事——那条测的是错误注入路径，本条测的是「正常路径返回 OK 却没有效果」。

## 验收判据（可证伪）

1. `CREATE DATABASE` 在 binary 存储上：要么真的建成（`SHOW DATABASES` 能看见），要么明确返回错误。不得继续返回 OK。
2. `CREATE TRIGGER` 在 binary 存储上：要么登记并真的执行，要么明确报错。`CREATE TRIGGER` 返 OK 而 `DROP TRIGGER` 报 `not found` 这种自相矛盾必须消失。
3. 触发器被跳过不得只体现在日志里 —— 调用方必须能从返回码或结果里看到。
4. 修完复跑：`binmode_probe.py` 的 `false_success_count` 必须降到 0；`capability_smoke.py run` 的 binary 臂**仍必须判红**（它测的是「把生产存储换成非生产必须更红」，判别力不得丢）。
5. 注意覆盖面：capability_smoke 的 binary 臂在 `CREATE TABLE smoke.t` 就失败，走不到触发器路径 ⇒ 触发器那一维**只有** `binmode_probe.py` 覆盖，不得声称「capability_smoke 已覆盖」。

## 关联

- 看板：`docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md` 行 `BIN-1`
- 同形状：#5525（restore 假成功）；不同事：#5478（BinaryTableStorage 故障注入）
