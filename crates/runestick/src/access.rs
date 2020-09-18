use crate::RawStr;
use std::cell::Cell;
use std::fmt;
use std::future::Future;
use std::marker;
use std::ops;
use std::pin::Pin;
use std::task::{Context, Poll};
use thiserror::Error;

/// Flag to used to mark access as taken.
const FLAG: isize = 1isize;
/// Sentinel value to indicate that access is taken.
const TAKEN: isize = (isize::max_value() ^ FLAG) >> 1;
/// Panic if we reach this number of shared accesses and we try to add one more,
/// since it's the largest we can support.
const MAX_USES: isize = 0b11isize.rotate_right(2);

/// An error raised while downcasting.
#[derive(Debug, Error)]
pub enum AccessError {
    /// Error raised when we expect a specific external type but got another.
    #[error("expected data of type `{expected}`, but found `{actual}`")]
    UnexpectedType {
        /// The type that was expected.
        expected: RawStr,
        /// The type that was found.
        actual: RawStr,
    },
    /// Trying to access an inaccessible reference.
    #[error("{error}")]
    NotAccessibleRef {
        /// Source error.
        #[from]
        error: NotAccessibleRef,
    },
    /// Trying to access an inaccessible mutable reference.
    #[error("{error}")]
    NotAccessibleMut {
        /// Source error.
        #[from]
        error: NotAccessibleMut,
    },
    /// Trying to access an inaccessible taking.
    #[error("{error}")]
    NotAccessibleTake {
        /// Source error.
        #[from]
        error: NotAccessibleTake,
    },
}

/// The kind of access to perform.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AccessKind {
    /// Access a reference.
    Any,
    /// Access something owned.
    Owned,
}

/// Error raised when tried to access for shared access but it was not
/// accessible.
#[derive(Debug, Error)]
#[error("cannot read, value is {0}")]
pub struct NotAccessibleRef(Snapshot);

/// Error raised when tried to access for exclusive access but it was not
/// accessible.
#[derive(Debug, Error)]
#[error("cannot write, value is {0}")]
pub struct NotAccessibleMut(Snapshot);

/// Error raised when tried to access the guarded data for taking.
///
/// This requires exclusive access, but it's a scenario we structure separately
/// for diagnostics purposes.
#[derive(Debug, Error)]
#[error("cannot take, value is {0}")]
pub struct NotAccessibleTake(Snapshot);

/// Snapshot that can be used to indicate how the value was being accessed at
/// the time of an error.
#[derive(Debug)]
#[repr(transparent)]
pub struct Snapshot(isize);

impl fmt::Display for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 >> 1 {
            0 => write!(f, "fully accessible")?,
            1 => write!(f, "exclusively accessed")?,
            TAKEN => write!(f, "moved")?,
            n if n < 0 => write!(f, "shared by {}", -n)?,
            n => write!(f, "invalidly marked ({})", n)?,
        }

        if self.0 & FLAG == 1 {
            write!(f, " (ref)")?;
        }

        Ok(())
    }
}

/// Access flags.
///
/// These accomplish the following things:
/// * Indicates if a value is a reference.
/// * Indicates if a value is exclusively held.
/// * Indicates if a value is shared, and if so by how many.
///
/// It has the following bit-pattern (assume isize is 16 bits for simplicity):
///
/// ```text
/// S0000000_00000000_00000000_0000000F
/// |                                ||
/// '-- Sign bit and number base ----'|
///                   Reference Flag -'
///
/// The reference flag is the LSB, and the rest is treated as a signed number
/// with the following properties:
/// * If the value is `0`, it is not being accessed.
/// * If the value is `1`, it is being exclusively accessed.
/// * If the value is negative `n`, it is being shared accessed by `-n` uses.
/// * If the value is
///
/// This means that the maximum number of accesses for a 64-bit `isize` is
/// `(1 << 62) - 1` uses.
///
/// ```
#[derive(Clone)]
pub(crate) struct Access(Cell<isize>);

impl Access {
    /// Construct a new default access.
    pub(crate) const fn new(is_ref: bool) -> Self {
        let initial = if is_ref { 1 } else { 0 };
        Self(Cell::new(initial))
    }

    /// Test if access is guarding a reference.
    #[inline]
    pub(crate) fn is_ref(&self) -> bool {
        self.0.get() & FLAG != 0
    }

    /// Test if we have shared access without modifying the internal count.
    #[inline]
    pub(crate) fn is_shared(&self) -> bool {
        self.get().wrapping_sub(1) < 0
    }

