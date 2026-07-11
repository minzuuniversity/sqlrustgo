# Spec — cli-executor-parallelism-flag

> **Capability**: New CLI flag `--executor-parallelism=N` to opt into intra-query parallelism.
> **Default**: 1 (sequential, zero regression)
> **Env var**: `SQLRUSTGO_EXECUTOR_PARALLELISM` (alternative)

## ADDED Requirements

### Requirement: CLI flag declaration

The system SHALL expose `--executor-parallelism=<N>` as a long-form CLI argument on `sqlrustgo-mysql-server` and other server binaries.

#### Scenario: Flag visible in help
- **WHEN** running `sqlrustgo-mysql-server --help`
- **THEN** the help output includes:
  ```
  --executor-parallelism <EXECUTOR_PARALLELISM>
          Intra-query executor parallelism (1=sequential, N=parallel workers) [default: 1] [env: SQLRUSTGO_EXECUTOR_PARALLELISM]
  ```

#### Scenario: Default value is 1
- **WHEN** starting `sqlrustgo-mysql-server` without `--executor-parallelism`
- **THEN** the executor runs sequentially (parallel_degree = 1)

#### Scenario: Parse integer value
- **WHEN** starting with `--executor-parallelism=4`
- **THEN** `ExecutionEngine::set_parallel_degree(4)` is called before `serve()`
- **AND** the engine reports `parallel_degree = 4`

#### Scenario: Parse via environment variable
- **WHEN** `SQLRUSTGO_EXECUTOR_PARALLELISM=8` is set
- **AND** `--executor-parallelism` is NOT provided on CLI
- **THEN** the env var value (8) is used

#### Scenario: CLI overrides env var
- **WHEN** `SQLRUSTGO_EXECUTOR_PARALLELISM=8` is set
- **AND** `--executor-parallelism=2` is on CLI
- **THEN** the CLI value (2) takes precedence

### Requirement: Value validation

The system SHALL validate `--executor-parallelism` and reject invalid values.

#### Scenario: N=0 is rejected
- **WHEN** `--executor-parallelism=0`
- **THEN** the binary exits with non-zero status
- **AND** prints: `error: --executor-parallelism must be >= 1 (got 0)`

#### Scenario: Negative is rejected
- **WHEN** `--executor-parallelism=-1`
- **THEN** clap's auto-generated error is shown
- **AND** the binary exits with non-zero status

#### Scenario: Non-integer is rejected
- **WHEN** `--executor-parallelism=abc`
- **THEN** clap's auto-generated error is shown
- **AND** the binary exits with non-zero status

#### Scenario: Excessive N is capped
- **WHEN** `--executor-parallelism=10000`
- **THEN** the binary logs a warning: `executor-parallelism 10000 exceeds CPU count 10, capped to 10`
- **AND** `parallel_degree` is set to `min(N, num_cpus::get())`

### Requirement: REPL and exec subcommands

The REPL (`sqlrustgo repl`) and `sqlrustgo exec "<SQL>"` subcommands SHALL also respect `--executor-parallelism`.

#### Scenario: REPL inherits flag
- **WHEN** starting `sqlrustgo repl --executor-parallelism=4`
- **THEN** every SELECT executed in the REPL runs with parallel_degree=4
- **AND** `\set executor_parallelism 8` updates the value at runtime (additional sub-feature)

#### Scenario: exec one-shot respects flag
- **WHEN** running `sqlrustgo exec "SELECT * FROM big_table" --executor-parallelism=4`
- **THEN** the single SELECT runs with parallel_degree=4
- **AND** output is identical to `--executor-parallelism=1` (modulo ordering)
