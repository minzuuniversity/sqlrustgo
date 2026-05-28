# SPEC: PostgreSQL Compatibility P0

## Issue #1467

## 1. Overview

This specification defines the PostgreSQL BI/Driver compatibility layer for SQLRustGo. The P0 (Phase 0) implementation provides basic PostgreSQL wire protocol support, enabling standard PostgreSQL clients and BI tools to connect to SQLRustGo.

## 2. Architecture

### 2.1 Module Structure

```
crates/executor/src/pg_compat/
├── mod.rs       # Module entry point and public exports
├── types.rs     # PostgreSQL OID to SQLRustGo type mapping
└── auth.rs      # Authentication handler (cleartext password)
```

### 2.2 Core Components

- **PgType**: PostgreSQL type definitions with OID mapping
- **PgAuth**: Simple cleartext password authentication handler
- **No external PostgreSQL library dependencies**

## 3. PostgreSQL Type Mapping (P0)

### 3.1 Supported OIDs

| OID | PostgreSQL Type | SQLRustGo Value |
|-----|-----------------|-----------------|
| 16 | BOOL | Boolean(bool) |
| 20 | INT8 | Integer(i64) |
| 21 | INT2 | Integer(i64) |
| 23 | INT4 | Integer(i64) |
| 25 | TEXT | Text(String) |
| 700 | FLOAT4 | Float(f64) |
| 701 | FLOAT8 | Float(f64) |
| 1043 | VARCHAR | Text(String) |
| 1424 | BYTEA | Blob(Vec<u8>) |
| 2950 | UUID | Text(String) |
| 3802 | JSONB | Text(String) |
| 3807 | JSONPATH | Text(String) |

### 3.2 Type Conversion Rules

- PostgreSQL INT2 → SQLRustGo Integer(i64)
- PostgreSQL INT4 → SQLRustGo Integer(i64)
- PostgreSQL INT8 → SQLRustGo Integer(i64)
- PostgreSQL FLOAT4/FLOAT8 → SQLRustGo Float(f64)
- PostgreSQL TEXT/VARCHAR → SQLRustGo Text(String)
- PostgreSQL BYTEA → SQLRustGo Blob(Vec<u8>)
- PostgreSQL BOOL → SQLRustGo Boolean(bool)

## 4. Authentication (P0)

### 4.1 Cleartext Password Authentication

- **AuthType**: `pg_compat::auth::AuthHandler`
- **Method**: Cleartext password (AuthenticationCleartextPassword)
- **No MD5/SCRAM support in P0**

### 4.2 Auth Flow

```
1. Client sends StartupMessage (user, database)
2. Server responds with AuthenticationCleartextPassword
3. Client sends PasswordMessage (password)
4. Server validates and responds with AuthenticationOk or ErrorResponse
```

## 5. Implementation Notes

### 5.1 Constraints

- No external `pg` or `postgres` crate dependencies
- Pure Rust implementation
- Must compile with `cargo build -p sqlrustgo-executor`
- Tests must pass with `cargo test -p sqlrustgo-executor --lib`

### 5.2 Future Phases (P1-P3)

- P1: Extended type support (numeric, timestamp, array)
- P2: SCRAM authentication
- P3: Full protocol implementation (COPY, prepared statements)

## 6. Acceptance Criteria

- [ ] SPEC_PG_COMPATIBILITY.md created at docs/releases/v3.6.0/
- [ ] pg_compat module created at crates/executor/src/pg_compat/
- [ ] types.rs implements OID to SQLRustGo Value mapping
- [ ] auth.rs implements cleartext password authentication
- [ ] cargo build -p sqlrustgo-executor succeeds
- [ ] cargo test -p sqlrustgo-executor --lib passes
- [ ] No external PostgreSQL library dependencies added
