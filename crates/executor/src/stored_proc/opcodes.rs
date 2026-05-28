//! VM Opcodes for Stored Procedure Execution
//!
//! This module defines the instruction set for the stored procedure VM.
//! The VM is stack-based and supports arithmetic, control flow, and memory operations.

use sqlrustgo_types::Value;

/// Comparison operators for Compare opcode
#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

/// VM Instructions (Opcodes)
///
/// Each opcode represents a single instruction in the VM's instruction set.
/// The VM is stack-based: most operations pop operands from the stack and push results.
#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    /// Stop execution and return from the program
    Halt,

    /// Push a constant value onto the stack
    /// Operand: The constant value to push
    LoadConst(Value),

    /// Add two values: pop b, pop a, push a + b
    /// Supported types: Integer, Float
    Add,

    /// Subtract two values: pop b, pop a, push a - b
    Sub,

    /// Multiply two values: pop b, pop a, push a * b
    Mul,

    /// Divide two values: pop b, pop a, push a / b
    /// Returns error on division by zero
    Div,

    /// Compare two values and push a boolean result
    /// Operand: The comparison operator
    Compare(CompareOp),

    /// Unconditional jump to a target instruction address
    /// Operand: Target instruction index
    Jump(usize),

    /// Conditional jump: pop condition, jump if true
    /// Operand: Target instruction index
    JumpIf(usize),

    /// Call a subroutine at the target address
    /// Pushes return address onto call stack
    /// Operand: Target instruction index
    Call(usize),

    /// Return from a subroutine
    /// Pops return address from call stack and jumps there
    Return,

    /// Store a value into a variable
    /// Operand: Variable name
    /// Pop: Value from stack
    Store(String),

    /// Load a variable value onto the stack
    /// Operand: Variable name
    Load(String),

    /// Duplicate the top value on the stack
    Dup,

    /// Pop and discard the top value
    Pop,
}

impl Opcode {
    /// Returns true if this opcode consumes stack operands
    pub fn consumes_stack(&self) -> bool {
        match self {
            Opcode::Halt => false,
            Opcode::LoadConst(_) => false,
            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div => true,
            Opcode::Compare(_) => true,
            Opcode::Jump(_) => false,
            Opcode::JumpIf(_) => true,
            Opcode::Call(_) => false,
            Opcode::Return => false,
            Opcode::Store(_) => true,
            Opcode::Load(_) => false,
            Opcode::Dup => false,
            Opcode::Pop => true,
        }
    }

    /// Returns true if this opcode produces a stack value
    pub fn produces_stack(&self) -> bool {
        match self {
            Opcode::Halt => false,
            Opcode::LoadConst(_) => true,
            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div => true,
            Opcode::Compare(_) => true,
            Opcode::Jump(_) | Opcode::JumpIf(_) => false,
            Opcode::Call(_) => false,
            Opcode::Return => false,
            Opcode::Store(_) => false,
            Opcode::Load(_) => true,
            Opcode::Dup => true,
            Opcode::Pop => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_consumer_producer() {
        // LoadConst produces but doesn't consume
        let op = Opcode::LoadConst(Value::Integer(42));
        assert!(!op.consumes_stack());
        assert!(op.produces_stack());

        // Add consumes two and produces one
        assert!(Opcode::Add.consumes_stack());
        assert!(Opcode::Add.produces_stack());

        // Jump doesn't touch the stack
        assert!(!Opcode::Jump(0).consumes_stack());
        assert!(!Opcode::Jump(0).produces_stack());

        // Store consumes one
        assert!(Opcode::Store("x".to_string()).consumes_stack());
        assert!(!Opcode::Store("x".to_string()).produces_stack());

        // Pop consumes one
        assert!(Opcode::Pop.consumes_stack());
        assert!(!Opcode::Pop.produces_stack());
    }

    #[test]
    fn test_compare_op_equality() {
        let eq1 = CompareOp::Equal;
        let eq2 = CompareOp::Equal;
        let ne = CompareOp::NotEqual;

        assert_eq!(eq1, eq2);
        assert_ne!(eq1, ne);
    }

    #[test]
    fn test_opcode_clone() {
        let op = Opcode::LoadConst(Value::Text("hello".to_string()));
        let cloned = op.clone();
        assert_eq!(op, cloned);
    }
}
