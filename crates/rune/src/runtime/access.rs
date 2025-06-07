use core::cell::Cell;
use core::fmt;
use core::mem::ManuallyDrop;
use core::ptr::NonNull;

/// Test if exclusively held.
const EXCLUSIVE: usize = 1usize.rotate_right(2);
/// Sentinel value to indicate that access is taken.
const MOVED: usize = 1usize.rotate_right(1);
/// Mask indicating if the value is exclusively set or moved.
const MASK: usize = EXCLUSIVE | MOVED;

/// An error raised when failing to access a value.
///
/// Access errors can be raised for various reasons, such as:
/// * The value you are trying to access is an empty placeholder.
/// * The value is already being accessed in an incompatible way, such as trying
///   to access a value exclusively twice.
/// * The value has been taken and is no longer present.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[non_exhaustive]
pub struct AccessError {
    kind: AccessErrorKind,
}

impl AccessError {
    #[inline]
    const fn new(kind: AccessErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for AccessError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            AccessErrorKind::NotAccessibleRef(s) => write!(
                f,
                "Cannot read, value has snapshot {s} and is not available for reading"
            ),
            AccessErrorKind::NotAccessibleMut(s) => write!(
                f,
                "Cannot write, value has snapshot {s} and is not available for writing"
            ),
            AccessErrorKind::NotAccessibleTake(s) => write!(
                f,
                "Cannot take, value has snapshot {s} and is not available for taking"
            ),
        }
    }
}

impl core::error::Error for AccessError {}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum AccessErrorKind {
    NotAccessibleRef(Snapshot),
    NotAccessibleMut(Snapshot),
    NotAccessibleTake(Snapshot),
}

/// Snapshot that can be used to indicate how the value was being accessed at
/// the time of an error.
#[derive(PartialEq)]
#[repr(transparent)]
pub(crate) struct Snapshot(usize);

impl Snapshot {
    /// Test if the snapshot indicates that the value is readable.
    pub(crate) fn is_readable(&self) -> bool {
        self.0 & MASK == 0
    }

    /// Test if the snapshot indicates that the value is writable.
    pub(crate) fn is_writable(&self) -> bool {
        self.0 & MASK == 0
    }

    /// Test if access is exclusively held.
    pub(crate) fn is_exclusive(&self) -> bool {
        self.0 & MASK != 0
    }

    /// The number of times a value is shared.
    pub(crate) fn shared(&self) -> usize {
        self.0 & !MASK
    }
}

impl fmt::Display for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 & MOVED != 0 {
            write!(f, "M")?;
        } else {
            write!(f, "-")?;
        }

        if self.0 & EXCLUSIVE != 0 {
            write!(f, "X")?;
        } else {
            write!(f, "-")?;
        }

        write!(f, "{:06}", self.shared())?;
        Ok(())
    }
}

impl fmt::Debug for Snapshot {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Snapshot({self})")
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
    pub(crate) fn shared(&self) -> Result<AccessGuard<'_>, AccessError> {
        self.try_shared()?;
        Ok(AccessGuard(self))
    }

    #[inline(always)]
    pub(crate) fn try_shared(&self) -> Result<(), AccessError> {
        let state = self.0.get();

        if state & MASK != 0 {
            debug_assert_eq!(
                state & !MASK,
                0,
                "count should be zero, but was {}",
                Snapshot(state)
            );
            return Err(AccessError::new(AccessErrorKind::NotAccessibleRef(
                Snapshot(state),
            )));
        }

        // NB: Max number of shared.
        if state == !MASK {
            crate::alloc::abort();
        }

        self.0.set(state + 1);
        Ok(())
    }

    /// Mark that we want exclusive access to the given access token.
    #[inline(always)]
    pub(crate) fn exclusive(&self) -> Result<AccessGuard<'_>, AccessError> {
        self.try_exclusive()?;
        Ok(AccessGuard(self))
    }

    #[inline(always)]
    pub(crate) fn try_exclusive(&self) -> Result<(), AccessError> {
        let state = self.0.get();

        if state != 0 {
            return Err(AccessError::new(AccessErrorKind::NotAccessibleMut(
                Snapshot(state),
            )));
        }

        self.0.set(state | EXCLUSIVE);
        Ok(())
    }

    /// Mark that we want to mark the given access as "taken".
    ///
    /// I.e. whatever guarded data is no longer available.
    #[inline(always)]
    pub(crate) fn try_take(&self) -> Result<(), AccessError> {
        let state = self.0.get();

        if state != 0 {
            return Err(AccessError::new(AccessErrorKind::NotAccessibleTake(
                Snapshot(state),
            )));
        }

        self.0.set(state | MOVED);
        Ok(())
    }

    /// Unconditionally mark the given access as "taken".
    #[inline(always)]
    pub(crate) fn take(&self) {
        let state = self.0.get();
        self.0.set(state | MOVED);
    }

    /// Release the current access, unless it's moved.
    #[inline(always)]
    pub(super) fn release(&self) {
        let b = self.0.get();

        let b = if b & EXCLUSIVE != 0 {
            b & !EXCLUSIVE
        } else {
            debug_assert_ne!(b & !MASK, 0, "count should be zero but was {}", Snapshot(b));
            b - 1
        };

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
        RawAccessGuard(NonNull::from(ManuallyDrop::new(self).0))
    }
}

impl Drop for AccessGuard<'_> {
    fn drop(&mut self) {
        self.0.release();
    }
}

/// A raw guard around some level of access which will be released once the guard is dropped.
#[repr(transparent)]
pub(crate) struct RawAccessGuard(NonNull<Access>);

impl Drop for RawAccessGuard {
    fn drop(&mut self) {
        unsafe { self.0.as_ref().release() }
    }
}
