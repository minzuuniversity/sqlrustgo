# Spec — V312-56D: Prepared Statement / Wire Protocol Teaching

## Overview

整理 COM_QUERY、COM_STMT_PREPARE/EXECUTE、text/binary result、error packet、reset、LOAD DATA 的受控路径为可教学、可验证的实验。

## Specification

### Prepared Statement Roundtrip

#### Success Path

```
COM_STMT_PREPARE: INSERT INTO t VALUES (?)
COM_STMT_EXECUTE: [1]
COM_STMT_EXECUTE: [2]
COM_STMT_EXECUTE: [3]
COM_STMT_CLOSE
```

#### Error Paths

| Error | Condition |
|---|---|
| Wrong param count | prepare 2 params, execute with 1 |
| Type mismatch | prepare expects INT, execute with STRING |

### Wire Protocol Packets

#### COM_QUERY

```
| 1 byte     | N bytes           |
| COM_QUERY  | query string       |
```

#### COM_STMT_PREPARE

```
| 1 byte     | N bytes           |
| COM_STMT_  | query string       |
| PREPARE    |                   |
```

#### COM_STMT_EXECUTE

```
| 1 byte     | 4 bytes    | 1 byte | N bytes      |
| COM_STMT_  | statement_ | NULL-  | param values |
| EXECUTE    | id         | flags  |              |
```

#### Error Packet

```
| 1 byte    | 2 bytes    | 1 byte   | variable    |
| 0xFF      | error code | sql state| message     |
```

### LOAD DATA

Fixture includes:
- CSV file with N rows
- Expected row count after LOAD DATA
- Expected SHA256 hash of loaded data
- Boundary: empty file, large file, special characters

## Boundaries

- TLS/compression: DEFERRED if not ready for v3.12 Beta
- DEFERRED items must have owner/expiry/closing criteria
