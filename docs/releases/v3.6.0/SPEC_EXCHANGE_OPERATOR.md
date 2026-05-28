# Exchange Operator Interface - Specification

## Issue
Issue #1465: Exchange Operator Interface

## Overview

The Exchange Operator provides a unified interface for data distribution in distributed query execution. It extends the existing `Operator` trait from the pipeline module to support different exchange patterns: Local, Broadcast, and Shuffle exchanges with various partitioning strategies.

## Architecture

### Relationship to Existing Components

```
┌─────────────────────────────────────────────────────┐
│                  Pipeline Module                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
│  │  Operator   │  │  Pipeline   │  │ ChunkOrDone │ │
│  │   trait     │  │             │  │             │ │
│  └──────┬──────┘  └─────────────┘  └─────────────┘ │
└─────────┼───────────────────────────────────────────┘
          │ extends
          ▼
┌─────────────────────────────────────────────────────┐
│                 Exchange Module                      │
│  ┌─────────────────────┐  ┌─────────────────────┐  │
│  │ ExchangeOperator    │  │ Partitioning        │  │
│  │ (extends Operator)  │  │ - HashPartition     │  │
│  │                     │  │ - RangePartition    │  │
│  │                     │  │ - RoundRobin        │  │
│  └─────────────────────┘  └─────────────────────┘  │
│  ┌─────────────────────┐  ┌─────────────────────┐  │
│  │ ExchangeType        │  │ Exchange Trait      │  │
│  │ - Local             │  │ - LocalExchange     │  │
│  │ - Broadcast         │  │ - BroadcastExchange│  │
│  │ - Shuffle           │  │                    │  │
│  └─────────────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

## Functionality Specification

### 1. ExchangeOperator Trait

The `ExchangeOperator` trait extends the base `Operator` trait with distributed execution capabilities:

```rust
pub trait ExchangeOperator: Operator {
    /// Returns the number of partitions this exchange produces.
    fn partition_count(&self) -> usize;
    
    /// Returns the partition ID for the current thread/context.
    fn current_partition(&self) -> usize;
    
    /// Returns the exchange type being performed.
    fn exchange_type(&self) -> ExchangeType;
}
```

### 2. ExchangeType Enum

Defines the different exchange patterns:

```rust
pub enum ExchangeType {
    /// No exchange - data stays local
    Local,
    /// Broadcast - all data sent to all partitions
    Broadcast,
    /// Shuffle - data redistributed based on partitioning
    Shuffle,
}
```

### 3. Partitioning Strategies

```rust
pub enum Partitioning {
    /// Hash-based partitioning on specified columns
    HashPartition { columns: Vec<String> },
    /// Range-based partitioning on specified columns
    RangePartition { columns: Vec<String> },
    /// Round-robin distribution
    RoundRobin,
}
```

### 4. Core Methods

| Method | Description |
|--------|-------------|
| `open()` | Initialize the exchange, set up partition context |
| `next_chunk()` | Return the next chunk from the appropriate partition |
| `close()` | Clean up exchange resources |
| `partition_count()` | Return number of output partitions |
| `current_partition()` | Return current partition ID |
| `exchange_type()` | Return the type of exchange |

## Implementations

### LocalExchange

- Data remains on the local partition
- No network transfer
- partition_count() returns 1

### BroadcastExchange

- All data is sent to all partitions
- Useful for broadcast joins and CTE materialization
- partition_count() returns the configured number of receivers

## Module Structure

```
crates/executor/src/exchange/
├── mod.rs          # Module exports
├── operator.rs     # ExchangeOperator trait, ExchangeType, Partitioning
├── local.rs        # LocalExchange implementation
└── broadcast.rs    # BroadcastExchange implementation
```

## Files

| File | Purpose |
|------|---------|
| `crates/executor/src/exchange/mod.rs` | Module exports |
| `crates/executor/src/exchange/operator.rs` | ExchangeOperator trait, enums |
| `crates/executor/src/exchange/local.rs` | LocalExchange implementation |
| `crates/executor/src/exchange/broadcast.rs` | BroadcastExchange implementation |

## Testing

All implementations must:
- Compile without errors
- Pass existing pipeline tests
- Follow the Operator trait contract (open/next_chunk/close)

## Dependencies

- Uses existing `pipeline::Operator` trait
- Uses existing `ChunkOrDone` enum
- Uses existing `PipelineResult` error type
- Uses existing `Schema` struct
