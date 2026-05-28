# SPEC: Stored Procedure VM Refactoring

## Issue
Issue #1463: Architecture: stored_proc VM Refactoring

## Context
The existing `stored_proc` module provides stored procedure execution with control flow support. This refactoring introduces a VM (Virtual Machine) architecture to execute stored procedures as a sequence of opcodes, providing a cleaner separation between compilation (AST → opcodes) and execution (VM interpreter).

## Goals
1. Define a simple VM instruction set (opcodes) for stored procedure execution
2. Implement a basic VM interpreter that executes opcodes
3. Maintain backward compatibility with existing stored_proc functionality
4. Establish foundation for future optimizations (JIT, etc.)

## Architecture

### Module Structure
```
stored_proc/
├── mod.rs           # Main module, exports VM components
├── opcodes.rs       # VM instruction definitions
├── vm.rs           # VM interpreter
├── context.rs      # (existing) Procedure context
├── execution.rs    # (existing) Statement execution
├── expression.rs   # (existing) Expression evaluation
├── cursor.rs      # (existing) Cursor management
├── handler.rs      # (existing) Exception handling
└── cte.rs         # (existing) CTE support
```

### Opcode Design

The VM uses a stack-based architecture with the following instructions:

| Opcode | Operands | Description |
|--------|----------|-------------|
| `Halt` | - | Stop execution |
| `LoadConst` | value: Value | Push constant onto stack |
| `Add` | - | Pop b, pop a, push a + b |
| `Sub` | - | Pop b, pop a, push a - b |
| `Mul` | - | Pop b, pop a, push a * b |
| `Div` | - | Pop b, pop a, push a / b |
| `Compare` | op: CompareOp | Pop b, pop a, compare, push result |
| `Jump` | target: usize | Unconditional jump |
| `JumpIf` | target: usize | Pop condition, jump if true |
| `Call` | target: usize | Call subroutine at address |
| `Return` | - | Return from subroutine |
| `Store` | name: String | Pop value, store in variable |
| `Load` | name: String | Push variable value onto stack |

### VM Design

```rust
pub struct VM {
    instructions: Vec<Opcode>,
    pc: usize,           // Program counter
    stack: Vec<Value>,   // Value stack
}

impl VM {
    pub fn new(instructions: Vec<Opcode>) -> Self;
    pub fn run(&mut self) -> Result<Option<Value>, String>;
    fn step(&mut self) -> Result<bool, String>;  // returns false if Halt
}
```

## Implementation Plan

### Phase 1: Core VM (This Issue)
- [x] Define `Opcode` enum with all instructions
- [x] Define `Value` type for stack values
- [x] Implement `VM` struct with `run()` method
- [x] Implement `step()` for single instruction execution
- [x] Basic arithmetic operations
- [x] Control flow (Jump, JumpIf, Call, Return)
- [x] Memory operations (Load, Store)

### Phase 2: Future Extensions (Not in this issue)
- [ ] Expression compilation to opcodes
- [ ] Statement compilation to opcodes
- [ ] Integration with existing stored_proc execution
- [ ] Built-in function calls
- [ ] Cursor operations in VM

## Testing Strategy
- Unit tests for each opcode in isolation
- Integration tests for complete programs
- Ensure existing stored_proc tests continue to pass

## Backward Compatibility
The existing `stored_proc` module is preserved. The VM is introduced as a new component that can eventually replace or augment the existing execution engine.

## Risks and Mitigations
- **Risk**: Breaking existing functionality
  - **Mitigation**: All changes are additive; existing tests must continue to pass
- **Risk**: Performance regression
  - **Mitigation**: VM provides foundation for future optimizations; initial implementation prioritizes correctness over performance