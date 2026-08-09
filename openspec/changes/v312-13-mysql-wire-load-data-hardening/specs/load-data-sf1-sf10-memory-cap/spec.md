## ADDED Requirements

### Requirement: SF=1 LOAD DATA LOCAL INFILE with evidence

`LOAD DATA LOCAL INFILE` MUST complete against a TPC-H `lineitem` SF=1 fixture (6,001,215 rows) with all of the following constraints:

- Complete within `LOAD_DATA_SF1_DURATION_S` seconds (default 600).
- Result in `COUNT(*) = 6001215`.
- Produce a `SHA256` over the imported table that matches the
  reference hash stored at `tests/load_data/sf1/hash.txt`.
- Hold peak RSS ≤ `LOAD_DATA_SF1_PEAK_RSS_MB` MiB (default 4096).

The gate script `scripts/gate/check_v312_13_wire_load_data.sh` MUST run
this test only on `--ignored` mode.

#### Scenario: SF=1 load GREEN

- **GIVEN** the `lineitem_sf1.tsv` fixture exists
- **WHEN** an ephemeral server ingests the file via `LOAD DATA LOCAL INFILE`
- **THEN** `COUNT(*)` equals 6,001,215
- **AND** the SHA256 of the result table equals the reference hash
- **AND** peak RSS does not exceed 4 GiB
- **AND** the test completes within 600 s

### Requirement: SF=10 LOAD DATA LOCAL INFILE on tag builds

`LOAD DATA LOCAL INFILE` MUST complete against a TPC-H `lineitem` SF=10 fixture (60,013,775 rows) with all of the following constraints:

- Complete within `LOAD_DATA_SF10_DURATION_S` seconds (default 3600).
- Result in `COUNT(*) = 60013775`.
- Produce a `SHA256` matching the reference at
  `tests/load_data/sf10/hash.txt`.
- Hold peak RSS ≤ `LOAD_DATA_SF10_PEAK_RSS_MB` MiB (default 12288).

The gate script MUST run this test only on tags `v3.12.0-rc*` and
`v3.12.0-ga*`.

#### Scenario: SF=10 load GREEN on tag

- **GIVEN** `git describe --tags` matches `v3.12.0-*` AND the
  `lineitem_sf10.tsv` fixture exists
- **WHEN** the ephemeral server ingests the file
- **THEN** the row count, SHA256, and RSS constraints all hold

### Requirement: Memory cap fail-closed

The executor's batch loader MUST accept a `MAX_BATCH_MEMORY_MB` config and
MUST abort ingestion with an error message containing the substring
"memory cap exceeded" if the in-flight batch exceeds the cap.

#### Scenario: 1 MiB cap rejects SF=1 load

- **GIVEN** the server is configured with `MAX_BATCH_MEMORY_MB=1`
- **WHEN** a client issues `LOAD DATA LOCAL INFILE` for the SF=1 fixture
- **THEN** the server SHALL respond with an `ERR` packet whose message
  contains "memory cap exceeded"
- **AND** the table is left empty
