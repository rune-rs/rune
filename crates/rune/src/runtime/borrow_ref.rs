use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::NonNull;

use super::AccessGuard;

/// Guard for a data borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
pub struct BorrowRef<'a, T: ?Sized + 'a> {
    data: NonNull<T>,
    guard: AccessGuard<'a>,
    _marker: PhantomData<&'a T>,
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
    pub(crate) unsafe fn new(data: NonNull<T>, guard: AccessGuard<'a>) -> Self {
        Self {
            data,
            guard,
            _marker: PhantomData,
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
    /// let bytes = bytes.borrow_ref::<Bytes>()?;
    ///
    /// let bytes: BorrowRef<[u8]> = BorrowRef::map(bytes, |bytes| &bytes[0..2]);
    ///
    /// assert_eq!(&bytes[..], &[1u8, 2u8][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn map<U: ?Sized>(this: Self, m: impl FnOnce(&T) -> &U) -> BorrowRef<'a, U> {
        unsafe {
            BorrowRef {
                data: NonNull::from(m(this.data.as_ref())),
                guard: this.guard,
                _marker: PhantomData,
            }
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
    /// let bytes = bytes.borrow_ref::<Bytes>()?;
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
        unsafe {
            let Some(data) = m(this.data.as_ref()) else {
                return Err(BorrowRef {
                    data: this.data,
                    guard: this.guard,
                    _marker: PhantomData,
                });
            };

            Ok(BorrowRef {
                data: NonNull::from(data),
                guard: this.guard,
                _marker: PhantomData,
            })
        }
    }
}

impl<T: ?Sized> Deref for BorrowRef<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> AsRef<T> for BorrowRef<'_, T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.data.as_ref() }
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