    /// Test if we have exclusive access without modifying the internal count.
    #[inline]
    pub(crate) fn is_exclusive(&self) -> bool {
        self.get() == 0
    }

    /// Test if the data has been taken.
    #[inline]
    pub(crate) fn is_taken(&self) -> bool {
        self.get() == TAKEN
    }

    /// Mark that we want shared access to the given access token.
    ///
    /// # Safety
    ///
    /// The returned guard must not outlive the access token that created it.
    #[inline]
    pub(crate) unsafe fn shared(
        &self,
        kind: AccessKind,
    ) -> Result<RawSharedGuard, NotAccessibleRef> {
        if let AccessKind::Owned = kind {
            if self.is_ref() {
                return Err(NotAccessibleRef(Snapshot(self.0.get())));
            }
        }

        let state = self.get();

        if state == MAX_USES {
            std::process::abort();
        }

        let n = state.wrapping_sub(1);

        if n >= 0 {
            return Err(NotAccessibleRef(Snapshot(self.0.get())));
        }

        self.set(n);
        Ok(RawSharedGuard { access: self })
    }

    /// Mark that we want exclusive access to the given access token.
    ///
    /// # Safety
    ///
    /// The returned guard must not outlive the access token that created it.
    #[inline]
    pub(crate) unsafe fn exclusive(
        &self,
        kind: AccessKind,
    ) -> Result<RawExclusiveGuard, NotAccessibleMut> {
        if let AccessKind::Owned = kind {
            if self.is_ref() {
                return Err(NotAccessibleMut(Snapshot(self.0.get())));
            }
        }

        let state = self.get();
        let n = state.wrapping_add(1);

        if n != 1 {
            return Err(NotAccessibleMut(Snapshot(self.0.get())));
        }

        self.set(n);
        Ok(RawExclusiveGuard { access: self })
    }

    /// Mark that we want to mark the given access as "taken".
    ///
    /// I.e. whatever guarded data is no longer available.
    ///
    /// # Safety
    ///
    /// The returned guard must not outlive the access token that created it.
    #[inline]
    pub(crate) unsafe fn take(&self, kind: AccessKind) -> Result<RawTakeGuard, NotAccessibleTake> {
        if let AccessKind::Owned = kind {
            if self.is_ref() {
                return Err(NotAccessibleTake(Snapshot(self.0.get())));
            }
        }

        let state = self.get();

        if state != 0 {
            return Err(NotAccessibleTake(Snapshot(self.0.get())));
        }

        self.set(TAKEN);
        Ok(RawTakeGuard { access: self })
    }

    /// Unshare the current access.
    #[inline]
    fn release_shared(&self) {
        let b = self.get().wrapping_add(1);
        debug_assert!(b <= 0);
        self.set(b);
    }

    /// Unshare the current access.
    #[inline]
    fn release_exclusive(&self) {
        let b = self.get().wrapping_sub(1);
        debug_assert!(b == 0);
        self.set(b);
    }

    /// Untake the current access.
    #[inline]
    fn release_take(&self) {
        let b = self.get();
        debug_assert!(b == TAKEN);
        self.set(0);
    }

    /// Get the current value of the flag.
    #[inline]
    fn get(&self) -> isize {
        self.0.get() >> 1
    }

    /// Set the current value of the flag.
    #[inline]
    fn set(&self, value: isize) {
        self.0.set(self.0.get() & FLAG | value << 1);
    }
}

impl fmt::Debug for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Snapshot(self.get()))
    }
}

/// A shared access guard.
///
/// This is created with [Access::shared], and must not outlive the [Access]
/// instance it was created from.
pub struct RawSharedGuard {
    access: *const Access,
}

impl Drop for RawSharedGuard {
    fn drop(&mut self) {
        unsafe { (*self.access).release_shared() };
    }
}

/// Guard for a data borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
pub struct BorrowRef<'a, T: ?Sized + 'a> {
    data: *const T,
    guard: RawSharedGuard,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T: ?Sized> BorrowRef<'a, T> {
    /// Construct a new raw reference guard.
    ///
    /// # Safety
    ///
    /// The provided components must be valid for the lifetime of the returned
    /// reference, which is unbounded.
    pub(crate) unsafe fn from_raw(data: *const T, guard: RawSharedGuard) -> Self {
        Self {
            data,
            guard,
            _marker: marker::PhantomData,
        }
    }

