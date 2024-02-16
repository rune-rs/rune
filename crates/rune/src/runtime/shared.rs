use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::future::Future;
use core::mem::{self, transmute, ManuallyDrop};
use core::ops;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

#[cfg(feature = "alloc")]
use ::rust_alloc::rc::Rc;
#[cfg(feature = "alloc")]
use ::rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::alloc::{self, Box};
use crate::runtime::{Access, AccessError, BorrowMut, BorrowRef, RawAccessGuard, Snapshot};

/// A shared value.
pub(crate) struct Shared<T: ?Sized> {
    inner: ptr::NonNull<SharedBox<T>>,
}

impl<T> Shared<T> {
    /// Construct a new shared value.
    pub(crate) fn new(data: T) -> alloc::Result<Self> {
        let shared = SharedBox {
            access: Access::new(),
            count: Cell::new(1),
            data: data.into(),
        };

        let inner = Box::leak(Box::try_new(shared)?);

        Ok(Self {
            inner: inner.into(),
        })
    }

    /// Test if the value is sharable.
    pub(crate) fn is_readable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.inner.as_ref().access.is_shared() }
    }

    /// Test if the value is exclusively accessible.
    pub(crate) fn is_writable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.inner.as_ref().access.is_exclusive() }
    }

    /// Get access snapshot of shared value.
    pub(crate) fn snapshot(&self) -> Snapshot {
        unsafe { self.inner.as_ref().access.snapshot() }
    }

    /// Take the interior value, if we have exlusive access to it and there
    /// are no other live exlusive or shared references.
    ///
    /// A value that has been taken can no longer be accessed.
    pub(crate) fn take(self) -> Result<T, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();

            // NB: don't drop guard to avoid yielding access back.
            // This will prevent the value from being dropped in the shared
            // destructor and future illegal access of any kind.
            let _ = ManuallyDrop::new(inner.access.take()?);

            // Read the pointer out without dropping the inner structure.
            // The data field will be invalid at this point, which should be
            // flagged through a `taken` access flag.
            //
            // Future access is forever prevented since we never release
            // the access (see above).
            Ok(ptr::read(inner.data.get()))
        }
    }

    /// Get a reference to the interior value while checking for shared access
    /// that holds onto a reference count of the inner value.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub(crate) fn into_ref(self) -> Result<Ref<T>, AccessError> {
        // NB: we default to a "safer" mode with `AccessKind::Owned`, where
        // references cannot be converted to an `Mut<T>` in order to avoid
        // a potential soundness panic.
        self.internal_into_ref()
    }

    /// Internal implementation of into_ref.
    pub(crate) fn internal_into_ref(self) -> Result<Ref<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let guard = self.inner.as_ref().access.shared()?.into_raw();

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(Ref {
                data: ptr::NonNull::new_unchecked(this.inner.as_ref().data.get()),
                guard: Some(guard),
                inner: RawDrop::decrement_shared_box(this.inner),
            })
        }
    }

    /// Get a reference to the interior value while checking for exclusive
    /// access that holds onto a reference count of the inner value.
    ///
    /// This prevents other exclusive and shared accesses from being performed
    /// while the guard returned from this function is live.
    pub(crate) fn into_mut(self) -> Result<Mut<T>, AccessError> {
        // NB: we default to a "safer" mode with `AccessKind::Owned`, where
        // references cannot be converted to an `Mut<T>` in order to avoid
        // a potential soundness panic.
        self.internal_into_mut()
    }

    /// Internal implementation of into_mut.
    pub(crate) fn internal_into_mut(self) -> Result<Mut<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let guard = self.inner.as_ref().access.exclusive()?.into_raw();

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(Mut {
                data: ptr::NonNull::new_unchecked(this.inner.as_ref().data.get()),
                guard: Some(guard),
                inner: RawDrop::decrement_shared_box(this.inner),
            })
        }
    }

    /// Deconstruct the shader value into a guard and shared box.
    ///
    /// # Safety
    ///
    /// The content of the shared value will be forcibly destructed once the
    /// returned guard is dropped, use of the shared value after this point will
    /// lead to undefined behavior.
    pub(crate) unsafe fn into_drop_guard(self) -> (Self, SharedPointerGuard) {
        // Increment the reference count by one, to prevent it from every being
        // dropped.
        SharedBox::inc(self.inner.as_ptr());

        let guard = SharedPointerGuard {
            _inner: RawDrop::take_shared_box(self.inner),
        };

        (self, guard)
    }
}

