# mysql-server-wal-persistence Specification

## Purpose
TBD - created by archiving change fix-mysql-server-wal-bypass. Update Purpose after archive.
## Requirements
### Requirement: mysql-server DML Operations MUST Persist via WalStorage

The mysql-server crate's `do_command_loop` function and all entry points (handshake paths at lines 1457, 1509) MUST wrap `FileStorage` in `WalStorage` before constructing `ExecutionEngine`. All DML operations (INSERT, UPDATE, DELETE) executed through mysql-server MUST traverse the `WalStorage` layer, producing WAL entries with non-zero `tx_id` and monotonically increasing `LSN`.

#### Scenario: INSERT via mysql-server generates WAL entry
- **WHEN** a client connects to mysql-server on the listening port and executes `INSERT INTO test_table VALUES (1, 'foo')`
- **THEN** the system MUST create a `WalEntryType::Insert` entry in the WAL log with `tx_id` matching the active transaction
- **AND** the `next_lsn` counter MUST increment by 1
- **AND** the insert MUST be recoverable from the WAL after a simulated crash

#### Scenario: UPDATE via mysql-server generates WAL entry
- **WHEN** a client executes `UPDATE test_table SET name = 'bar' WHERE id = 1`
- **THEN** the system MUST create a `WalEntryType::Update` entry in the WAL log
- **AND** the entry MUST include the row filter (`WHERE id = 1`) and the new row values

#### Scenario: DELETE via mysql-server generates WAL entry
- **WHEN** a client executes `DELETE FROM test_table WHERE id = 1`
- **THEN** the system MUST create a `WalEntryType::Delete` entry in the WAL log
- **AND** the entry MUST include the row filter

#### Scenario: Transaction Commit emits WAL Commit entry
- **WHEN** a client issues `COMMIT` after a series of DML statements
- **THEN** the system MUST emit a `WalEntryType::Commit` entry referencing the same `tx_id`
- **AND** the entry MUST be flushed to the WAL file before the `OK` packet is sent to the client

#### Scenario: Transaction Rollback emits WAL Rollback entry
- **WHEN** a client issues `ROLLBACK` after a series of DML statements
- **THEN** the system MUST emit a `WalEntryType::Rollback` entry referencing the same `tx_id`
- **AND** the partial DML entries MUST be marked as aborted in the WAL log

### Requirement: do_command_loop Signature MUST Accept WalStorage-Wrapped Engine

The `do_command_loop` function signature in `crates/mysql-server/src/lib.rs` MUST accept `Arc<RwLock<WalStorage<FileStorage, FileBackedWalManager>>>` for the `storage` parameter and `Arc<RwLock<ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>>>` for the `engine` parameter. Direct use of `Arc<RwLock<FileStorage>>` and `Arc<RwLock<ExecutionEngine<FileStorage>>>` MUST NOT appear in mysql-server crate (except in test code paths that explicitly opt out via documentation comments).

#### Scenario: do_command_loop compiles with WalStorage types
- **WHEN** `cargo build -p sqlrustgo-mysql-server --all-features` is invoked
- **THEN** the build MUST succeed with zero errors
- **AND** the `do_command_loop` function signature MUST match the WalStorage-wrapped types

#### Scenario: Existing test paths still compile
- **WHEN** `cargo test -p sqlrustgo-mysql-server --all-features` is invoked
- **THEN** all existing test cases MUST continue to pass
- **AND** new tests for WAL persistence MUST also pass

### Requirement: Recovery Engine CAN Replay mysql-server WAL Entries

After a simulated crash and restart, the WAL recovery engine (existing component in `crates/transaction/`) MUST be able to replay WAL entries written by mysql-server, restoring all committed DML state to the `FileStorage` layer.

#### Scenario: Post-crash recovery restores INSERT
- **GIVEN** mysql-server has processed 5 INSERTs, 3 UPDATEs, 2 DELETEs, all committed
- **AND** the process is killed without clean shutdown
- **WHEN** mysql-server restarts and the recovery engine runs
- **THEN** the `FileStorage` MUST contain the final state reflecting all 5 INSERTs, 3 UPDATEs, 2 DELETEs
- **AND** no data MUST be lost

#### Scenario: Post-crash recovery discards uncommitted DML
- **GIVEN** mysql-server has processed 3 INSERTs, all in an open transaction (no COMMIT)
- **AND** the process is killed without clean shutdown
- **WHEN** mysql-server restarts and the recovery engine runs
- **THEN** the `FileStorage` MUST NOT contain any of the 3 INSERTs
- **AND** a `WalEntryType::Rollback` marker MUST be appended to the WAL (or the open transaction MUST be marked as aborted in the recovery logic)

