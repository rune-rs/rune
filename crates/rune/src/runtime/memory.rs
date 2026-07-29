use core::array;
use core::convert::Infallible;
use core::fmt;
use core::mem::replace;
use core::slice;
#[cfg(feature = "cli")]
use core::slice::SliceIndex;

use crate::alloc::alloc::Global;
use crate::alloc::prelude::*;
use crate::alloc::{self, Vec};

use super::{Address, Dismantle, Handover, IntoOutput, Output, Value, VmErrorKind, Worklist};

// This is a bit tricky. We know that `Value::empty()` is `Sync` but we can't
// convince Rust that is the case.
struct AssertSync<T>(T);
unsafe impl<T> Sync for AssertSync<T> {}

static EMPTY: AssertSync<Value> = AssertSync(Value::empty());

/// An error raised when accessing an address on the stack.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[non_exhaustive]
pub struct StackError {
    addr: Address,
}

impl From<Infallible> for StackError {
    #[inline]
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl fmt::Display for StackError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tried to access out-of-bounds stack entry {}", self.addr)
    }
}

impl core::error::Error for StackError {}

/// An error raised when accessing a slice on the stack.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[non_exhaustive]
pub struct SliceError {
    addr: Address,
    len: usize,
    stack: usize,
}

impl fmt::Display for SliceError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tried to access out-of-bounds stack slice {}-{} in 0-{}",
            self.addr,
            self.addr.offset() + self.len,
            self.stack
        )
    }
}

impl core::error::Error for SliceError {}

pub(crate) enum Pair<'a> {
    Same(&'a mut Value),
    Pair(&'a mut Value, &'a Value),
}

/// An error produced by a call to `Memory::store`.
pub struct StoreError<E> {
    kind: StoreErrorKind<E>,
}

impl<E> StoreError<E> {
    #[inline]
    pub(crate) fn into_kind(self) -> StoreErrorKind<E> {
        self.kind
    }
}

pub(crate) enum StoreErrorKind<E> {
    Stack(StackError),
    Error(E),
    /// Taking the value which was replaced apart ran out of memory.
    Alloc(alloc::Error),
}

impl<E> From<StackError> for StoreError<E> {
    #[inline]
    fn from(error: StackError) -> Self {
        Self {
            kind: StoreErrorKind::Stack(error),
        }
    }
}

impl<E> From<alloc::Error> for StoreError<E> {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        Self {
            kind: StoreErrorKind::Alloc(error),
        }
    }
}

impl<E> StoreError<E> {
    #[inline]
    fn error(error: E) -> Self {
        Self {
            kind: StoreErrorKind::Error(error),
        }
    }
}

/// Memory access.
pub trait Memory {
    /// Get the slice at the given address with the given length.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Address, Memory, Output, VmError};
    ///
    /// fn sum(memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     let mut number = 0;
    ///
    ///     for value in memory.slice_at(addr, args)? {
    ///         number += value.as_integer::<i64>()?;
    ///     }
    ///
    ///     memory.store(out, number)?;
    ///     Ok(())
    /// }
    /// ```
    fn slice_at(&self, addr: Address, len: usize) -> Result<&[Value], SliceError>;

    /// Access the given slice mutably.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Address, Memory, Output, Value, VmError};
    ///
    /// fn drop_values(memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     for value in memory.slice_at_mut(addr, args)? {
    ///         *value = Value::empty();
    ///     }
    ///
    ///     memory.store(out, ())?;
    ///     Ok(())
    /// }
    /// ```
    fn slice_at_mut(&mut self, addr: Address, len: usize) -> Result<&mut [Value], SliceError>;

    /// Get a value mutable at the given index from the stack bottom.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Address, Memory, Output, VmError};
    ///
    /// fn add_one(memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     let mut value = memory.at_mut(addr)?;
    ///     let number = value.as_integer::<i64>()?;
    ///     *value = rune::to_value(number + 1)?;
    ///     memory.store(out, ())?;
    ///     Ok(())
    /// }
    /// ```
    fn at_mut(&mut self, addr: Address) -> Result<&mut Value, StackError>;

