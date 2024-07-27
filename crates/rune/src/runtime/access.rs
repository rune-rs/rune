use core::cell::Cell;
use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

use crate::runtime::{AnyObjError, RawStr};

/// Test if exclusively held.
const EXCLUSIVE: usize = 1usize.rotate_right(2);
/// Sentinel value to indicate that access is taken.
const MOVED: usize = 1usize.rotate_right(1);
/// Mask indicating if the value is exclusively set or moved.
const MASK: usize = EXCLUSIVE | MOVED;

/// An error raised while downcasting.
#[derive(Debug, PartialEq)]
#[allow(missing_docs)]
#[non_exhaustive]
pub struct AccessError {
    kind: AccessErrorKind,
}

impl AccessError {
    #[inline]
    pub(crate) const fn empty() -> Self {
        Self {
            kind: AccessErrorKind::Empty,
        }
    }

    #[inline]
    pub(crate) fn new(kind: AccessErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            AccessErrorKind::Empty => write!(f, "Empty value"),
            AccessErrorKind::UnexpectedType { expected, actual } => write!(
                f,
                "Expected data of type `{expected}`, but found `{actual}`",
            ),
            AccessErrorKind::NotAccessibleRef { error } => error.fmt(f),
            AccessErrorKind::NotAccessibleMut { error } => error.fmt(f),
            AccessErrorKind::NotAccessibleTake { error } => error.fmt(f),
            AccessErrorKind::AnyObjError { error } => error.fmt(f),
        }
    }
}

cfg_std! {
    impl std::error::Error for AccessError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match &self.kind {
                AccessErrorKind::NotAccessibleRef { error, .. } => Some(error),
                AccessErrorKind::NotAccessibleMut { error, .. } => Some(error),
                AccessErrorKind::NotAccessibleTake { error, .. } => Some(error),
                AccessErrorKind::AnyObjError { error, .. } => Some(error),
                _ => None,
            }
        }
    }
}

impl From<NotAccessibleRef> for AccessError {
    #[inline]
    fn from(error: NotAccessibleRef) -> Self {
        AccessError::new(AccessErrorKind::NotAccessibleRef { error })
    }
}

impl From<NotAccessibleMut> for AccessError {
    #[inline]
    fn from(error: NotAccessibleMut) -> Self {
        AccessError::new(AccessErrorKind::NotAccessibleMut { error })
    }
}

impl From<NotAccessibleTake> for AccessError {
    #[inline]
    fn from(error: NotAccessibleTake) -> Self {
        AccessError::new(AccessErrorKind::NotAccessibleTake { error })
    }
}

impl From<AnyObjError> for AccessError {
    #[inline]
    fn from(source: AnyObjError) -> Self {
        AccessError::new(AccessErrorKind::AnyObjError { error: source })
    }
}

impl From<AccessErrorKind> for AccessError {
    #[inline]
    fn from(kind: AccessErrorKind) -> Self {
        AccessError::new(kind)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum AccessErrorKind {
    Empty,
    UnexpectedType { expected: RawStr, actual: RawStr },
    NotAccessibleRef { error: NotAccessibleRef },
    NotAccessibleMut { error: NotAccessibleMut },
    NotAccessibleTake { error: NotAccessibleTake },
    AnyObjError { error: AnyObjError },
}

/// Error raised when tried to access for shared access but it was not
/// accessible.
#[derive(Debug, PartialEq)]
pub(crate) struct NotAccessibleRef(Snapshot);

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
#[derive(Debug, PartialEq)]
pub(crate) struct NotAccessibleMut(Snapshot);

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
#[derive(Debug, PartialEq)]
pub(crate) struct NotAccessibleTake(Snapshot);

impl fmt::Display for NotAccessibleTake {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cannot take, value is {}", self.0)
    }
}

cfg_std! {
    impl std::error::Error for NotAccessibleTake {}
}

/// Snapshot that can be used to indicate how the value was being accessed at
/// the time of an error.
#[derive(PartialEq)]
#[repr(transparent)]
pub struct Snapshot(usize);

impl Snapshot {
    /// Test if the snapshot indicates that the value is readable.
    pub(crate) fn is_readable(&self) -> bool {
        self.0 & MASK == 0
    }

    /// The number of times a value is shared.
    pub(crate) fn shared(&self) -> usize {
        self.0 & !MASK
    }
}

impl fmt::Display for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0 => write!(f, "fully accessible")?,
            EXCLUSIVE => write!(f, "exclusively accessed")?,
            MOVED => write!(f, "moved")?,
            n => write!(f, "shared by {}", n)?,
        }

        Ok(())
    }
}