impl<T: ?Sized> Shared<T> {
    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub(crate) fn borrow_ref(&self) -> Result<BorrowRef<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.shared()?;
            mem::forget(guard);
            Ok(BorrowRef::new(&*inner.data.get(), &inner.access))
        }
    }

    /// Get a reference to the interior value while checking for exclusive access.
    ///
    /// This prevents other shared or exclusive accesses from being performed
    /// while the guard returned from this function is live.
    pub(crate) fn borrow_mut(&self) -> Result<BorrowMut<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.exclusive()?;
            mem::forget(guard);
            Ok(BorrowMut::new(&mut *inner.data.get(), &inner.access))
        }
    }
}

impl<T: ?Sized> TryClone for Shared<T> {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(self.clone())
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        unsafe {
            SharedBox::inc(self.inner.as_ptr());
        }

        Self { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Shared<T> {
    fn drop(&mut self) {
        unsafe {
            SharedBox::dec(self.inner.as_ptr());
        }
    }
}

impl<T: ?Sized> fmt::Debug for Shared<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: by virtue of holding onto a shared we can safely access
        // `inner` because it must outlive any `Shared` instances.
        unsafe {
            let inner = self.inner.as_ref();

            if !inner.access.is_shared() {
                write!(fmt, "*not accessible*")
            } else {
                write!(fmt, "{:?}", &&*inner.data.get())
            }
        }
    }
}

/// The boxed internals of [Shared].
#[repr(C)]
struct SharedBox<T: ?Sized> {
    /// The access of the shared data.
    access: Access,
    /// The number of strong references to the shared data.
    count: Cell<usize>,
    /// The value being held. Guarded by the `access` field to determine if it
    /// can be access shared or exclusively.
    data: UnsafeCell<T>,
}

impl<T: ?Sized> SharedBox<T> {
    /// Increment the reference count of the inner value.
    unsafe fn inc(this: *const Self) {
        let count = (*this).count.get();

        if count == 0 || count == usize::MAX {
            crate::alloc::abort();
        }

        (*this).count.set(count + 1);
    }

    /// Decrement the reference count in inner, and free the underlying data if
    /// it has reached zero.
    ///
    /// # Safety
    ///
    /// ProtocolCaller needs to ensure that `this` is a valid pointer.
    unsafe fn dec(this: *mut Self) -> bool {
        let count = (*this).count.get();

        if count == 0 {
            crate::alloc::abort();
        }

        let count = count - 1;
        (*this).count.set(count);

        if count != 0 {
            return false;
        }

        let this = Box::from_raw_in(this, rune_alloc::alloc::Global);

        if this.access.is_taken() {
            // NB: This prevents the inner `T` from being dropped in case it
            // has already been taken (as indicated by `is_taken`).
            //
            // If it has been taken, the shared box contains invalid memory.
            drop(transmute::<_, Box<SharedBox<ManuallyDrop<T>>>>(this));
        } else {
            // NB: At the point of the final drop, no on else should be using
            // this.
            debug_assert!(
                this.access.is_exclusive(),
                "expected exclusive, but was: {:?}",
                this.access
            );
        }

        true
    }
}

type DropFn = unsafe fn(*const ());

struct RawDrop {
    data: *const (),
    drop_fn: DropFn,
}

impl RawDrop {
    /// Construct an empty raw drop function.
    const fn empty() -> Self {
        fn drop_fn(_: *const ()) {}

        Self {
            data: ptr::null(),
            drop_fn,
        }
    }

    /// Construct from an atomically reference-counted `Arc<T>`.
    ///
    /// The argument must have been produced using `Arc::into_raw`.
    #[cfg(feature = "alloc")]
    unsafe fn from_rc<T>(data: *const T) -> Self {
        unsafe fn drop_fn<T>(data: *const ()) {
            let _ = Rc::from_raw(data as *const T);
        }

        Self {
            data: data as *const (),
            drop_fn: drop_fn::<T>,
        }
    }