    /// Get the slice at the given address with the given static length.
    fn array_at<const N: usize>(&self, addr: Address) -> Result<[&Value; N], SliceError>
    where
        Self: Sized,
    {
        let slice = self.slice_at(addr, N)?;
        Ok(array::from_fn(|i| &slice[i]))
    }
}

impl dyn Memory + '_ {
    /// Write output using the provided [`IntoOutput`] implementation onto the
    /// stack.
    ///
    /// The [`IntoOutput`] trait primarily allows for deferring a computation
    /// since it's implemented by [`FnOnce`]. However, you must take care that
    /// any side effects calling a function may have are executed outside of the
    /// call to `store`. Like if the function would error.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Output, Memory, ToValue, VmError, Address};
    /// use rune::vm_try;
    ///
    /// fn sum(memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     let mut number = 0;
    ///
    ///     for value in memory.slice_at(addr, args)? {
    ///         number += value.as_integer::<i64>()?;
    ///     }
    ///
    ///     memory.store(out, number)?;
    ///     Ok(())
    /// }
    /// ```
    #[inline(always)]
    pub fn store<O>(&mut self, out: Output, o: O) -> Result<(), StoreError<O::Error>>
    where
        O: IntoOutput,
    {
        if let Some(addr) = out.as_addr() {
            let value = o.into_output().map_err(StoreError::error)?;

            // A native function has no worklist of its own to take the value
            // which is replaced apart over, so it gets one which is empty. It
            // costs nothing until it is handed a value made of other values.
            let mut worklist = Worklist::new();

            // The value which is replaced is taken apart rather than left to
            // its destructor, so that the walk does not descend into it.
            worklist.replace(self.at_mut(addr)?, value);
        }

        Ok(())
    }
}

impl<M> Memory for &mut M
where
    M: Memory + ?Sized,
{
    #[inline]
    fn slice_at(&self, addr: Address, len: usize) -> Result<&[Value], SliceError> {
        (**self).slice_at(addr, len)
    }

    #[inline]
    fn slice_at_mut(&mut self, addr: Address, len: usize) -> Result<&mut [Value], SliceError> {
        (**self).slice_at_mut(addr, len)
    }

    #[inline]
    fn at_mut(&mut self, addr: Address) -> Result<&mut Value, StackError> {
        (**self).at_mut(addr)
    }
}

impl<const N: usize> Memory for [Value; N] {
    fn slice_at(&self, addr: Address, len: usize) -> Result<&[Value], SliceError> {
        if len == 0 {
            return Ok(&[]);
        }

        let start = addr.offset();

        let Some(values) = start.checked_add(len).and_then(|end| self.get(start..end)) else {
            return Err(SliceError {
                addr,
                len,
                stack: N,
            });
        };

        Ok(values)
    }

    fn slice_at_mut(&mut self, addr: Address, len: usize) -> Result<&mut [Value], SliceError> {
        if len == 0 {
            return Ok(&mut []);
        }

        let start = addr.offset();

        let Some(values) = start
            .checked_add(len)
            .and_then(|end| self.get_mut(start..end))
        else {
            return Err(SliceError {
                addr,
                len,
                stack: N,
            });
        };

        Ok(values)
    }

    #[inline]
    fn at_mut(&mut self, addr: Address) -> Result<&mut Value, StackError> {
        let Some(value) = self.get_mut(addr.offset()) else {
            return Err(StackError { addr });
        };

        Ok(value)
    }
}

/// The stack of the virtual machine, where all values are stored.
#[derive(Default, Debug)]
pub struct Stack {
    /// The current stack of values.
    stack: Vec<Value>,
    /// The top of the current stack frame.
    ///
    /// It is not possible to interact with values below this stack frame.
    top: usize,
}

impl Stack {
    /// Construct a new stack.
    #[inline]
    pub(crate) const fn new() -> Self {
        Self {
            stack: Vec::new(),
            top: 0,
        }
    }

