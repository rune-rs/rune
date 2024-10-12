use core::fmt;
use core::future::Future;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::NonNull;
use core::task::{Context, Poll};

#[cfg(feature = "alloc")]
use ::rust_alloc::rc::Rc;
#[cfg(feature = "alloc")]
use ::rust_alloc::sync::Arc;

pub(super) struct RefVtable {
    pub(super) drop: DropFn,
}

type DropFn = unsafe fn(NonNull<()>);

/// A strong reference to the given type.
pub struct Ref<T: ?Sized> {
    value: NonNull<T>,
    guard: RawAnyGuard,
}

#[cfg(feature = "alloc")]
impl<T> From<Rc<T>> for Ref<T> {
    /// Construct from an atomically reference-counted value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    /// use rune::runtime::Ref;
    ///
    /// let value: Ref<String> = Ref::from(Rc::new(String::from("hello world")));
    /// assert_eq!(value.as_ref(), "hello world");
    /// ```
    fn from(value: Rc<T>) -> Ref<T> {
        unsafe fn drop_fn<T>(data: NonNull<()>) {
            let _ = Rc::from_raw(data.cast::<T>().as_ptr().cast_const());
        }

        let value = Rc::into_raw(value);
        let value = unsafe { NonNull::new_unchecked(value as *mut _) };

        let guard = RawAnyGuard::new(value.cast(), &RefVtable { drop: drop_fn::<T> });

        Ref::new(value, guard)
    }
}

#[cfg(feature = "alloc")]
impl<T> From<Arc<T>> for Ref<T> {
    /// Construct from an atomically reference-counted value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rune::runtime::Ref;
    ///
    /// let value: Ref<String> = Ref::from(Arc::new(String::from("hello world")));
    /// assert_eq!(value.as_ref(), "hello world");
    /// ```
    fn from(value: Arc<T>) -> Ref<T> {
        unsafe fn drop_fn<T>(data: NonNull<()>) {
            let _ = Arc::from_raw(data.cast::<T>().as_ptr().cast_const());
        }

        let value = Arc::into_raw(value);
        let value = unsafe { NonNull::new_unchecked(value as *mut _) };

        let guard = RawAnyGuard::new(value.cast(), &RefVtable { drop: drop_fn::<T> });

        Ref::new(value, guard)
    }
}

impl<T: ?Sized> Ref<T> {
    pub(super) const fn new(value: NonNull<T>, guard: RawAnyGuard) -> Self {
        Self { value, guard }
    }

    /// Construct a static reference.
    pub const fn from_static(value: &'static T) -> Ref<T> {
        let value = unsafe { NonNull::new_unchecked((value as *const T).cast_mut()) };
        let guard = RawAnyGuard::new(NonNull::dangling(), &RefVtable { drop: |_| {} });
        Self::new(value, guard)
    }

    /// Map the interior reference of an owned mutable value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Bytes, Ref};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes: Ref<Bytes> = rune::from_value(bytes)?;
    /// let value: Ref<[u8]> = Ref::map(bytes, |vec| &vec[0..2]);
    ///
    /// assert_eq!(&*value, &[1, 2][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn map<U: ?Sized, F>(this: Self, f: F) -> Ref<U>
    where
        F: FnOnce(&T) -> &U,
    {
        let Self { value, guard } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        let value = f(unsafe { value.as_ref() });

        Ref::new(value.into(), guard)
    }

    /// Try to map the reference to a projection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Bytes, Ref};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes: Ref<Bytes> = rune::from_value(bytes)?;
    ///
    /// let Ok(value) = Ref::try_map(bytes, |bytes| bytes.get(0..2)) else {
    ///     panic!("Conversion failed");
    /// };
    ///
    /// assert_eq!(&value[..], &[1, 2][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn try_map<U: ?Sized, F>(this: Self, f: F) -> Result<Ref<U>, Ref<T>>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        let Self { value, guard } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.

