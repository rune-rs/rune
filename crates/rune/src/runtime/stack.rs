use crate::runtime::{InstAddress, Value};
use std::borrow::Cow;
use std::iter;
use std::mem;
use std::slice;
use thiserror::Error;

/// An error raised when interacting with the stack.
#[derive(Debug, Error)]
#[error("tried to access out-of-bounds stack entry")]
pub struct StackError(());

/// The stack of the virtual machine, where all values are stored.
#[derive(Default, Debug, Clone)]
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
    ///
    /// ```
    /// use rune::runtime::Stack;
    /// use rune::Value;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.pop().is_err());
    /// stack.push(String::from("Hello World"));
    /// assert!(matches!(stack.pop()?, Value::String(..)));
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub const fn new() -> Self {
        Self {
            stack: Vec::new(),
            stack_bottom: 0,
        }
    }

    /// Construct a new stack with the given capacity pre-allocated.
    ///
    /// ```
    /// use rune::runtime::Stack;
    /// use rune::Value;
    ///
    /// let mut stack = Stack::with_capacity(16);
    /// assert!(stack.pop().is_err());
    /// stack.push(String::from("Hello World"));
    /// assert!(matches!(stack.pop()?, Value::String(..)));
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: Vec::with_capacity(capacity),
            stack_bottom: 0,
        }
    }

    /// Check if the stack is empty.
    ///
    /// This ignores [stack_bottom] and will just check if the full stack is
    /// empty.
    ///
    /// ```
    /// use rune::runtime::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.is_empty());
    /// stack.push(String::from("Hello World"));
    /// assert!(!stack.is_empty());
    /// ```
    ///
    /// [stack_bottom]: Self::stack_bottom()
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get the length of the stack.
    ///
    /// This ignores [stack_bottom] and will just return the total length of
    /// the stack.
    ///
    /// ```
    /// use rune::runtime::Stack;
    ///
    /// let mut stack = Stack::new();
    /// assert_eq!(stack.len(), 0);
    /// stack.push(String::from("Hello World"));
    /// assert_eq!(stack.len(), 1);
    /// ```
    ///
    /// [stack_bottom]: Self::stack_bottom()
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Perform a raw access over the stack.
    ///
    /// This ignores [stack_bottom] and will just check that the given slice
    /// index is within range.
    ///
    /// [stack_bottom]: Self::stack_bottom()
    pub fn get<I>(&self, index: I) -> Option<&<I as slice::SliceIndex<[Value]>>::Output>
    where
        I: slice::SliceIndex<[Value]>,
    {
        self.stack.get(index)
    }

    /// Push a value onto the stack.
    ///
    /// ```
    /// use rune::runtime::Stack;
    /// use rune::Value;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.pop().is_err());
    /// stack.push(String::from("Hello World"));
    /// assert!(matches!(stack.pop()?, Value::String(..)));
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn push<T>(&mut self, value: T)
    where
        Value: From<T>,
    {
        self.stack.push(Value::from(value));
    }

    /// Pop a value from the stack.
    ///
    /// ```
    /// use rune::runtime::Stack;
    /// use rune::Value;
    ///
    /// let mut stack = Stack::new();
    /// assert!(stack.pop().is_err());
    /// stack.push(String::from("Hello World"));
    /// assert!(matches!(stack.pop()?, Value::String(..)));
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn pop(&mut self) -> Result<Value, StackError> {
        if self.stack.len() == self.stack_bottom {
            return Err(StackError(()));
        }

        self.stack.pop().ok_or(StackError(()))
    }

    /// Drain the top `count` elements of the stack in the order that they were
    /// pushed, from bottom to top.
    ///
    /// ```
    /// use rune::runtime::Stack;
    /// use rune::Value;
    ///
    /// let mut stack = Stack::new();
    ///
    /// stack.push(42i64);
    /// stack.push(String::from("foo"));
    /// stack.push(());
    ///
    /// let mut it = stack.drain(2)?;
    ///
    /// assert!(matches!(it.next(), Some(Value::String(..))));
    /// assert!(matches!(it.next(), Some(Value::Unit)));
    /// assert!(matches!(it.next(), None));
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn drain(
        &mut self,
        count: usize,
    ) -> Result<impl DoubleEndedIterator<Item = Value> + '_, StackError> {
        match self.stack.len().checked_sub(count) {
            Some(start) if start >= self.stack_bottom => Ok(self.stack.drain(start..)),
            _ => Err(StackError(())),
        }
    }

    /// Extend the current stack with an iterator.
    ///
    /// ```
    /// use rune::runtime::Stack;
    /// use rune::Value;
    ///
    /// let mut stack = Stack::new();
    ///
    /// stack.extend([Value::from(42i64), Value::from(String::from("foo")), Value::Unit]);
    ///
    /// let mut it = stack.drain(2)?;
    ///
    /// assert!(matches!(it.next(), Some(Value::String(..))));
    /// assert!(matches!(it.next(), Some(Value::Unit)));
    /// assert!(matches!(it.next(), None));
    /// # Ok::<_, rune::Error>(())
    /// ```
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Value>,
    {
        self.stack.extend(iter);
    }

    /// Clear the current stack.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.stack_bottom = 0;
    }

    /// Get the last position on the stack.
    #[inline]
    pub fn last(&self) -> Result<&Value, StackError> {
        self.stack.last().ok_or(StackError(()))
    }

    /// Get the last position on the stack.
    #[inline]
    pub(crate) fn peek(&self) -> Option<&Value> {
        self.stack.last()
    }

    /// Iterate over the stack.
    pub fn iter(&self) -> impl Iterator<Item = &Value> + '_ {
        self.stack.iter()
    }

    /// Get the offset that corresponds to the bottom of the stack right now.
    ///
    /// The stack is partitioned into call frames, and once we enter a call
    /// frame the bottom of the stack corresponds to the bottom of the current
    /// call frame.
    pub fn stack_bottom(&self) -> usize {
        self.stack_bottom
    }

    /// Access the value at the given frame offset.
    pub(crate) fn at_offset(&self, offset: usize) -> Result<&Value, StackError> {
        self.stack_bottom
            .checked_add(offset)
            .and_then(|n| self.stack.get(n))
            .ok_or(StackError(()))
    }

    /// Peek the value at the given offset from the top.
    pub(crate) fn at_offset_from_top(&self, offset: usize) -> Result<&Value, StackError> {
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
    pub(crate) fn at_offset_mut(&mut self, offset: usize) -> Result<&mut Value, StackError> {
        let n = match self.stack_bottom.checked_add(offset) {
            Some(n) => n,
            None => return Err(StackError(())),
        };

        match self.stack.get_mut(n) {
            Some(value) => Ok(value),
            None => Err(StackError(())),
        }
    }

    /// Address a value on the stack.
    pub(crate) fn address(&mut self, address: InstAddress) -> Result<Value, StackError> {
        Ok(match address {
            InstAddress::Top => self.pop()?,
            InstAddress::Offset(offset) => self.at_offset(offset)?.clone(),
        })
    }

    /// Address a value on the stack.
    pub(crate) fn address_ref(
        &mut self,
        address: InstAddress,
    ) -> Result<Cow<'_, Value>, StackError> {
        Ok(match address {
            InstAddress::Top => Cow::Owned(self.pop()?),
            InstAddress::Offset(offset) => Cow::Borrowed(self.at_offset(offset)?),
        })
    }

    /// Pop the given number of elements from the stack.
    pub(crate) fn popn(&mut self, count: usize) -> Result<(), StackError> {
        drop(self.drain(count)?);
        Ok(())
    }

    /// Pop a sequence of values from the stack.
    pub(crate) fn pop_sequence(&mut self, count: usize) -> Result<Vec<Value>, StackError> {
        Ok(self.drain(count)?.collect::<Vec<_>>())
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
        tracing::trace!(
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