impl fmt::Debug for Snapshot {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Snapshot({})", self)
    }
}

/// Access flags.
///
/// These accomplish the following things:
/// * Indicates if a value is exclusively held.
/// * Indicates if a value is taken .
/// * Indicates if a value is shared, and if so by how many.
#[repr(transparent)]
pub(crate) struct Access(Cell<usize>);

impl Access {
    /// Construct a new default access.
    pub(crate) const fn new() -> Self {
        Self(Cell::new(0))
    }

    /// Test if we can have shared access without modifying the internal count.
    #[inline(always)]
    pub(crate) fn is_shared(&self) -> bool {
        self.0.get() & MASK == 0
    }

    /// Test if we can have exclusive access without modifying the internal
    /// count.
    #[inline(always)]
    pub(crate) fn is_exclusive(&self) -> bool {
        self.0.get() == 0
    }

    /// Test if the data has been taken.
    #[inline(always)]
    pub(crate) fn is_taken(&self) -> bool {
        self.0.get() & MOVED != 0
    }

    /// Mark that we want shared access to the given access token.
    #[inline(always)]
    pub(crate) fn shared(&self) -> Result<AccessGuard<'_>, NotAccessibleRef> {
        let state = self.0.get();

        if state == MASK {
            crate::alloc::abort();
        }

        if state & MASK != 0 {
            return Err(NotAccessibleRef(Snapshot(state)));
        }

        self.0.set(state + 1);
        Ok(AccessGuard(self))
    }

    /// Mark that we want exclusive access to the given access token.
    #[inline(always)]
    pub(crate) fn exclusive(&self) -> Result<AccessGuard<'_>, NotAccessibleMut> {
        let state = self.0.get();

        if state != 0 {
            return Err(NotAccessibleMut(Snapshot(state)));
        }

        self.0.set(EXCLUSIVE);
        Ok(AccessGuard(self))
    }

    /// Mark that we want to mark the given access as "taken".
    ///
    /// I.e. whatever guarded data is no longer available.
    #[inline(always)]
    pub(crate) fn take(&self) -> Result<RawAccessGuard, NotAccessibleTake> {
        let state = self.0.get();

        if state != 0 {
            return Err(NotAccessibleTake(Snapshot(state)));
        }

        self.0.set(MOVED);
        Ok(RawAccessGuard(self.into()))
    }

    /// Release the current access, unless it's moved.
    #[inline(always)]
    fn release(&self) {
        let b = self.0.get();

        let b = if b & MASK == 0 { b - 1 } else { 0 };

        self.0.set(b);
    }

    /// Get a snapshot of current access.
    #[inline(always)]
    pub(super) fn snapshot(&self) -> Snapshot {
        Snapshot(self.0.get())
    }
}

impl fmt::Debug for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Snapshot(self.0.get()))
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
    /// use rune::runtime::{BorrowRef, Bytes};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes = bytes.borrow_bytes_ref()?;
    ///
    /// let bytes: BorrowRef<[u8]> = BorrowRef::map(bytes, |bytes| &bytes[0..2]);
    ///
    /// assert_eq!(&bytes[..], &[1u8, 2u8][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn map<U: ?Sized>(this: Self, m: impl FnOnce(&T) -> &U) -> BorrowRef<'a, U> {
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
    /// use rune::runtime::{BorrowRef, Bytes};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes = bytes.borrow_bytes_ref()?;
    ///
    /// let Ok(bytes) = BorrowRef::try_map(bytes, |bytes| bytes.get(0..2)) else {
    ///     panic!("Conversion failed");
    /// };
    ///
    /// assert_eq!(&bytes[..], &[1u8, 2u8][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn try_map<U: ?Sized>(
        this: Self,
        m: impl FnOnce(&T) -> Option<&U>,
    ) -> Result<BorrowRef<'a, U>, Self> {
        let Some(data) = m(this.data) else {
            return Err(BorrowRef {
                data: this.data,
                guard: this.guard,
            });
        };

        Ok(BorrowRef {
            data,
            guard: this.guard,
        })
    }
}

impl<T: ?Sized> ops::Deref for BorrowRef<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T: ?Sized> AsRef<T> for BorrowRef<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
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
pub(crate) struct AccessGuard<'a>(&'a Access);