        unsafe {
            let Some(value) = f(value.as_ref()) else {
                return Err(Ref::new(value, guard));
            };

            Ok(Ref::new(value.into(), guard))
        }
    }

    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    pub fn into_raw(this: Self) -> (NonNull<T>, RawAnyGuard) {
        (this.value, this.guard)
    }

    /// Convert a raw reference and guard into a regular reference.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that the raw reference is
    /// associated with the specific pointer.
    pub unsafe fn from_raw(value: NonNull<T>, guard: RawAnyGuard) -> Self {
        Self { value, guard }
    }
}

impl<T: ?Sized> AsRef<T> for Ref<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> Deref for Ref<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety: An owned ref holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned ref.
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized> fmt::Debug for Ref<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

/// A strong mutable reference to the given type.
pub struct Mut<T: ?Sized> {
    value: NonNull<T>,
    guard: RawAnyGuard,
}

impl<T: ?Sized> Mut<T> {
    pub(super) const fn new(value: NonNull<T>, guard: RawAnyGuard) -> Self {
        Self { value, guard }
    }

    /// Construct a static mutable reference.
    pub fn from_static(value: &'static mut T) -> Mut<T> {
        let value = unsafe { NonNull::new_unchecked((value as *const T).cast_mut()) };
        let guard = RawAnyGuard::new(NonNull::dangling(), &RefVtable { drop: |_| {} });
        Self::new(value, guard)
    }

    /// Map the interior reference of an owned mutable value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Bytes, Mut};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes: Mut<Bytes> = rune::from_value(bytes)?;
    /// let value: Mut<[u8]> = Mut::map(bytes, |bytes| &mut bytes[0..2]);
    ///
    /// assert_eq!(&*value, &mut [1, 2][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn map<U: ?Sized, F>(this: Self, f: F) -> Mut<U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        let Self {
            mut value, guard, ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        let value = f(unsafe { value.as_mut() });

        Mut::new(value.into(), guard)
    }

    /// Try to map the mutable reference to a projection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Bytes, Mut};
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = rune::to_value(Bytes::from_vec(try_vec![1, 2, 3, 4]))?;
    /// let bytes: Mut<Bytes> = rune::from_value(bytes)?;
    ///
    /// let Ok(mut value) = Mut::try_map(bytes, |bytes| bytes.get_mut(0..2)) else {
    ///     panic!("Conversion failed");
    /// };
    ///
    /// assert_eq!(&mut value[..], &mut [1, 2][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn try_map<U: ?Sized, F>(this: Self, f: F) -> Result<Mut<U>, Mut<T>>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        let Self {
            mut value, guard, ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        unsafe {
            let Some(value) = f(value.as_mut()) else {
                return Err(Mut::new(value, guard));
            };

            Ok(Mut::new(value.into(), guard))
        }
    }

    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    pub fn into_raw(this: Self) -> (NonNull<T>, RawAnyGuard) {
        (this.value, this.guard)
    }

    /// Convert a raw mutable reference and guard into a regular mutable
    /// reference.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that the raw mutable reference is
    /// associated with the specific pointer.
    pub unsafe fn from_raw(value: NonNull<T>, guard: RawAnyGuard) -> Self {
        Self { value, guard }
    }
}

impl<T: ?Sized> AsRef<T> for Mut<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> AsMut<T> for Mut<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> Deref for Mut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: An owned mut holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned mut.
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Mut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: An owned mut holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned mut.
        unsafe { self.value.as_mut() }
    }
}

impl<T: ?Sized> fmt::Debug for Mut<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

impl<F> Future for Mut<F>
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

/// A raw guard for a [`Ref`] or a [`Mut`] that has been converted into its raw
/// components through [`Ref::into_raw`] or [`Mut::into_raw`].
pub struct RawAnyGuard {
    data: NonNull<()>,
    vtable: &'static RefVtable,
}

impl RawAnyGuard {
    pub(super) const fn new(data: NonNull<()>, vtable: &'static RefVtable) -> Self {
        Self { data, vtable }
    }
}

impl Drop for RawAnyGuard {
    fn drop(&mut self) {
        // Safety: type and referential safety is guaranteed at construction
        // time, since all constructors are unsafe.
        unsafe {
            (self.vtable.drop)(self.data);
        }
    }
}
