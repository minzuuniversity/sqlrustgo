//! Virtual Machine for Stored Procedure Execution
//!
//! This module provides a stack-based VM interpreter that executes
//! stored procedure opcodes.

use crate::stored_proc::opcodes::{CompareOp, Opcode};
use sqlrustgo_types::Value;
use std::collections::HashMap;

/// Call frame for subroutine calls
#[derive(Debug, Clone)]
struct Frame {
    /// Return address (instruction index)
    return_addr: usize,
}

impl Frame {
    fn new(return_addr: usize) -> Self {
        Self { return_addr }
    }
}

/// VM state and interpreter
///
/// The VM executes a list of opcodes using a stack-based architecture.
/// It maintains:
/// - `pc`: Program counter (instruction index)
/// - `stack`: Value stack for operands and results
/// - `variables`: Variable storage
/// - `call_stack`: Stack for subroutine calls
/// - `running`: Whether the VM is currently executing
#[derive(Debug)]
pub struct VM {
    /// Instructions to execute
    instructions: Vec<Opcode>,
    /// Program counter (index into instructions)
    pc: usize,
    /// Value stack
    stack: Vec<Value>,
    /// Local variables
    variables: HashMap<String, Value>,
    /// Call stack for subroutine calls
    call_stack: Vec<Frame>,
    /// Whether the VM is running
    running: bool,
}

impl VM {
    /// Create a new VM with the given instructions
    pub fn new(instructions: Vec<Opcode>) -> Self {
        Self {
            instructions,
            pc: 0,
            stack: Vec::new(),
            variables: HashMap::new(),
            call_stack: Vec::new(),
            running: false,
        }
    }

    /// Run the VM until Halt
    ///
    /// Returns:
    /// - `Ok(Some(Value))` if the program returns a value
    /// - `Ok(None)` if the program exits without a return value
    /// - `Err(String)` on execution error
    pub fn run(&mut self) -> Result<Option<Value>, String> {
        self.running = true;
        self.pc = 0;
        self.stack.clear();
        self.variables.clear();
        self.call_stack.clear();

        while self.running {
            if self.pc >= self.instructions.len() {
                return Err(format!(
                    "Program counter out of bounds: {} (instruction count: {})",
                    self.pc,
                    self.instructions.len()
                ));
            }

            let opcode = self.instructions[self.pc].clone();
            let should_continue = self.execute(&opcode)?;
            if !should_continue {
                break;
            }
        }

        Ok(self.stack.pop())
    }

    /// Execute a single opcode
    ///
    /// Returns:
    /// - `Ok(true)` if execution should continue
    /// - `Ok(false)` if the program should halt
    /// - `Err(String)` on error
    fn execute(&mut self, opcode: &Opcode) -> Result<bool, String> {
        match opcode {
            Opcode::Halt => {
                self.running = false;
                Ok(false)
            }

            Opcode::LoadConst(value) => {
                self.stack.push(value.clone());
                self.pc += 1;
                Ok(true)
            }

            Opcode::Add => {
                self.binary_op(|a, b| match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
                    _ => Err(format!("Add: unsupported types: {:?} + {:?}", a, b)),
                })
            }

            Opcode::Sub => {
                self.binary_op(|a, b| match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
                    _ => Err(format!("Sub: unsupported types: {:?} - {:?}", a, b)),
                })
            }

