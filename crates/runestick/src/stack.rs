use crate::value::Value;
use thiserror::Error;

/// An error raised when interacting with the stack.
#[derive(Debug, Error)]
pub enum StackError {
    /// Trying to pop an empty stack.
    #[error("stack is empty")]
    StackEmpty,
    /// Attempt to access out-of-bounds stack item.
    #[error("tried to access an out-of-bounds stack entry")]
    StackOutOfBounds,
    /// Attempt to pop outside of current frame offset.
    #[error("attempted to pop beyond current stack frame `{frame}`")]
    PopOutOfBounds {
        /// CallFrame offset that we tried to pop.
        frame: usize,
    },
    /// We encountered a corrupted stack frame.
    #[error("stack size `{stack_top}` starts before the current stack frame `{frame_at}`")]
    CorruptedStackFrame {
        /// The size of the stack.
        stack_top: usize,
        /// The location of the stack frame.
        frame_at: usize,
    },
}

/// The stack of the virtual machine, where all values are stored.
#[derive(Debug)]
pub struct Stack {
    /// The current stack of values.
    stack: Vec<Value>,
    /// The top of the current stack frame.
    ///
    /// It is not possible to interact with values below this stack frame.
    stack_top: usize,
}

impl Stack {
    /// Construct a new stack.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            stack_top: 0,
        }
    }

    /// Clear the current stack.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.stack_top = 0;
    }

    /// Peek the top of the stack.
    pub fn peek(&mut self) -> Result<&Value, StackError> {
        self.stack.last().ok_or_else(|| StackError::StackEmpty)
    }

    /// Get the last position on the stack.
    pub fn last(&self) -> Result<&Value, StackError> {
        self.stack.last().ok_or_else(|| StackError::StackEmpty)
    }

    /// Access the value at the given frame offset.
    pub fn at_offset(&self, offset: usize) -> Result<&Value, StackError> {
        self.stack_top
            .checked_add(offset)
            .and_then(|n| self.stack.get(n))
            .ok_or_else(|| StackError::StackOutOfBounds)
    }

    /// Get the offset at the given location.
    pub fn at_offset_mut(&mut self, offset: usize) -> Result<&mut Value, StackError> {
        let n = match self.stack_top.checked_add(offset) {
            Some(n) => n,
            None => return Err(StackError::StackOutOfBounds),
        };

        match self.stack.get_mut(n) {
            Some(value) => Ok(value),
            None => Err(StackError::StackOutOfBounds),
        }
    }

    /// Push an unmanaged reference.
    ///
    /// The reference count of the value being referenced won't be modified.
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pop a reference to a value from the stack.
    pub fn pop(&mut self) -> Result<Value, StackError> {
        if self.stack.len() == self.stack_top {
            return Err(StackError::PopOutOfBounds {
                frame: self.stack_top,
            });
        }

        self.stack.pop().ok_or_else(|| StackError::StackEmpty)
    }

    /// Pop the given number of elements from the stack.
    pub fn popn(&mut self, n: usize) -> Result<(), StackError> {
        if self.stack.len().saturating_sub(self.stack_top) < n {
            return Err(StackError::PopOutOfBounds {
                frame: self.stack_top,
            });
        }

        for _ in 0..n {
            // NB: bounds have already been checked above.
            let value = self.stack.pop();
            debug_assert!(value.is_some());
        }

        Ok(())
    }

    /// Test if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get the length of the stack.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Iterate over the stack.
    pub fn iter(&self) -> impl Iterator<Item = &Value> + '_ {
        self.stack.iter()
    }

    /// Modify stack top by subtracting the given count from it while checking
    /// that it is in bounds of the stack.
    ///
    /// This is used internally when returning from a call frame.
    pub(crate) fn push_stack_top(&mut self, count: usize) -> Result<usize, StackError> {
        let new_stack_top = self
            .stack
            .len()
            .checked_sub(count)
            .ok_or_else(|| StackError::StackOutOfBounds)?;

        self.stack_top = new_stack_top;
        Ok(self.stack_top)
    }

    // Assert that the stack frame has been restored to the previous top
    // at the point of return.
    pub(crate) fn check_stack_top(&self) -> Result<(), StackError> {
        if self.stack.len() != self.stack_top {
            return Err(StackError::CorruptedStackFrame {
                stack_top: self.stack.len(),
                frame_at: self.stack_top,
            });
        }

        Ok(())
    }

    /// Pop the current stack top and modify it to a different one.
    ///
    /// This asserts that the size of the current stack frame is exactly zero
    /// before restoring it.
    pub(crate) fn pop_stack_top(&mut self, new_stack_top: usize) -> Result<(), StackError> {
        self.check_stack_top()?;
        self.stack_top = new_stack_top;
        Ok(())
    }
}