impl AccessGuard<'_> {
    /// Convert into a raw guard which does not have a lifetime associated with
    /// it. Droping the raw guard will release the resource.
    ///
    /// # Safety
    ///
    /// Since we're losing track of the lifetime, caller must ensure that the
    /// access outlives the guard.
    pub(crate) unsafe fn into_raw(self) -> RawAccessGuard {
        RawAccessGuard(ptr::NonNull::from(ManuallyDrop::new(self).0))
    }
}

impl Drop for AccessGuard<'_> {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// A raw guard around some level of access which will be released once the guard is dropped.
#[repr(transparent)]
pub(crate) struct RawAccessGuard(ptr::NonNull<Access>);

impl Drop for RawAccessGuard {
    fn drop(&mut self) {
        unsafe { self.0.as_ref().release() }
    }
}

/// Guard for data exclusively borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
pub struct BorrowMut<'a, T: ?Sized> {
    data: ptr::NonNull<T>,
    guard: AccessGuard<'a>,
    _marker: PhantomData<&'a mut T>,
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
            data: ptr::NonNull::from(data),
            guard: AccessGuard(access),
            _marker: PhantomData,
        }
    }

    /// Map the mutable reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{BorrowMut, Bytes};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes = bytes.borrow_bytes_mut()?;
    ///
    /// let mut bytes: BorrowMut<[u8]> = BorrowMut::map(bytes, |bytes| &mut bytes[0..2]);
    ///
    /// assert_eq!(&mut bytes[..], &mut [1u8, 2u8][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn map<U: ?Sized>(mut this: Self, m: impl FnOnce(&mut T) -> &mut U) -> BorrowMut<'a, U> {
        // SAFETY: This is safe per construction.
        unsafe {
            BorrowMut {
                data: ptr::NonNull::from(m(this.data.as_mut())),
                guard: this.guard,
                _marker: PhantomData,
            }
        }
    }

    /// Try to map the mutable reference to a projection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{BorrowMut, Bytes};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes = bytes.borrow_bytes_mut()?;
    ///
    /// let Ok(mut bytes) = BorrowMut::try_map(bytes, |bytes| bytes.get_mut(0..2)) else {
    ///     panic!("Conversion failed");
    /// };
    ///
    /// assert_eq!(&mut bytes[..], &mut [1u8, 2u8][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn try_map<U: ?Sized>(
        mut this: Self,
        m: impl FnOnce(&mut T) -> Option<&mut U>,
    ) -> Result<BorrowMut<'a, U>, Self> {
        unsafe {
            let Some(data) = m(this.data.as_mut()) else {
                return Err(BorrowMut {
                    data: this.data,
                    guard: this.guard,
                    _marker: PhantomData,
                });
            };

            Ok(BorrowMut {
                data: ptr::NonNull::from(data),
                guard: this.guard,
                _marker: PhantomData,
            })
        }
    }
}

impl<T: ?Sized> ops::Deref for BorrowMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: This is correct per construction.
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> ops::DerefMut for BorrowMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: This is correct per construction.
        unsafe { self.data.as_mut() }
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
    use super::Access;

    #[test]
    fn access_shared() {
        let access = Access::new();

        assert!(access.is_shared());
        assert!(access.is_exclusive());
        assert!(!access.is_taken());

        let g1 = access.shared().unwrap();
        let g2 = access.shared().unwrap();

        assert!(access.is_shared());
        assert!(!access.is_exclusive());
        assert!(!access.is_taken());

        drop(g1);

        assert!(access.is_shared());
        assert!(!access.is_exclusive());
        assert!(!access.is_taken());

        drop(g2);

        assert!(access.is_shared());
        assert!(access.is_exclusive());
        assert!(!access.is_taken());
    }

    #[test]
    fn access_exclusive() {
        let access = Access::new();

        let guard = access.exclusive().unwrap();
        assert!(access.exclusive().is_err());

        assert!(!access.is_shared());
        assert!(!access.is_exclusive());
        assert!(!access.is_taken());

        drop(guard);

        assert!(access.is_shared());
        assert!(access.is_exclusive());
        assert!(!access.is_taken());
    }

    #[test]
    fn access_take() {
        let access = Access::new();

        let guard = access.exclusive().unwrap();
        assert!(access.take().is_err());
        drop(guard);

        let g = access.take().unwrap();

        assert!(!access.is_shared());
        assert!(!access.is_exclusive());
        assert!(access.is_taken());

        drop(g);

        assert!(access.is_shared());
        assert!(access.is_exclusive());
        assert!(!access.is_taken());
    }
}
