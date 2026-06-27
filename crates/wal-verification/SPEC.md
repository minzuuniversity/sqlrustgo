# WAL Formal Verification Specifications

## Overview

This crate contains formal specifications for WAL (Write-Ahead Log) correctness properties. It provides TLA+ specifications and Rust verification code for:

1. **WAL State Machine** - WAL operations as a state machine
2. **Crash Recovery** - Recovery properties after crash
3. **Idempotency** - Safe replay of WAL entries
4. **Torn Write Prevention** - Atomic write guarantees
5. **Checkpoint Correctness** - Consistent checkpoint creation

## TLA+ Specifications

### WAL State Machine (WAL.tla)

```tla
(* WAL State Machine States *)
VARIABLE state

States == {
    "idle",
    "writing",
    "flushing",
    "committing",
    "checkpointing"
}

(* WAL Record Types *)
RecordTypes == {
    "BEGIN_TXN",
    "WRITE_ROW",
    "DELETE_ROW",
    "UPDATE_ROW",
    "COMMIT_TXN",
    "ABORT_TXN",
    "CHECKPOINT"
}

(* Initial state *)
Init == state = "idle"

(* State transitions *)
WriteRecord(rec) == /\ state = "idle"
                    /\ state' = "writing"

FlushRecord == /\ state = "writing"
               /\ state' = "flushing"

Commit == /\ state = "flushing"
          /\ state' = "committing"
          /\ state' = "idle"
```

### Key Invariants

```tla
(* WAL is never empty when not idle *)
Invariant1 == state /= "idle" => wal_length > 0

(* No torn writes - record is either fully written or not *)
Invariant2 == \A rec \in Records :
    rec.written => rec.flushed

(* Commit LSN is always >= any written record LSN *)
Invariant3 == commit_lsn >= Max({r.lsn : r \in Records})
```

## Rust Verification Code

### WAL Properties

```rust
pub enum WALState {
    Idle,
    Writing,
    Flushing,
    Committing,
    Checkpointing,
}

pub trait WALProperty {
    fn is_idempotent(&self) -> bool;
    fn is_atomic(&self) -> bool;
    fn lsn_monotonic(&self) -> bool;
}
```

### Crash Recovery Properties

```rust
pub enum CrashRecoveryProperty {
    /// After recovery, all committed transactions are present
    CommittedTransactionsPreserved,

    /// No uncommitted transactions appear after recovery
    UncommittedTransactionsRolledBack,

    /// WAL LSN sequence is continuous after recovery
    LSNSequenceContinuous,

    /// No phantom commits (transactions that shouldn't have committed)
    NoPhantomCommits,

    /// No lost updates (committed updates are not lost)
    NoLostUpdates,
}
```

### Verification Results

```rust
pub struct WALVerificationResult {
    pub property: WALProperty,
    pub holds: bool,
    pub counterexample: Option<String>,
    pub severity: VerificationSeverity,
}

pub enum VerificationSeverity {
    Critical,  // Must hold, database integrity depends on it
    Important, // Should hold, affects correctness
    Advisory,  // Best practice, minor impact
}
```

## Verification Checklist

### Before GA Release

- [ ] **Idempotent replay** - WAL can be replayed multiple times safely
- [ ] **Torn write prevention** - Partial writes are detected and rejected
- [ ] **LSN monotonicity** - LSN always increases
- [ ] **Checkpoint atomicity** - Checkpoint is either complete or not present
- [ ] **Recovery completeness** - All committed txns recovered, all uncommitted rolled back
- [ ] **Audit chain continuity** - Audit chain has no gaps after recovery
- [ ] **Signature preservation** - All signatures valid after recovery

### Critical Thresholds

| Property | Threshold | Severity |
|----------|-----------|----------|
| Idempotent replay | 100% | Critical |
| No torn writes | 100% | Critical |
| LSN monotonicity | 100% | Critical |
| Checkpoint atomicity | 100% | Critical |
| Recovery completeness | 100% | Critical |

## Formal Specifications

### 1. WAL Replay Idempotency

**Theorem**: Replaying WAL entries N times produces the same result as replaying once.

**Proof sketch**:
- Each WAL entry has a unique transaction ID
- Each row modification has a unique row ID
- Applying the same modification twice to the same row is idempotent
- Therefore, full WAL replay is idempotent

**Verification**: Property-based testing with:
- Generate random WAL sequence
- Replay twice
- Compare final state

### 2. Torn Write Detection

**Theorem**: A torn write (partial record written) is always detected.

**Mechanism**:
- Each WAL record has a header with length and checksum
- Record is only considered written if header + body + checksum are all valid
- Partial write fails checksum

**Verification**:
- Inject simulated partial writes
- Verify checksum detects corruption

### 3. Checkpoint Consistency

**Theorem**: A checkpoint is atomic - either fully present or not present.

**Mechanism**:
- Checkpoint has BEGIN and END markers
- END marker contains summary of all pages
- Recovery only uses checkpoint if both markers present

**Verification**:
- Simulate crash during checkpoint
- Verify recovery rejects partial checkpoint

## Integration with Crash Simulation

The WAL verification framework integrates with `sqlrustgo-crash-sim`:

```rust
use sqlrustgo_wal_verification::*;
use sqlrustgo_crash_sim::*;

// Create WAL-aware crash scenario
let scenario = CrashScenario::at(CrashPoint::WalFlush);

// Verify WAL properties after recovery
let verifier = WALRecoveryVerifier::new();
let result = verifier.verify_wal_properties();
```

## References

- TLA+ website: https://lamport.azurewebsites.net/tla/tla.html
- Leslie Lamport, "Specifying Systems"
- "Physical Resilience of Database Systems" - CMU Technical Report
