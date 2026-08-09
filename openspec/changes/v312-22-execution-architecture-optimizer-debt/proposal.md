## Why

V312-22 addresses execution architecture and optimizer debt from v3.6-v3.10 cycles:
1. DML path bypass verification - confirm all DML goes through PhysicalPlan/LocalExecutor
2. VTU/Parallel/SIMD main path capability verification
3. Hash Semi Join, Anti Join, subquery decorrelation, CBO/histogram status
4. Execution path invariant script output
5. Q4/correlation subquery benchmark baseline

## What Changes

### Invariant Verification
- Run existing `check_arch3_no_bypass.sh` to verify DML path integrity
- Document any bypass detected

### Parallel/SIMD Path
- Verify `ParallelVolcanoExecutor` is the main execution path
- Confirm `--executor-parallelism` CLI flag works

### Optimizer Debt Assessment
- Hash Semi Join: status assessment
- Anti Join: status assessment
- Subquery decorrelation: status assessment
- CBO/histogram: status assessment
- Document each as: IN_PROGRESS, DEFERRED, CLOSED, or NOT_PLANNED

## Capabilities

### New Capabilities
- `execution-invariant-verification`: Script output verifying DML path integrity
- `optimizer-debt-disposition`: Documented status for each optimizer feature

## Impact

### Affected Modules
- `crates/executor` - ParallelVolcanoExecutor integration
- `crates/optimizer` - CBO, decorrelation
- Gate scripts - `check_arch3_no_bypass.sh`
