# executor-trigger-fire-on-dml Specification

## Purpose
TBD - created by archiving change v312-63-executor-batch-2. Update Purpose after archive.
## Requirements
### Requirement: FileStorage persists trigger metadata
`FileStorage::create_trigger` MUST insert the supplied `TriggerInfo` into an in-memory map and persist it to `<data_dir>/triggers.json`. The map is reloaded on construction.

#### Scenario: Trigger created on FileStorage is listed on the same instance
- GIVEN a `FileStorage` opened on an empty directory
- WHEN `create_trigger(info)` is called for table `t`
- THEN `list_triggers("t")` returns a vector containing the just-inserted trigger

#### Scenario: Trigger persists across FileStorage restarts
- GIVEN a `FileStorage` with one trigger registered on table `t`
- WHEN a new `FileStorage` is opened on the same directory
- THEN `list_triggers("t")` returns the trigger

### Requirement: DML paths trigger fires on insert
When a `BEFORE INSERT` or `AFTER INSERT` trigger is registered on a target table, the corresponding DML execute path MUST dispatch the trigger body.

#### Scenario: BEFORE INSERT trigger mutates the inserted row
- GIVEN a table `t(id, v)` and `BEFORE INSERT ON t` trigger that sets `v = 42`
- WHEN `INSERT INTO t VALUES (1, 99)` runs
- THEN the stored row is `(1, 42)` and the trigger body executed at least once

#### Scenario: AFTER INSERT trigger inserts into audit table
- GIVEN a table `t(id, val)` and a `log(id, msg)` audit table
- AND `AFTER INSERT ON t` trigger with body `INSERT INTO log(msg) VALUES ('fired')`
- WHEN `INSERT INTO t VALUES (1, 100)` runs
- THEN `SELECT count(*) FROM log` returns `1`

### Requirement: BinaryStorage / AppendOnlyStorage keep triggers in-memory
Storage backends other than FileStorage MUST keep triggers in an in-memory map so that the trigger executor can see them within the same process.

#### Scenario: BinaryStorage create_trigger then list_triggers returns the trigger
- GIVEN a `BinaryStorage` instance
- WHEN `create_trigger(info)` for table `t` then `list_triggers("t")` runs
- THEN the returned vector contains the trigger