    /// Construct from an atomically reference-counted `Arc<T>`.
    ///
    /// The argument must have been produced using `Arc::into_raw`.
    #[cfg(feature = "alloc")]
    unsafe fn from_arc<T>(data: *const T) -> Self {
        unsafe fn drop_fn<T>(data: *const ()) {
            let _ = Arc::from_raw(data as *const T);
        }

        Self {
            data: data as *const (),
            drop_fn: drop_fn::<T>,
        }
    }

    /// Construct a raw drop that will decrement the shared box when dropped.
    ///
    /// # Safety
    ///
    /// Should only be constructed over a pointer that is lively owned.
    fn decrement_shared_box<T>(inner: ptr::NonNull<SharedBox<T>>) -> Self {
        unsafe fn drop_fn_impl<T>(data: *const ()) {
            let shared = data as *mut () as *mut SharedBox<T>;
            SharedBox::dec(shared);
        }

        Self {
            data: inner.as_ptr() as *const (),
            drop_fn: drop_fn_impl::<T>,
        }
    }

    /// Construct a raw drop that will take the shared box as it's being
    /// dropped.
    ///
    /// # Safety
    ///
    /// Should only be constructed over a pointer that is lively owned.
    fn take_shared_box<T>(inner: ptr::NonNull<SharedBox<T>>) -> Self {
        unsafe fn drop_fn_impl<T>(data: *const ()) {
            let shared = data as *mut () as *mut SharedBox<T>;

            // Mark the shared box for exclusive access.
            let _ = ManuallyDrop::new(
                (*shared)
                    .access
                    .take()
                    .expect("raw pointers must not be shared"),
            );

            // Free the inner `Any` structure, and since we have marked the
            // Shared as taken, this will prevent anyone else from doing it.
            drop(ptr::read((*shared).data.get()));

            SharedBox::dec(shared);
        }

        Self {
            data: inner.as_ptr() as *const (),
            drop_fn: drop_fn_impl::<T>,
        }
    }
}

impl Drop for RawDrop {
    fn drop(&mut self) {
        // Safety: type and referential safety is guaranteed at construction
        // time, since all constructors are unsafe.
        unsafe {
            (self.drop_fn)(self.data);
        }
    }
}

/// A strong reference to the given type.
pub struct Ref<T: ?Sized> {
    data: ptr::NonNull<T>,
    // Safety: it is important that the guard is dropped before `RawDrop`, since
    // `RawDrop` might deallocate the `Access` instance the guard is referring
    // to. This is guaranteed by: https://github.com/rust-lang/rfcs/pull/1857
    guard: Option<RawAccessGuard>,
    // We need to keep track of the original value so that we can deal with what
    // it means to drop a reference to it.
    inner: RawDrop,
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
        let data = Rc::into_raw(value);

        Ref {
            data: unsafe { ptr::NonNull::new_unchecked(data as *mut _) },
            guard: None,
            inner: unsafe { RawDrop::from_rc(data) },
        }
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
        let data = Arc::into_raw(value);

        Ref {
            data: unsafe { ptr::NonNull::new_unchecked(data as *mut _) },
            guard: None,
            inner: unsafe { RawDrop::from_arc(data) },
        }
    }
}