    /// Try to map the interior reference the reference.
    pub fn try_map<M, U: ?Sized, E>(this: Self, m: M) -> Result<BorrowRef<'a, U>, E>
    where
        M: FnOnce(&T) -> Result<&U, E>,
    {
        let data = m(unsafe { &*this.data })?;
        let guard = this.guard;

        Ok(BorrowRef {
            data,
            guard,
            _marker: marker::PhantomData,
        })
    }
}

impl<T: ?Sized> ops::Deref for BorrowRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: ?Sized> fmt::Debug for BorrowRef<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

/// An exclusive access guard.
///
/// This is created with [Access::exclusive], and must not outlive the [Access]
/// instance it was created from.
pub struct RawExclusiveGuard {
    access: *const Access,
}

impl Drop for RawExclusiveGuard {
    fn drop(&mut self) {
        unsafe { (*self.access).release_exclusive() }
    }
}

/// A taken access guard.
///
/// This is created with [Access::take], and must not outlive the [Access]
/// instance it was created from.
pub(crate) struct RawTakeGuard {
    access: *const Access,
}

impl Drop for RawTakeGuard {
    fn drop(&mut self) {
        unsafe { (*self.access).release_take() }
    }
}

/// Guard for data exclusively borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
pub struct BorrowMut<'a, T: ?Sized> {
    data: *mut T,
    guard: RawExclusiveGuard,
    _marker: marker::PhantomData<&'a mut T>,
}

impl<'a, T: ?Sized> BorrowMut<'a, T> {
    /// Construct a new raw reference guard.
    ///
    /// # Safety
    ///
    /// The provided components must be valid for the lifetime of the returned
    /// reference, which is unbounded.
    pub(crate) unsafe fn from_raw(data: *mut T, guard: RawExclusiveGuard) -> Self {
        Self {
            data,
            guard,
            _marker: marker::PhantomData,
        }
    }

    /// Map the mutable reference.
    pub fn map<M, U: ?Sized>(this: Self, m: M) -> BorrowMut<'a, U>
    where
        M: FnOnce(&mut T) -> &mut U,
    {
        let data = m(unsafe { &mut *this.data });
        let guard = this.guard;

        BorrowMut {
            data,
            guard,
            _marker: marker::PhantomData,
        }
    }

    /// Try to optionally map the mutable reference.
    pub fn try_map<M, U: ?Sized>(this: Self, m: M) -> Option<BorrowMut<'a, U>>
    where
        M: FnOnce(&mut T) -> Option<&mut U>,
    {
        let data = m(unsafe { &mut *this.data })?;

        Some(BorrowMut {
            data,
            guard: this.guard,
            _marker: marker::PhantomData,
        })
    }
}

impl<T: ?Sized> ops::Deref for BorrowMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: ?Sized> ops::DerefMut for BorrowMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<T: ?Sized> fmt::Debug for BorrowMut<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

impl<F> Future for BorrowMut<'_, F>
where
    F: Unpin + Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // NB: inner Future is Unpin.
        let this = self.get_mut();
        Pin::new(&mut **this).poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, AccessKind};

    #[test]
    fn test_non_ref() {
        unsafe {
            let access = Access::new(false);

            assert!(!access.is_ref());
            assert!(access.is_shared());
            assert!(access.is_exclusive());

            let guard = access.shared(AccessKind::Any).unwrap();

            assert!(!access.is_ref());
            assert!(access.is_shared());
            assert!(!access.is_exclusive());

            drop(guard);

            assert!(!access.is_ref());
            assert!(access.is_shared());
            assert!(access.is_exclusive());

            let guard = access.exclusive(AccessKind::Any).unwrap();

            assert!(!access.is_ref());
            assert!(!access.is_shared());
            assert!(!access.is_exclusive());

            drop(guard);

            assert!(!access.is_ref());
            assert!(access.is_shared());
            assert!(access.is_exclusive());
        }
    }

    #[test]
    fn test_ref() {
        unsafe {
            let access = Access::new(true);

            assert!(access.is_ref());
            assert!(access.is_shared());
            assert!(access.is_exclusive());

            let guard = access.shared(AccessKind::Any).unwrap();

            assert!(access.is_ref());
            assert!(access.is_shared());
            assert!(!access.is_exclusive());

            drop(guard);

            assert!(access.is_ref());
            assert!(access.is_shared());
            assert!(access.is_exclusive());

            let guard = access.exclusive(AccessKind::Any).unwrap();

            assert!(access.is_ref());
            assert!(!access.is_shared());
            assert!(!access.is_exclusive());

            drop(guard);

            assert!(access.is_ref());
            assert!(access.is_shared());
            assert!(access.is_exclusive());
        }
    }
}
