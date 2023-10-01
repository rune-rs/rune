use core::cell::Cell;
use core::fmt;
use core::future::Future;
use core::mem::ManuallyDrop;
use core::ops;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

use crate::runtime::{AnyObjError, RawStr};

/// Bitflag which if set indicates that the accessed value is an external
/// reference (exclusive or not).
const IS_REF_MASK: isize = 1isize;
/// Sentinel value to indicate that access is taken.
const TAKEN: isize = (isize::max_value() ^ IS_REF_MASK) >> 1;
/// Panic if we reach this number of shared accesses and we try to add one more,
/// since it's the largest we can support.
const MAX_USES: isize = 0b11isize.rotate_right(2);

/// An error raised while downcasting.
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum AccessError {
    UnexpectedType { expected: RawStr, actual: RawStr },
    NotAccessibleRef { error: NotAccessibleRef },
    NotAccessibleMut { error: NotAccessibleMut },
    NotAccessibleTake { error: NotAccessibleTake },
    AnyObjError { error: AnyObjError },
}

cfg_std! {
    impl std::error::Error for AccessError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                AccessError::NotAccessibleRef { error, .. } => Some(error),
                AccessError::NotAccessibleMut { error, .. } => Some(error),
                AccessError::NotAccessibleTake { error, .. } => Some(error),
                AccessError::AnyObjError { error, .. } => Some(error),
                _ => None,
            }
        }
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AccessError::UnexpectedType { expected, actual } => write!(
                f,
                "Expected data of type `{expected}`, but found `{actual}`",
            ),
            AccessError::NotAccessibleRef { error } => error.fmt(f),
            AccessError::NotAccessibleMut { error } => error.fmt(f),
            AccessError::NotAccessibleTake { error } => error.fmt(f),
            AccessError::AnyObjError { error } => error.fmt(f),
        }
    }
}

impl From<NotAccessibleRef> for AccessError {
    #[inline]
    fn from(error: NotAccessibleRef) -> Self {
        AccessError::NotAccessibleRef { error }
    }
}

impl From<NotAccessibleMut> for AccessError {
    #[inline]
    fn from(error: NotAccessibleMut) -> Self {
        AccessError::NotAccessibleMut { error }
    }
}

impl From<NotAccessibleTake> for AccessError {
    #[inline]
    fn from(error: NotAccessibleTake) -> Self {
        AccessError::NotAccessibleTake { error }
    }
}

impl From<AnyObjError> for AccessError {
    #[inline]
    fn from(source: AnyObjError) -> Self {
        AccessError::AnyObjError { error: source }
    }
}

/// Error raised when tried to access for shared access but it was not
/// accessible.
#[derive(Debug)]
pub struct NotAccessibleRef(Snapshot);

impl fmt::Display for NotAccessibleRef {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cannot read, value is {}", self.0)
    }
}

cfg_std! {
    impl std::error::Error for NotAccessibleRef {}
}

/// Error raised when tried to access for exclusive access but it was not
/// accessible.
#[derive(Debug)]
pub struct NotAccessibleMut(Snapshot);

impl fmt::Display for NotAccessibleMut {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cannot write, value is {}", self.0)
    }
}

cfg_std! {
    impl std::error::Error for NotAccessibleMut {}
}

/// Error raised when tried to access the guarded data for taking.
///
/// This requires exclusive access, but it's a scenario we structure separately
/// for diagnostics purposes.
#[derive(Debug)]
pub struct NotAccessibleTake(Snapshot);

impl fmt::Display for NotAccessibleTake {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cannot take, value is {}", self.0)
    }
}

cfg_std! {
    impl std::error::Error for NotAccessibleTake {}
}

/// The kind of access to perform.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AccessKind {
    /// Access a reference.
    Any,
    /// Access something owned.
    Owned,
}

/// Snapshot that can be used to indicate how the value was being accessed at
/// the time of an error.
#[derive(Debug)]
#[repr(transparent)]
struct Snapshot(isize);

impl fmt::Display for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 >> 1 {
            0 => write!(f, "fully accessible")?,
            1 => write!(f, "exclusively accessed")?,
            TAKEN => write!(f, "moved")?,
            n if n < 0 => write!(f, "shared by {}", -n)?,
            n => write!(f, "invalidly marked ({})", n)?,
        }

        if self.0 & IS_REF_MASK == 1 {
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
///
/// This means that the maximum number of accesses for a 64-bit `isize` is
/// `(1 << 62) - 1` uses.
///
/// ```
#[repr(transparent)]
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
        self.0.get() & IS_REF_MASK != 0
    }

    /// Test if we can have shared access without modifying the internal count.
    #[inline]
    pub(crate) fn is_shared(&self) -> bool {
        self.get() <= 0
    }

    /// Test if we can have exclusive access without modifying the internal
    /// count.
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
    ) -> Result<AccessGuard<'_>, NotAccessibleRef> {
        if let AccessKind::Owned = kind {
            if self.is_ref() {
                return Err(NotAccessibleRef(Snapshot(self.0.get())));
            }
        }

        let state = self.get();

        if state == MAX_USES {
            crate::alloc::abort();
        }

        let n = state.wrapping_sub(1);

        if n >= 0 {
            return Err(NotAccessibleRef(Snapshot(self.0.get())));
        }

        self.set(n);
        Ok(AccessGuard(self))
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
    ) -> Result<AccessGuard<'_>, NotAccessibleMut> {
        if let AccessKind::Owned = kind {
            if self.is_ref() {
                return Err(NotAccessibleMut(Snapshot(self.0.get())));
            }
        }

        let n = self.get();

        if n != 0 {
            return Err(NotAccessibleMut(Snapshot(self.0.get())));
        }

        self.set(n.wrapping_add(1));
        Ok(AccessGuard(self))
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

    /// Release the current access level.
    #[inline]
    fn release(&self) {
        let b = self.get();

        let b = if b < 0 {
            debug_assert!(b < 0);
            b.wrapping_add(1)
        } else {
            debug_assert_eq!(b, 1, "borrow value should be exclusive (0)");
            b.wrapping_sub(1)
        };

        self.set(b);
    }

    /// Untake the current access.
    #[inline]
    fn release_take(&self) {
        let b = self.get();
        debug_assert_eq!(b, TAKEN, "borrow value should be TAKEN ({})", TAKEN);
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
        self.0.set(self.0.get() & IS_REF_MASK | value << 1);
    }
}

