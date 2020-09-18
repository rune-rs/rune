use crate::Value;
use std::iter;
use std::mem;
use std::slice;
use thiserror::Error;

/// An error raised when interacting with the stack.
#[derive(Debug, Error)]
#[error("tried to access out-of-bounds stack entry")]
pub struct StackError(());

/// The stack of the virtual machine, where all values are stored.
#[derive(Debug, Clone)]
pub struct Stack {
    /// The current stack of values.
    stack: Vec<Value>,
    /// The top of the current stack frame.
    ///
    /// It is not possible to interact with values below this stack frame.
    stack_bottom: usize,
}

impl Stack {
    /// Construct a new stack.
    pub const fn new() -> Self {
        Self {
            stack: Vec::new(),
            stack_bottom: 0,
        }
    }

    /// Extend the current stack.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Value>,
    {
        self.stack.extend(iter);
    }

    /// Get the offset that corresponds to the top of the stack right now.
    pub fn stack_bottom(&self) -> usize {
        self.stack_bottom
    }

    /// Construct a new stack with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: Vec::with_capacity(capacity),
            stack_bottom: 0,
        }
    }

    /// Clear the current stack.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.stack_bottom = 0;
    }

    /// Get the given slice of the stack, if it isn't out of range.
    pub fn get<I>(&self, index: I) -> Option<&<I as slice::SliceIndex<[Value]>>::Output>
    where
        I: slice::SliceIndex<[Value]>,
    {
        self.stack.get(index)
    }

    /// Peek the top of the stack.
    #[inline]
    pub fn peek(&mut self) -> Option<&Value> {
        self.stack.last()
    }

    /// Get the last position on the stack.
    #[inline]
    pub fn last(&self) -> Result<&Value, StackError> {
        self.stack.last().ok_or_else(|| StackError(()))
    }

    /// Access the value at the given frame offset.
    pub fn at_offset(&self, offset: usize) -> Result<&Value, StackError> {
        self.stack_bottom
            .checked_add(offset)
            .and_then(|n| self.stack.get(n))
            .ok_or_else(|| StackError(()))
    }

    /// Peek the value at the given offset from the top.
    pub fn at_offset_from_top(&self, offset: usize) -> Result<&Value, StackError> {
        match self
            .stack
            .len()
            .checked_sub(offset)
            .filter(|n| *n >= self.stack_bottom)
            .and_then(|n| self.stack.get(n))
        {
            Some(value) => Ok(value),
            None => Err(StackError(())),
        }
    }

    /// Get the offset at the given location.
    pub fn at_offset_mut(&mut self, offset: usize) -> Result<&mut Value, StackError> {
        let n = match self.stack_bottom.checked_add(offset) {
            Some(n) => n,
            None => return Err(StackError(())),
        };

        match self.stack.get_mut(n) {
            Some(value) => Ok(value),
            None => Err(StackError(())),
        }
    }

    /// Push a value onto the stack.
    pub fn push<T>(&mut self, value: T)
    where
        Value: From<T>,
    {
        self.stack.push(Value::from(value));
    }

    /// Pop a reference to a value from the stack.
    pub fn pop(&mut self) -> Result<Value, StackError> {
        if self.stack.len() == self.stack_bottom {
            return Err(StackError(()));
        }

        self.stack.pop().ok_or_else(|| StackError(()))
    }

    /// Pop the given number of elements from the stack.
    pub fn popn(&mut self, count: usize) -> Result<(), StackError> {
        drop(self.drain_stack_top(count)?);
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

    /// Pop a sequence of values from the stack.
    pub fn pop_sequence(&mut self, count: usize) -> Result<Vec<Value>, StackError> {
        Ok(self.drain_stack_top(count)?.collect::<Vec<_>>())
    }

    /// Drain the top `count` elements of the stack in the order that they were
    /// pushed, from bottom to top.
    pub fn drain_stack_top(
        &mut self,
        count: usize,
    ) -> Result<impl DoubleEndedIterator<Item = Value> + '_, StackError> {
        match self.stack.len().checked_sub(count) {
            Some(start) if start >= self.stack_bottom => Ok(self.stack.drain(start..)),
            _ => Err(StackError(())),
        }
    }

    /// Modify stack top by subtracting the given count from it while checking
    /// that it is in bounds of the stack.
    ///
    /// This is used internally when returning from a call frame.
    ///
    /// Returns the old stack top.
    pub(crate) fn swap_stack_bottom(&mut self, count: usize) -> Result<usize, StackError> {
        match self.stack.len().checked_sub(count) {
            Some(new_top) => Ok(mem::replace(&mut self.stack_bottom, new_top)),
            None => Err(StackError(())),
        }
    }

    // Assert that the stack frame has been restored to the previous top
    // at the point of return.
    pub(crate) fn check_stack_top(&self) -> Result<(), StackError> {
        log::trace!(
            "check_stack_top: self.stack.len() ({}) == self.stack_bottom ({})",
            self.stack.len(),
            self.stack_bottom
        );

        if self.stack.len() == self.stack_bottom {
            return Ok(());
        }

        Err(StackError(()))
    }

    /// Pop the current stack top and modify it to a different one.
    ///
    /// This asserts that the size of the current stack frame is exactly zero
    /// before restoring it.
    pub(crate) fn pop_stack_top(&mut self, stack_bottom: usize) -> Result<(), StackError> {
        self.check_stack_top()?;
        self.stack_bottom = stack_bottom;
        Ok(())
    }
}

impl iter::FromIterator<Value> for Stack {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self {
            stack: iter.into_iter().collect(),
            stack_bottom: 0,
        }
    }
}

impl From<Vec<Value>> for Stack {
    fn from(stack: Vec<Value>) -> Self {
        Self {
            stack,
            stack_bottom: 0,
        }
    }
}