    /// Access the value at the given frame offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::vm_try;
    /// use rune::Module;
    /// use rune::runtime::{Output, Stack, VmError, Address};
    ///
    /// fn add_one(memory: &mut Stack, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     let value = memory.at(addr).as_integer::<i64>()?;
    ///     memory.store(out, value + 1);
    ///     Ok(())
    /// }
    /// ```
    #[inline(always)]
    pub fn at(&self, addr: Address) -> &Value {
        let n = self.top.wrapping_add(addr.offset());
        self.stack.get(n).unwrap_or(&EMPTY.0)
    }

    /// Get a value mutable at the given index from the stack bottom.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::vm_try;
    /// use rune::Module;
    /// use rune::runtime::{Output, Stack, VmError, Address};
    ///
    /// fn add_one(memory: &mut Stack, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     let mut value = memory.at_mut(addr)?;
    ///     let number = value.as_integer::<i64>()?;
    ///     *value = rune::to_value(number + 1)?;
    ///     memory.store(out, ())?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn at_mut(&mut self, addr: Address) -> Result<&mut Value, StackError> {
        let n = self.top.wrapping_add(addr.offset());
        self.stack.get_mut(n).ok_or(StackError { addr })
    }

    /// Get the slice at the given address with the given length.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::vm_try;
    /// use rune::Module;
    /// use rune::runtime::{Output, Stack, ToValue, VmError, Address};
    ///
    /// fn sum(memory: &mut Stack, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     let mut number = 0;
    ///
    ///     for value in memory.slice_at(addr, args)? {
    ///         number += value.as_integer::<i64>()?;
    ///     }
    ///
    ///     memory.store(out, number)?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn slice_at(&self, addr: Address, len: usize) -> Result<&[Value], SliceError> {
        let stack_len = self.stack.len();

        if let Some(slice) = inner_slice_at(&self.stack, self.top, addr, len) {
            return Ok(slice);
        }

        Err(slice_error(stack_len, self.top, addr, len))
    }

    /// Get the mutable slice at the given address with the given length.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::vm_try;
    /// use rune::Module;
    /// use rune::runtime::{Output, Memory, VmError, Address};
    ///
    /// fn sum(memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     for value in memory.slice_at_mut(addr, args)? {
    ///         let number = value.as_integer::<i64>()?;
    ///         *value = rune::to_value(number + 1)?;
    ///     }
    ///
    ///     memory.store(out, ())?;
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub fn slice_at_mut(&mut self, addr: Address, len: usize) -> Result<&mut [Value], SliceError> {
        let stack_len = self.stack.len();

        if let Some(slice) = inner_slice_at_mut(&mut self.stack, self.top, addr, len) {
            return Ok(slice);
        }

        Err(slice_error(stack_len, self.top, addr, len))
    }

    /// Write output using the provided [`IntoOutput`] implementation onto the
    /// stack.
    ///
    /// The [`IntoOutput`] trait primarily allows for deferring a computation
    /// since it's implemented by [`FnOnce`]. However, you must take care that
    /// any side effects calling a function may have are executed outside of the
    /// call to `store`. Like if the function would error.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Output, Memory, ToValue, VmError, Address};
    /// use rune::vm_try;
    ///
    /// fn sum(memory: &mut dyn Memory, addr: Address, args: usize, out: Output) -> Result<(), VmError> {
    ///     let mut number = 0;
    ///
    ///     for value in memory.slice_at(addr, args)? {
    ///         number += value.as_integer::<i64>()?;
    ///     }
    ///
    ///     memory.store(out, number)?;
    ///     Ok(())
    /// }
    /// ```
    #[inline(always)]
    pub fn store<O>(&mut self, out: Output, o: O) -> Result<(), StoreError<O::Error>>
    where
        O: IntoOutput,
    {
        // Whoever writes to a stack without a machine behind them has no
        // worklist to take the value which is replaced apart over, so they get
        // one which is empty. It costs nothing until it is handed a value made
        // of other values.
        self.store_with(out, o, &mut Worklist::new())
    }

    /// Write output using the provided [`IntoOutput`] implementation onto the
    /// stack, taking the value which was there apart over the given worklist.
    ///
    /// The worklist is handed in by the machine, which keeps one for as long as
    /// it lives, so that whatever memory taking values apart needs is grown
    /// once rather than for every value which is written over.
    #[inline(always)]
    pub(crate) fn store_with<O>(
        &mut self,
        out: Output,
        o: O,
        work: &mut Worklist,
    ) -> Result<(), StoreError<O::Error>>
    where
        O: IntoOutput,
    {
        if let Some(addr) = out.as_addr() {
            let value = o.into_output().map_err(StoreError::error)?;

            // The value which is replaced is taken apart rather than left to
            // its destructor, so that the machine does not descend into it.
            work.replace(self.at_mut(addr)?, value);
        }

        Ok(())
    }

    /// The current top address of the stack.
    #[inline]
    pub(crate) const fn addr(&self) -> Address {
        Address::new(self.stack.len().saturating_sub(self.top))
    }

    /// Try to resize the stack with space for the given size.
    #[inline]
    pub(crate) fn resize(&mut self, size: usize) -> alloc::Result<()> {
        if size == 0 {
            return Ok(());
        }

        self.stack.try_resize_with(self.top + size, Value::empty)?;
        Ok(())
    }

    /// Construct a new stack with the given capacity pre-allocated.
    #[inline]
    pub(crate) fn with_capacity(capacity: usize) -> alloc::Result<Self> {
        Ok(Self {
            stack: Vec::try_with_capacity(capacity)?,
            top: 0,
        })
    }

    /// Perform a raw access over the stack.
    ///
    /// This ignores [top] and will just check that the given slice
    /// index is within range.
    ///
    /// [top]: Self::top()
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn get<I>(&self, index: I) -> Option<&<I as SliceIndex<[Value]>>::Output>
    where
        I: SliceIndex<[Value]>,
    {
        self.stack.get(index)
    }

    /// Push a value onto the stack.
    #[inline]
    pub(crate) fn push<T>(&mut self, value: T) -> alloc::Result<()>
    where
        T: TryInto<Value, Error: Into<alloc::Error>>,
    {
        self.stack.try_push(value.try_into().map_err(Into::into)?)?;
        Ok(())
    }

    /// Truncate the stack at the given address, taking the values which are
    /// discarded apart rather than leaving them to their destructors.
    ///
    /// This is what the machine uses, so that the values share the worklist it
    /// keeps - see [`Worklist::dismantle`].
    #[inline]
    pub(crate) fn dismantle_to(&mut self, addr: Address, work: &mut Worklist) {
        let Some(len) = self.top.checked_add(addr.offset()) else {
            return;
        };

        if let Some(values) = self.stack.get_mut(len..) {
            work.dismantle_all(values.iter_mut());
        }

        self.stack.truncate(len);
    }

    /// Open a new call frame at the top of the stack holding the values which
    /// `other` was working over.
    ///
    /// This is what splicing an unstarted execution into the machine which
    /// awaits it uses - the values the awaited machine holds become the
    /// arguments of an ordinary call frame here. Returns the old stack top, for
    /// the [`CallFrame`] the caller pushes.
    ///
    /// [`CallFrame`]: crate::runtime::CallFrame
    #[inline]
    pub(crate) fn push_frame_from(&mut self, other: &mut Stack) -> alloc::Result<usize> {
        let old_len = self.stack.len();
        self.stack.try_reserve(other.stack.len())?;

        for value in other.stack.drain(..) {
            self.stack.try_push(value)?;
        }

        other.top = 0;
        Ok(replace(&mut self.top, old_len))
    }

    /// Drain the current stack down to the current stack bottom.
    #[inline]
    pub(crate) fn drain(&mut self) -> impl DoubleEndedIterator<Item = Value> + '_ {
        self.stack.drain(self.top..)
    }

    /// Clear the current stack.
    #[inline]
    pub(crate) fn clear(&mut self) {
        self.stack.clear();
        self.top = 0;
    }

    /// Clear the current stack, taking the values it held apart rather than
    /// leaving them to their destructors.
    ///
    /// This is what the machine uses - see [`Worklist::dismantle`].
    #[inline]
    pub(crate) fn dismantle_clear(&mut self, work: &mut Worklist) {
        work.dismantle_all(self.stack.iter_mut());
        self.stack.clear();
        self.top = 0;
    }

    /// Get the offset that corresponds to the bottom of the stack right now.
    ///
    /// The stack is partitioned into call frames, and once we enter a call
    /// frame the bottom of the stack corresponds to the bottom of the current
    /// call frame.
    #[cfg_attr(not(feature = "tracing"), allow(unused))]
    #[inline]
    pub(crate) const fn top(&self) -> usize {
        self.top
    }

    /// Get the length of the stack.
    #[cfg_attr(not(feature = "tracing"), allow(unused))]
    #[inline]
    pub(crate) const fn len(&self) -> usize {
        self.stack.len()
    }

    /// Swap the value at position a with the value at position b.
    pub(crate) fn swap(&mut self, a: Address, b: Address) -> Result<(), StackError> {
        if a == b {
            return Ok(());
        }

        let a = self
            .top
            .checked_add(a.offset())
            .filter(|&n| n < self.stack.len())
            .ok_or(StackError { addr: a })?;

        let b = self
            .top
            .checked_add(b.offset())
            .filter(|&n| n < self.stack.len())
            .ok_or(StackError { addr: b })?;

        self.stack.swap(a, b);
        Ok(())
    }

    /// Modify stack top by subtracting the given count from it while checking
    /// that it is in bounds of the stack.
    ///
    /// This is used internally when returning from a call frame.
    ///
    /// Returns the old stack top.
    #[tracing::instrument(skip_all)]
    pub(crate) fn swap_top(&mut self, addr: Address, len: usize) -> Result<usize, VmErrorKind> {
        let old_len = self.stack.len();

        if len == 0 {
            return Ok(replace(&mut self.top, old_len));
        }

        let Some(start) = self.top.checked_add(addr.offset()) else {
            return Err(VmErrorKind::StackError {
                error: StackError { addr },
            });
        };

        let Some(new_len) = old_len.checked_add(len) else {
            return Err(VmErrorKind::StackError {
                error: StackError { addr },
            });
        };

        if old_len < start + len {
            return Err(VmErrorKind::StackError {
                error: StackError { addr },
            });
        }

        self.stack.try_reserve(len)?;

        // SAFETY: We've ensured that the collection has space for the new
        // values. It is also guaranteed to be non-overlapping.
        unsafe {
            let ptr = self.stack.as_mut_ptr();
            let from = slice::from_raw_parts_mut(ptr.add(start), len);

            for (value, n) in from.iter_mut().zip(old_len..) {
                ptr.add(n).write(replace(value, Value::empty()));
            }

            self.stack.set_len(new_len);
        }

        Ok(replace(&mut self.top, old_len))
    }

    /// Pop the current stack top and modify it to a different one.
    ///
    /// The values which the call frame being left was working over are taken
    /// apart rather than left to their destructors, since this is the machine
    /// working - see [`Worklist::dismantle`].
    #[inline]
    #[tracing::instrument(skip_all)]
    pub(crate) fn pop_stack_top(&mut self, top: usize, work: &mut Worklist) {
        tracing::trace!(stack = self.stack.len(), self.top);

        if let Some(values) = self.stack.get_mut(self.top..) {
            work.dismantle_all(values.iter_mut());
        }

        self.stack.truncate(self.top);
        self.top = top;
    }

    /// Copy the value at the given address to the output.
    ///
    /// The value which is written over is taken apart rather than left to its
    /// destructor, since this is the machine working - see
    /// [`Worklist::dismantle`].
    pub(crate) fn copy(
        &mut self,
        from: Address,
        out: Output,
        work: &mut Worklist,
    ) -> Result<(), StoreError<Infallible>> {
        let Some(to) = out.as_addr() else {
            return Ok(());
        };

        if from == to {
            return Ok(());
        }

        let from = self.top.wrapping_add(from.offset());
        let to = self.top.wrapping_add(to.offset());

        if from.max(to) >= self.stack.len() {
            return Err(StackError {
                addr: Address::new(from.max(to).wrapping_sub(self.top)),
            }
            .into());
        }

        // SAFETY: We've checked that both addresses are in-bound and different
        // just above.
        let old = unsafe {
            let ptr = self.stack.as_mut_ptr();
            let value = (*ptr.add(from).cast_const()).clone();
            replace(&mut *ptr.add(to), value)
        };

        work.dismantle(old);
        Ok(())
    }

    /// Get a pair of addresses.
    pub(crate) fn pair(&mut self, a: Address, b: Address) -> Result<Pair<'_>, StackError> {
        if a == b {
            return Ok(Pair::Same(self.at_mut(a)?));
        }

        let a = self
            .top
            .checked_add(a.offset())
            .filter(|&n| n < self.stack.len())
            .ok_or(StackError { addr: a })?;

        let b = self
            .top
            .checked_add(b.offset())
            .filter(|&n| n < self.stack.len())
            .ok_or(StackError { addr: b })?;

        let pair = unsafe {
            let ptr = self.stack.as_mut_ptr();
            Pair::Pair(&mut *ptr.add(a), &*ptr.add(b).cast_const())
        };

        Ok(pair)
    }
}

