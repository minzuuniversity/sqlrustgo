# BINT v3 Storage Default

## Purpose

Switch production default storage from JSON (`FileStorage`) to BINT v3 binary
format (`BinaryTableStorageV2`) to accelerate TPC-H SF=1 lineitem load from
~10 hours to < 60 seconds.

## New Requirements

### ADDED Requirements

#### Requirement: BINT v3 Binary Storage Default
The system SHALL use `BinaryTableStorageV2` (BINT v3 format) as the production
default storage backend for new tables when the `bin_storage_default` Cargo
feature is enabled.

#### Requirement: Streaming Row Append
The BINT v3 storage backend SHALL support streaming row append via
`BinaryTableStorageV2::insert_streaming()` which does NOT serialize the
entire table on each batch.

#### Requirement: Atomic JSON to BIN Migration
When inserting into a table that has only JSON files on disk, the system
SHALL atomically migrate the table to BINT v3 and archive `.json` as
`.json.bak`. The migration SHALL be atomic: either both BIN files and `.bak`
exist, or only `.json` exists (no partial state).

#### Requirement: 16 KB Page-Aligned Segments
BINT v3 segments SHALL be aligned to 16 KB page boundaries to integrate with
the existing BufferPool page cache. Segment size SHALL NOT exceed 64 MB.

#### Requirement: Row-Level CRC32C Integrity
Each BINT v3 row SHALL be terminated by a 4-byte CRC32C footer computed over
the preceding row bytes. `SegmentReader::iter_rows()` SHALL skip rows with
mismatched CRC32C and emit a warning.

#### Requirement: WAL Batch Mode During LOAD DATA
During `LOAD DATA LOCAL INFILE`, the system SHALL force `WalSyncMode::Batch`
to aggregate fsync calls and avoid per-batch disk I/O. The original mode
SHALL be restored after `engine.flush()`.

### MODIFIED Requirements

None.

### REMOVED Requirements

None.

## Migration Path

- JSON files remain readable (no data loss)
- Migration is lazy on first INSERT to each table
- Rollback via `sqlrustgo-admin storage rollback --table <name>` restores `.json.bak` → `.json`