impl fmt::Debug for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Snapshot(self.get()))
    }
}

/// Guard for a data borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
pub struct BorrowRef<'a, T: ?Sized + 'a> {
    data: &'a T,
    guard: AccessGuard<'a>,
}

impl<'a, T: ?Sized> BorrowRef<'a, T> {
    /// Construct a new shared guard.
    ///
    /// # Safety
    ///
    /// since this has implications for releasing access, the caller must
    /// ensure that access has been acquired correctly using e.g.
    /// [Access::shared]. Otherwise access can be release incorrectly once
    /// this guard is dropped.
    pub(crate) fn new(data: &'a T, access: &'a Access) -> Self {
        Self {
            data,
            guard: AccessGuard(access),
        }
    }

    /// Map the reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{BorrowRef, Shared};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.borrow_ref()?;
    /// let value: BorrowRef<[u32]> = BorrowRef::map(vec, |vec| &vec[0..2]);
    ///
    /// assert_eq!(&*value, &[1u32, 2u32][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn map<M, U: ?Sized>(this: Self, m: M) -> BorrowRef<'a, U>
    where
        M: FnOnce(&T) -> &U,
    {
        BorrowRef {
            data: m(this.data),
            guard: this.guard,
        }
    }

    /// Try to map the reference to a projection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{BorrowRef, Shared};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.borrow_ref()?;
    /// let mut value: Option<BorrowRef<[u32]>> = BorrowRef::try_map(vec, |vec| vec.get(0..2));
    ///
    /// assert_eq!(value.as_deref(), Some(&[1u32, 2u32][..]));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn try_map<M, U: ?Sized>(this: Self, m: M) -> Option<BorrowRef<'a, U>>
    where
        M: FnOnce(&T) -> Option<&U>,
    {
        Some(BorrowRef {
            data: m(this.data)?,
            guard: this.guard,
        })
    }
}

impl<T: ?Sized> ops::Deref for BorrowRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
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

/// A guard around some specific access access.
#[repr(transparent)]
pub struct AccessGuard<'a>(&'a Access);

impl AccessGuard<'_> {
    /// Convert into a raw guard which does not have a lifetime associated with
    /// it. Droping the raw guard will release the resource.
    ///
    /// # Safety
    ///
    /// Since we're losing track of the lifetime, caller must ensure that the
    /// access outlives the guard.
    pub unsafe fn into_raw(self) -> RawAccessGuard {
        RawAccessGuard(ptr::NonNull::from(ManuallyDrop::new(self).0))
    }
}

impl Drop for AccessGuard<'_> {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// A raw guard around some level of access.
#[repr(transparent)]
pub struct RawAccessGuard(ptr::NonNull<Access>);

impl Drop for RawAccessGuard {
    fn drop(&mut self) {
        unsafe { self.0.as_ref().release() }
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
    data: &'a mut T,
    guard: AccessGuard<'a>,
}

impl<'a, T: ?Sized> BorrowMut<'a, T> {
    /// Construct a new exclusive guard.
    ///
    /// # Safety
    ///
    /// since this has implications for releasing access, the caller must
    /// ensure that access has been acquired correctly using e.g.
    /// [Access::exclusive]. Otherwise access can be release incorrectly once
    /// this guard is dropped.
    pub(crate) unsafe fn new(data: &'a mut T, access: &'a Access) -> Self {
        Self {
            data,
            guard: AccessGuard(access),
        }
    }

    /// Map the mutable reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{BorrowMut, Shared};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.borrow_mut()?;
    /// let value: BorrowMut<[u32]> = BorrowMut::map(vec, |vec| &mut vec[0..2]);
    ///
    /// assert_eq!(&*value, &mut [1u32, 2u32][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn map<M, U: ?Sized>(this: Self, m: M) -> BorrowMut<'a, U>
    where
        M: FnOnce(&mut T) -> &mut U,
    {
        BorrowMut {
            data: m(this.data),
            guard: this.guard,
        }
    }

    /// Try to map the mutable reference to a projection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{BorrowMut, Shared};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.borrow_mut()?;
    /// let mut value: Option<BorrowMut<[u32]>> = BorrowMut::try_map(vec, |vec| vec.get_mut(0..2));
    ///
    /// assert_eq!(value.as_deref_mut(), Some(&mut [1u32, 2u32][..]));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn try_map<M, U: ?Sized>(this: Self, m: M) -> Option<BorrowMut<'a, U>>
    where
        M: FnOnce(&mut T) -> Option<&mut U>,
    {
        Some(BorrowMut {
            data: m(this.data)?,
            guard: this.guard,
        })
    }
}

impl<T: ?Sized> ops::Deref for BorrowMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: ?Sized> ops::DerefMut for BorrowMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
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