            Opcode::Mul => {
                self.binary_op(|a, b| match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
                    _ => Err(format!("Mul: unsupported types: {:?} * {:?}", a, b)),
                })
            }

            Opcode::Div => {
                self.binary_op(|a, b| match (a, b) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Integer(a / b))
                        }
                    }
                    (Value::Float(a), Value::Float(b)) => {
                        if *b == 0.0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Float(a / b))
                        }
                    }
                    (Value::Integer(a), Value::Float(b)) => {
                        if *b == 0.0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Float(*a as f64 / b))
                        }
                    }
                    (Value::Float(a), Value::Integer(b)) => {
                        if *b == 0 {
                            Err("Division by zero".to_string())
                        } else {
                            Ok(Value::Float(a / *b as f64))
                        }
                    }
                    _ => Err(format!("Div: unsupported types: {:?} / {:?}", a, b)),
                })
            }

            Opcode::Compare(op) => {
                let b = self.stack.pop().ok_or("Compare: stack underflow")?;
                let a = self.stack.pop().ok_or("Compare: stack underflow")?;
                let result = self.compare(&a, &b, op)?;
                self.stack.push(Value::Boolean(result));
                self.pc += 1;
                Ok(true)
            }

            Opcode::Jump(target) => {
                if *target >= self.instructions.len() {
                    return Err(format!("Jump: target {} out of bounds", target));
                }
                self.pc = *target;
                Ok(true)
            }

            Opcode::JumpIf(target) => {
                let cond = self.stack.pop().ok_or("JumpIf: stack underflow")?;
                let is_true = self.is_truthy(&cond)?;
                if is_true {
                    if *target >= self.instructions.len() {
                        return Err(format!("JumpIf: target {} out of bounds", target));
                    }
                    self.pc = *target;
                } else {
                    self.pc += 1;
                }
                Ok(true)
            }

            Opcode::Call(target) => {
                if *target >= self.instructions.len() {
                    return Err(format!("Call: target {} out of bounds", target));
                }
                // Push return address
                self.call_stack.push(Frame::new(self.pc + 1));
                self.pc = *target;
                Ok(true)
            }

            Opcode::Return => {
                if let Some(frame) = self.call_stack.pop() {
                    self.pc = frame.return_addr;
                    Ok(true)
                } else {
                    Err("Return: call stack underflow".to_string())
                }
            }

            Opcode::Store(name) => {
                let value = self.stack.pop().ok_or("Store: stack underflow")?;
                self.variables.insert(name.clone(), value);
                self.pc += 1;
                Ok(true)
            }

            Opcode::Load(name) => {
                match self.variables.get(name) {
                    Some(value) => {
                        self.stack.push(value.clone());
                        self.pc += 1;
                        Ok(true)
                    }
                    None => Err(format!("Load: variable '{}' not found", name)),
                }
            }

            Opcode::Dup => {
                if let Some(top) = self.stack.last().cloned() {
                    self.stack.push(top);
                    self.pc += 1;
                    Ok(true)
                } else {
                    Err("Dup: stack underflow".to_string())
                }
            }

            Opcode::Pop => {
                self.stack.pop().ok_or("Pop: stack underflow")?;
                self.pc += 1;
                Ok(true)
            }
        }
    }

    /// Execute a binary operation
    ///
    /// The operation function receives (a, b) where a is the second-to-top
    /// and b is the top of the stack.
    fn binary_op<F>(&mut self, op: F) -> Result<bool, String>
    where
        F: FnOnce(&Value, &Value) -> Result<Value, String>,
    {
        let b = self.stack.pop().ok_or("binary_op: stack underflow (b)")?;
        let a = self.stack.pop().ok_or("binary_op: stack underflow (a)")?;
        let result = op(&a, &b)?;
        self.stack.push(result);
        self.pc += 1;
        Ok(true)
    }

    /// Compare two values using the given comparison operator
    fn compare(&self, a: &Value, b: &Value, op: &CompareOp) -> Result<bool, String> {
        match op {
            CompareOp::Equal => Ok(a == b),
            CompareOp::NotEqual => Ok(a != b),
            CompareOp::Less => self.less_than(a, b),
            CompareOp::LessOrEqual => self.less_or_equal(a, b),
            CompareOp::Greater => self.greater_than(a, b),
            CompareOp::GreaterOrEqual => self.greater_or_equal(a, b),
        }
    }

    fn less_than(&self, a: &Value, b: &Value) -> Result<bool, String> {
        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(a < b),
            (Value::Float(a), Value::Float(b)) => Ok(a < b),
            (Value::Integer(a), Value::Float(b)) => Ok((*a as f64) < *b),
            (Value::Float(a), Value::Integer(b)) => Ok(*a < (*b as f64)),
            _ => Err(format!("Less: unsupported types: {:?} < {:?}", a, b)),
        }
    }

    fn less_or_equal(&self, a: &Value, b: &Value) -> Result<bool, String> {
        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(a <= b),
            (Value::Float(a), Value::Float(b)) => Ok(a <= b),
            (Value::Integer(a), Value::Float(b)) => Ok(*a as f64 <= *b),
            (Value::Float(a), Value::Integer(b)) => Ok(*a <= *b as f64),
            _ => Err(format!(
                "LessOrEqual: unsupported types: {:?} <= {:?}",
                a, b
            )),
        }
    }

    fn greater_than(&self, a: &Value, b: &Value) -> Result<bool, String> {
        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(a > b),
            (Value::Float(a), Value::Float(b)) => Ok(a > b),
            (Value::Integer(a), Value::Float(b)) => Ok(*a as f64 > *b),
            (Value::Float(a), Value::Integer(b)) => Ok(*a > *b as f64),
            _ => Err(format!("Greater: unsupported types: {:?} > {:?}", a, b)),
        }
    }

    fn greater_or_equal(&self, a: &Value, b: &Value) -> Result<bool, String> {
        match (a, b) {
            (Value::Integer(a), Value::Integer(b)) => Ok(a >= b),
            (Value::Float(a), Value::Float(b)) => Ok(a >= b),
            (Value::Integer(a), Value::Float(b)) => Ok(*a as f64 >= *b),
            (Value::Float(a), Value::Integer(b)) => Ok(*a >= *b as f64),
            _ => Err(format!(
                "GreaterOrEqual: unsupported types: {:?} >= {:?}",
                a, b
            )),
        }
    }

    /// Check if a value is truthy
    fn is_truthy(&self, value: &Value) -> Result<bool, String> {
        match value {
            Value::Boolean(b) => Ok(*b),
            Value::Null => Ok(false),
            Value::Integer(i) => Ok(*i != 0),
            Value::Float(f) => Ok(*f != 0.0),
            Value::Text(s) => Ok(!s.is_empty()),
            Value::Blob(_) => Err("is_truthy: Blob is not supported".to_string()),
            Value::Geometry(_) => Err("is_truthy: Geometry is not supported".to_string()),
        }
    }

    /// Get a reference to the current stack (for testing)
    #[cfg(test)]
    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    /// Get a reference to variables (for testing)
    #[cfg(test)]
    pub fn variables(&self) -> &HashMap<String, Value> {
        &self.variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stored_proc::opcodes::Opcode;

    #[test]
    fn test_vm_halt() {
        let mut vm = VM::new(vec![Opcode::Halt]);
        let result = vm.run();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_vm_load_const() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(42)),
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(42)));
    }

    #[test]
    fn test_vm_add_integers() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(10)),
            Opcode::LoadConst(Value::Integer(20)),
            Opcode::Add,
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(30)));
    }

    #[test]
    fn test_vm_add_floats() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Float(1.5)),
            Opcode::LoadConst(Value::Float(2.5)),
            Opcode::Add,
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Float(4.0)));
    }

    #[test]
    fn test_vm_sub() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(10)),
            Opcode::LoadConst(Value::Integer(3)),
            Opcode::Sub,
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(7)));
    }

    #[test]
    fn test_vm_mul() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(6)),
            Opcode::LoadConst(Value::Integer(7)),
            Opcode::Mul,
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(42)));
    }

    #[test]
    fn test_vm_div() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(20)),
            Opcode::LoadConst(Value::Integer(4)),
            Opcode::Div,
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(5)));
    }

    #[test]
    fn test_vm_div_by_zero() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(1)),
            Opcode::LoadConst(Value::Integer(0)),
            Opcode::Div,
            Opcode::Halt,
        ]);
        let result = vm.run();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_vm_compare_equal() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(10)),
            Opcode::LoadConst(Value::Integer(10)),
            Opcode::Compare(CompareOp::Equal),
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Boolean(true)));
    }

    #[test]
    fn test_vm_compare_less() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(5)),
            Opcode::LoadConst(Value::Integer(10)),
            Opcode::Compare(CompareOp::Less),
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Boolean(true)));
    }

    #[test]
    fn test_vm_jump() {
        // Program: load 100, jump to halt (skip Add)
        // The 100 stays on the stack and is returned
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(100)), // This stays on stack
            Opcode::Jump(3),                        // Jump to Halt
            Opcode::Add,                            // This should never execute
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(100))); // 100 is still on stack
    }

    #[test]
    fn test_vm_jump_if_true() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Boolean(true)),
            Opcode::JumpIf(3), // Jump to Halt
            Opcode::LoadConst(Value::Integer(999)),
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, None); // Jumped over the 999
    }

    #[test]
    fn test_vm_jump_if_false() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Boolean(false)),
            Opcode::JumpIf(4), // Don't jump
            Opcode::LoadConst(Value::Integer(42)),
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(42)));
    }

    #[test]
    fn test_vm_call_return() {
        // Program: Call a subroutine that adds 1, return result
        // Subroutine at index 3: LoadConst 1, Return
        // Main: LoadConst 10, Call 3, Halt
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(10)),
            Opcode::Call(3),      // Call subroutine at index 3
            Opcode::Halt,
            Opcode::LoadConst(Value::Integer(1)), // Subroutine
            Opcode::Return,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(1))); // Returns 1, not 10+1
    }

    #[test]
    fn test_vm_store_load() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(42)),
            Opcode::Store("x".to_string()),
            Opcode::Load("x".to_string()), // Should load 42 back
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(42)));
    }

    #[test]
    fn test_vm_dup() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(5)),
            Opcode::Dup, // Duplicate the 5
            Opcode::Add, // Add them: 5 + 5 = 10
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(10)));
    }

    #[test]
    fn test_vm_pop() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(1)),
            Opcode::LoadConst(Value::Integer(2)),
            Opcode::Pop, // Discard the 2
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(1)));
    }

    #[test]
    fn test_vm_complex_expression() {
        // Compute: (3 + 5) * 2 = 16
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(3)),
            Opcode::LoadConst(Value::Integer(5)),
            Opcode::Add,
            Opcode::LoadConst(Value::Integer(2)),
            Opcode::Mul,
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(16)));
    }

    #[test]
    fn test_vm_with_variables() {
        // Compute: x = 10; y = 20; x + y
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(10)),
            Opcode::Store("x".to_string()),
            Opcode::LoadConst(Value::Integer(20)),
            Opcode::Store("y".to_string()),
            Opcode::Load("x".to_string()),
            Opcode::Load("y".to_string()),
            Opcode::Add,
            Opcode::Halt,
        ]);
        let result = vm.run().unwrap();
        assert_eq!(result, Some(Value::Integer(30)));
    }

    #[test]
    fn test_vm_out_of_bounds_jump() {
        let mut vm = VM::new(vec![
            Opcode::Jump(100), // Out of bounds
            Opcode::Halt,
        ]);
        let result = vm.run();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_vm_stack_underflow() {
        let mut vm = VM::new(vec![
            Opcode::LoadConst(Value::Integer(1)),
            Opcode::Add, // Add without second operand
            Opcode::Halt,
        ]);
        let result = vm.run();
        assert!(result.is_err());
    }

    #[test]
    fn test_vm_undefined_variable() {
        let mut vm = VM::new(vec![
            Opcode::Load("undefined".to_string()),
            Opcode::Halt,
        ]);
        let result = vm.run();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }
}
