## ADDED Requirements

### Requirement: Triggers Fire on DML Operations

The trigger executor MUST fire BEFORE/AFTER triggers for INSERT/UPDATE/DELETE operations on tables that have matching `CREATE TRIGGER` definitions. The trigger body executes its DML (e.g., `UPDATE inventory SET stock = ...`) using the same storage engine and same transaction state as the outer DML statement.

#### Scenario: AFTER INSERT trigger fires
- **WHEN** a table `orders` has an AFTER INSERT trigger that does `UPDATE inventory SET stock = stock - NEW.quantity WHERE product_id = NEW.product_id`
- **AND** the user executes `INSERT INTO orders VALUES (1, 1, 10)`
- **THEN** the trigger fires
- **AND** `inventory.stock` for `product_id = 1` is decremented by 10

#### Scenario: AFTER UPDATE trigger fires
- **WHEN** a table `products` has an AFTER UPDATE trigger that inserts into `price_history`
- **AND** the user executes `UPDATE products SET price = 150 WHERE id = 1`
- **THEN** the trigger fires
- **AND** `price_history` contains a row with the old and new price

#### Scenario: BEFORE DELETE trigger fires
- **WHEN** a table `orders` has a BEFORE DELETE trigger that inserts into `cancelled_orders`
- **AND** the user executes `DELETE FROM orders WHERE id = 1`
- **THEN** the trigger fires
- **AND** `cancelled_orders` contains a row with the deleted order's data

### Requirement: Trigger DML Participates in User Transaction

When a trigger fires from within a user's `BEGIN ... COMMIT/ROLLBACK` block, the trigger's DML MUST be part of the same transaction as the user's DML. The trigger MUST NOT open a nested transaction. ROLLBACK undoes the trigger's modifications; COMMIT persists them.

#### Scenario: ROLLBACK undoes trigger DML
- **WHEN** a user begins a transaction
- **AND** the user executes `INSERT INTO orders VALUES (1, 1, 10)` which fires an AFTER INSERT trigger on `orders`
- **AND** the trigger body executes `UPDATE inventory SET stock = stock - NEW.quantity WHERE product_id = NEW.product_id`
- **AND** the user executes `ROLLBACK`
- **THEN** the order insert is undone
- **AND** the trigger's UPDATE on `inventory` is also undone
- **AND** the post-rollback state is identical to the pre-BEGIN state

#### Scenario: COMMIT persists trigger DML
- **WHEN** a user begins a transaction
- **AND** the user executes `INSERT INTO orders VALUES (1, 1, 10)` which fires an AFTER INSERT trigger
- **AND** the user executes `COMMIT`
- **THEN** the order insert is persisted
- **AND** the trigger's UPDATE on `inventory` is also persisted
- **AND** both modifications are visible to subsequent queries

### Requirement: Trigger DML in Autocommit Has Its Own Short Transaction

When a trigger fires from a DML statement that is NOT inside a user's transaction (i.e., the DML is autocommit), the trigger's DML MUST be executed in its own short transaction (BEGIN ... COMMIT) for atomicity. The trigger's DML is NOT visible mid-statement; it becomes visible only after the trigger body returns successfully.

#### Scenario: Autocommit INSERT fires trigger in its own tx
- **WHEN** there is no active user transaction
- **AND** the user executes `INSERT INTO orders VALUES (1, 1, 10)` which fires an AFTER INSERT trigger
- **THEN** the trigger body's DML is wrapped in a short transaction
- **AND** the trigger's DML is committed before the outer INSERT returns
- **AND** both the INSERT and the trigger's UPDATE are visible immediately after the statement

### Requirement: No Read-then-Write Lock Deadlock

The trigger executor MUST NOT hold a read lock on the storage RwLock when it needs to acquire a write lock in the same call path. If a function reads from the storage and then needs to write, it MUST drop the read guard before requesting the write lock.

#### Scenario: trigger_update releases read lock before write
- **WHEN** `execute_trigger_update` reads table data via `self.storage.read()` to build the `modified_rows` set
- **AND** it then calls `self.execute_dml_in_tx(...)` which needs `self.storage.write()`
- **THEN** the read lock is dropped before the write lock is requested
- **AND** the call completes without deadlock

### Requirement: NEW.col Expansion Before Parse

The trigger executor MUST expand `NEW.col` and `OLD.col` references in the trigger body SQL into literal values before passing the SQL to the parser. Without expansion, the parser would see unresolvable identifier references and the trigger would fail or hang.

#### Scenario: trigger body with NEW.col parses
- **WHEN** the trigger body is `UPDATE inventory SET stock = stock - NEW.quantity WHERE product_id = NEW.product_id`
- **AND** the trigger fires with `new_row = [1, 1, 10]`
- **THEN** `NEW.quantity` is replaced with the literal value `10`
- **AND** `NEW.product_id` is replaced with the literal value `1`
- **AND** the parser sees `UPDATE inventory SET stock = stock - 10 WHERE product_id = 1`
- **AND** the trigger executes successfully