/// A stack holds the values a machine is working over, so a machine which is
/// suspended - a generator or a stream - holds a graph of values through it.
impl Dismantle for Stack {
    fn dismantle(&mut self, out: &mut Handover<'_>) {
        // The values are being taken out of the machine, so which of them made
        // up its current call frame no longer means anything.
        self.top = 0;

        // Every value is handed over in one pass over the stack, and what is
        // left of it is dropped as soon as this returns.
        for value in self.stack.drain(..) {
            out.push(value);
        }
    }
}

impl Memory for Stack {
    #[inline]
    fn slice_at(&self, addr: Address, len: usize) -> Result<&[Value], SliceError> {
        Stack::slice_at(self, addr, len)
    }

    #[inline]
    fn slice_at_mut(&mut self, addr: Address, len: usize) -> Result<&mut [Value], SliceError> {
        Stack::slice_at_mut(self, addr, len)
    }

    #[inline]
    fn at_mut(&mut self, addr: Address) -> Result<&mut Value, StackError> {
        Stack::at_mut(self, addr)
    }
}

#[inline(always)]
fn inner_slice_at(values: &[Value], top: usize, addr: Address, len: usize) -> Option<&[Value]> {
    if len == 0 {
        return Some(&[]);
    }

    let start = top.checked_add(addr.offset())?;
    let end = start.checked_add(len)?;
    values.get(start..end)
}

#[inline(always)]
fn inner_slice_at_mut(
    values: &mut [Value],
    top: usize,
    addr: Address,
    len: usize,
) -> Option<&mut [Value]> {
    if len == 0 {
        return Some(&mut []);
    }

    let start = top.checked_add(addr.offset())?;
    let end = start.checked_add(len)?;
    values.get_mut(start..end)
}

#[inline(always)]
fn slice_error(stack: usize, bottom: usize, addr: Address, len: usize) -> SliceError {
    SliceError {
        addr,
        len,
        stack: stack.saturating_sub(bottom),
    }
}

impl TryClone for Stack {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            stack: self.stack.try_clone()?,
            top: self.top,
        })
    }
}

impl TryFromIteratorIn<Value, Global> for Stack {
    #[inline]
    fn try_from_iter_in<T: IntoIterator<Item = Value>>(
        iter: T,
        alloc: Global,
    ) -> alloc::Result<Self> {
        Ok(Self {
            stack: iter.into_iter().try_collect_in(alloc)?,
            top: 0,
        })
    }
}

impl From<Vec<Value>> for Stack {
    #[inline]
    fn from(stack: Vec<Value>) -> Self {
        Self { stack, top: 0 }
    }
}
