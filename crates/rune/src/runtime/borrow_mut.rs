use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll};

use super::RawAccessGuard;

/// Guard for data exclusively borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
pub struct BorrowMut<'a, T: ?Sized> {
    data: NonNull<T>,
    guard: Option<RawAccessGuard>,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: ?Sized> BorrowMut<'a, T> {
    /// Construct a borrow mut from static data.
    #[inline]
    pub(crate) fn from_static(data: &mut T) -> Self {
        Self {
            data: NonNull::from(data),
            guard: None,
            _marker: PhantomData,
        }
    }

    /// Construct a new exclusive guard.
    ///
    /// # Safety
    ///
    /// since this has implications for releasing access, the caller must
    /// ensure that access has been acquired correctly using e.g.
    /// [Access::exclusive]. Otherwise access can be release incorrectly once
    /// this guard is dropped.
    #[inline]
    pub(crate) unsafe fn new(data: NonNull<T>, guard: RawAccessGuard) -> Self {
        Self {
            data,
            guard: Some(guard),
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
    /// let bytes = bytes.borrow_mut::<Bytes>()?;
    ///
    /// let mut bytes = BorrowMut::map(bytes, |bytes| &mut bytes[0..2]);
    ///
    /// assert_eq!(&mut bytes[..], &mut [1u8, 2u8][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn map<U: ?Sized>(mut this: Self, m: impl FnOnce(&mut T) -> &mut U) -> BorrowMut<'a, U> {
        // SAFETY: This is safe per construction.
        unsafe {
            BorrowMut {
                data: NonNull::from(m(this.data.as_mut())),
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
    /// let bytes = bytes.borrow_mut::<Bytes>()?;
    ///
    /// let Ok(mut bytes) = BorrowMut::try_map(bytes, |bytes| bytes.get_mut(0..2)) else {
    ///     panic!("Conversion failed");
    /// };
    ///
    /// assert_eq!(&mut bytes[..], &mut [1u8, 2u8][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
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
                data: NonNull::from(data),
                guard: this.guard,
                _marker: PhantomData,
            })
        }
    }
}

impl<T: ?Sized> Deref for BorrowMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: This is correct per construction.
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for BorrowMut<'_, T> {
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