impl<T: ?Sized> Ref<T> {
    /// Construct a static reference.
    pub const fn from_static(value: &'static T) -> Ref<T> {
        Ref {
            data: unsafe { ptr::NonNull::new_unchecked(value as *const _ as *mut _) },
            guard: None,
            inner: RawDrop::empty(),
        }
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
        let Self {
            data, guard, inner, ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        let data = f(unsafe { data.as_ref() });

        Ref {
            data: data.into(),
            guard,
            inner,
        }
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
        let Self {
            data, guard, inner, ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.

        unsafe {
            let Some(data) = f(data.as_ref()) else {
                return Err(Ref { data, guard, inner });
            };

            Ok(Ref {
                data: data.into(),
                guard,
                inner,
            })
        }
    }

    #[inline]
    pub(crate) fn result_map<U: ?Sized, F, E>(this: Self, f: F) -> Result<Ref<U>, (E, Ref<T>)>
    where
        F: FnOnce(&T) -> Result<&U, E>,
    {
        let Self {
            data, guard, inner, ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        unsafe {
            let data = match f(data.as_ref()) {
                Ok(data) => data,
                Err(e) => return Err((e, Ref { data, guard, inner })),
            };

            Ok(Ref {
                data: data.into(),
                guard,
                inner,
            })
        }
    }

    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    pub fn into_raw(this: Self) -> (ptr::NonNull<T>, RawRef) {
        let guard = RawRef {
            _guard: this.guard,
            _inner: this.inner,
        };

        (this.data, guard)
    }

    /// Convert a raw reference and guard into a regular reference.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that the raw reference is
    /// associated with the specific pointer.
    pub unsafe fn from_raw(data: ptr::NonNull<T>, guard: RawRef) -> Self {
        Self {
            data,
            guard: guard._guard,
            inner: guard._inner,
        }
    }
}

impl<T: ?Sized> AsRef<T> for Ref<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> ops::Deref for Ref<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety: An owned ref holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned ref.
        unsafe { self.data.as_ref() }
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

/// A raw guard to a [Ref].
pub struct RawRef {
    _guard: Option<RawAccessGuard>,
    _inner: RawDrop,
}

/// A strong mutable reference to the given type.
pub struct Mut<T: ?Sized> {
    data: ptr::NonNull<T>,
    // Safety: it is important that the guard is dropped before `RawDrop`, since
    // `RawDrop` might deallocate the `Access` instance the guard is referring
    // to. This is guaranteed by: https://github.com/rust-lang/rfcs/pull/1857
    guard: Option<RawAccessGuard>,
    inner: RawDrop,
}

impl<T: ?Sized> Mut<T> {
    /// Construct a static mutable reference.
    pub fn from_static(value: &'static mut T) -> Mut<T> {
        Mut {
            data: unsafe { ptr::NonNull::new_unchecked(value as *mut _) },
            guard: None,
            inner: RawDrop::empty(),
        }
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
            mut data,
            guard,
            inner,
            ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        let data = f(unsafe { data.as_mut() });

        Mut {
            data: data.into(),
            guard,
            inner,
        }
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
            mut data,
            guard,
            inner,
            ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        unsafe {
            let Some(data) = f(data.as_mut()) else {
                return Err(Mut { data, guard, inner });
            };

            Ok(Mut {
                data: data.into(),
                guard,
                inner,
            })
        }
    }

    #[inline]
    pub(crate) fn result_map<U: ?Sized, F, E>(this: Self, f: F) -> Result<Mut<U>, (E, Mut<T>)>
    where
        F: FnOnce(&mut T) -> Result<&mut U, E>,
    {
        let Self {
            mut data,
            guard,
            inner,
            ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        unsafe {
            let data = match f(data.as_mut()) {
                Ok(data) => data,
                Err(error) => return Err((error, Mut { data, guard, inner })),
            };

            Ok(Mut {
                data: data.into(),
                guard,
                inner,
            })
        }
    }

    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    pub fn into_raw(this: Self) -> (ptr::NonNull<T>, RawMut) {
        let guard = RawMut {
            _guard: this.guard,
            _inner: this.inner,
        };

        (this.data, guard)
    }

    /// Convert a raw mutable reference and guard into a regular mutable
    /// reference.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that the raw mutable reference is
    /// associated with the specific pointer.
    pub unsafe fn from_raw(data: ptr::NonNull<T>, guard: RawMut) -> Self {
        Self {
            data,
            guard: guard._guard,
            inner: guard._inner,
        }
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

impl<T: ?Sized> ops::Deref for Mut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: An owned mut holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned mut.
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> ops::DerefMut for Mut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: An owned mut holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned mut.
        unsafe { self.data.as_mut() }
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

/// A raw guard to a [Ref].
pub struct RawMut {
    _guard: Option<RawAccessGuard>,
    _inner: RawDrop,
}

/// A drop guard for a shared value.
///
/// Once this is dropped, the shared value will be destructed.
pub struct SharedPointerGuard {
    _inner: RawDrop,
}
