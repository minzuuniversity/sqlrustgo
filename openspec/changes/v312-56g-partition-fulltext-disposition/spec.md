# Spec — V312-56G: Partition/FullText Disposition

## Overview

明确 Partition 和 FullText 是 4.0.0 生产能力、v3.12 受控教学/GMP keyword retrieval 能力，还是延期项。

## Specification

### Partition Disposition

#### Current State

| Component | Status |
|---|---|
| PartitionInfo (storage) | Implemented |
| ALTER TABLE SET PARTITIONED BY | Returns UNSUPPORTED |
| Conflict | Tests may conflict with unsupported behavior |

#### Decision Required

| Option | Action |
|---|---|
| DEFERRED to 4.0.0 | Document as DEFERRED, clear owner/expiry |
| v3.12 partial | Implement minimal supported subset with clear boundaries |

#### Validation

- `cargo test --test partition_e2e_test` must not conflict with unsupported behavior
- Storage-level tests must align with SQL-level unsupported errors

### FullText Disposition

#### Current State

| Component | Status |
|---|---|
| FullTextIndex (storage) | Implemented |
| MATCH/AGAINST (SQL) | May not be implemented |
| GMP keyword retrieval | Needs decision |

#### GMP Keyword Retrieval Decision

Is GMP keyword retrieval needed for v3.12?

| Answer | Action |
|---|---|
| Yes | Define controlled FullText subset for GMP, create fixtures |
| No | FullText stays DEFERRED until 4.0.0 or later |

#### If GMP needs keyword retrieval

Controlled subset:
- `MATCH(col) AGAINST('keyword')` basic support
- Controlled vocabulary (no arbitrary text search)
- Clear unsupported boundaries

## Boundaries

- README and release docs must explicitly state DEFERRED if not in v3.12
- No ambiguous PARTIAL without clear remediation path
