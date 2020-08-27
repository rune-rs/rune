use crate::access::{Access, AccessError, Mut, RawMutGuard, RawRefGuard, Ref};
use crate::any::Any;
use crate::shared_ptr::SharedPtr;
use std::any;
use std::cell::{Cell, UnsafeCell};
use std::fmt;
use std::marker;
use std::mem::ManuallyDrop;
use std::ops;
use std::process;
use std::ptr;

/// A shared value.
pub struct Shared<T: ?Sized> {
    inner: ptr::NonNull<SharedBox<T>>,
}

impl<T> Shared<T> {
    /// Construct a new shared value.
    pub fn new(data: T) -> Self {
        let inner = Box::leak(Box::new(SharedBox {
            access: Access::new(),
            count: Cell::new(1),
            data: data.into(),
        }));

        Self {
            inner: inner.into(),
        }
    }

    /// Take the interior value, if we have exlusive access to it and there
    /// are no other live exlusive or shared references.
    ///
    /// A value that has been taken can no longer be accessed.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 });
    /// let b = a.clone();
    ///
    /// {
    ///     let mut a = a.get_mut().unwrap();
    ///     // NB: this is prevented since we have a live reference.
    ///     assert!(b.take().is_err());
    ///     a.counter += 1;
    /// }
    ///
    /// let a = a.take().unwrap();
    /// assert_eq!(a.counter, 1);
    /// ```
    pub fn take(self) -> Result<T, AccessError> {
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
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 });
    /// let b = a.clone();
    ///
    /// b.get_mut().unwrap().counter += 1;
    ///
    /// {
    ///     // Consumes `a`.
    ///     let mut a = a.strong_ref().unwrap();
    ///     assert_eq!(a.counter, 1);
    ///     assert!(b.get_mut().is_err());
    /// }
    ///
    /// let mut b = b.get_mut().unwrap();
    /// b.counter += 1;
    /// assert_eq!(b.counter, 2);
    /// ```
    pub fn strong_ref(self) -> Result<StrongRef<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let guard = self.inner.as_ref().access.shared()?;

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(StrongRef {
                data: this.inner.as_ref().data.get(),
                guard,
                inner: RawSharedBox::from_inner(this.inner),
                _marker: marker::PhantomData,
            })
        }
    }

    /// Get a reference to the interior value while checking for exclusive
    /// access that holds onto a reference count of the inner value.
    ///
    /// This prevents other exclusive and shared accesses from being performed
    /// while the guard returned from this function is live.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 });
    /// let b = a.clone();
    ///
    /// {
    ///     // Consumes `a`.
    ///     let mut a = a.strong_mut().unwrap();
    ///     a.counter += 1;
    ///
    ///     assert!(b.get_ref().is_err());
    /// }
    ///
    /// assert_eq!(b.get_ref().unwrap().counter, 1);
    /// ```
    pub fn strong_mut(self) -> Result<StrongMut<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let guard = self.inner.as_ref().access.exclusive()?;

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(StrongMut {
                data: this.inner.as_ref().data.get(),
                guard,
                inner: RawSharedBox::from_inner(this.inner),
                _marker: marker::PhantomData,
            })
        }
    }
}

impl<T: ?Sized> Shared<T> {
    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 });
    ///
    /// a.get_mut().unwrap().counter += 1;
    ///
    /// {
    ///     let mut a_ref = a.get_ref().unwrap();
    ///     assert_eq!(a_ref.counter, 1);
    ///     assert!(a.get_mut().is_err());
    ///     assert!(a.get_ref().is_ok());
    /// }
    ///
    /// let mut a = a.get_mut().unwrap();
    /// a.counter += 1;
    /// assert_eq!(a.counter, 2);
    /// ```
    pub fn get_ref(&self) -> Result<Ref<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.shared()?;
            Ok(Ref::from_raw(inner.data.get(), guard))
        }
    }

    /// Get a reference to the interior value while checking for exclusive access.
    ///
    /// This prevents other shared or exclusive accesses from being performed
    /// while the guard returned from this function is live.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 });
    ///
    /// {
    ///     let mut a_mut = a.get_mut().unwrap();
    ///     a_mut.counter += 1;
    ///     assert_eq!(a_mut.counter, 1);
    ///     assert!(a.get_ref().is_err());
    /// }
    ///
    /// let a = a.get_ref().unwrap();
    /// assert_eq!(a.counter, 1);
    /// ```
    pub fn get_mut(&self) -> Result<Mut<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.exclusive()?;
            Ok(Mut::from_raw(inner.data.get(), guard))
        }
    }
}

impl Shared<Any> {
    /// Take the interior value, if we have exlusive access to it and there
    /// exist no other references.
    pub fn downcast_take<T>(self) -> Result<T, AccessError>
    where
        T: any::Any,
    {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();

            // NB: don't drop guard to avoid yielding access back.
            // This will prevent the value from being dropped in the shared
            // destructor and future illegal access of any kind.
            let guard = ManuallyDrop::new(inner.access.take()?);

            // Read the pointer out without dropping the inner structure.
            // Note that the data field will after this point be invalid.
            //
            // Future access is forever prevented since we never release
            // exclusive access (see above).
            let any = ptr::read(inner.data.get());

            match any.take_mut_ptr(any::TypeId::of::<T>()) {
                Ok(value) => Ok(*Box::from_raw(value as *mut T)),
                Err(any) => {
                    let actual = any.type_name();

                    // Type coercion failed, so reconstruct the state of the
                    // Shared container.

                    // Drop the guard to release exclusive access.
                    drop(ManuallyDrop::into_inner(guard));

                    // NB: write the potentially modified value back.
                    // It hasn't been modified, but there has been a period of
                    // time now that the value hasn't been valid for.
                    ptr::write(inner.data.get(), any);

                    Err(AccessError::UnexpectedType {
                        actual,
                        expected: any::type_name::<T>(),
                    })
                }
            }
        }
    }

    /// Get a shared value and downcast.
    pub fn downcast_ref<T>(&self) -> Result<Ref<'_, T>, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.shared()?;

            let data = match (*inner.data.get()).as_ptr(any::TypeId::of::<T>()) {
                Some(data) => data,
                None => {
                    return Err(AccessError::UnexpectedType {
                        expected: any::type_name::<T>(),
                        actual: (*inner.data.get()).type_name(),
                    });
                }
            };

            Ok(Ref::from_raw(data as *const T, guard))
        }
    }

    /// Get a shared value and downcast.
    pub fn downcast_strong_ref<T>(self) -> Result<StrongRef<T>, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let (data, guard) = {
                let inner = self.inner.as_ref();
                let guard = inner.access.shared()?;

                match (*inner.data.get()).as_ptr(any::TypeId::of::<T>()) {
                    Some(data) => (data, guard),
                    None => {
                        return Err(AccessError::UnexpectedType {
                            expected: any::type_name::<T>(),
                            actual: (*inner.data.get()).type_name(),
                        });
                    }
                }
            };

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(StrongRef {
                data: data as *const T,
                guard,
                inner: RawSharedBox::from_inner(this.inner),
                _marker: marker::PhantomData,
            })
        }
    }

    /// Get a exclusive value and downcast.
    pub fn downcast_mut<T>(&self) -> Result<Mut<'_, T>, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.exclusive()?;

            let data = match (*inner.data.get()).as_mut_ptr(any::TypeId::of::<T>()) {
                Some(data) => data,
                None => {
                    return Err(AccessError::UnexpectedType {
                        expected: any::type_name::<T>(),
                        actual: (*inner.data.get()).type_name(),
                    });
                }
            };

            Ok(Mut::from_raw(data as *mut T, guard))
        }
    }

    /// Get a shared value and downcast.
    pub fn downcast_strong_mut<T>(self) -> Result<StrongMut<T>, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let (data, guard) = {
                let inner = self.inner.as_ref();
                let guard = inner.access.exclusive()?;

                match (*inner.data.get()).as_mut_ptr(any::TypeId::of::<T>()) {
                    Some(data) => (data, guard),
                    None => {
                        return Err(AccessError::UnexpectedType {
                            expected: any::type_name::<T>(),
                            actual: (*inner.data.get()).type_name(),
                        });
                    }
                }
            };

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(StrongMut {
                data: data as *mut T,
                guard,
                inner: RawSharedBox::from_inner(this.inner),
                _marker: marker::PhantomData,
            })
        }
    }
}

impl Shared<SharedPtr> {
    /// Get a shared value and downcast.
    pub fn downcast_ref<T>(&self) -> Result<Ref<'_, T>, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.shared()?;
            let data = (*inner.data.get()).downcast_ref::<T>()?;
            Ok(Ref::from_raw(data, guard))
        }
    }

    /// Get a exclusive value and downcast.
    pub fn downcast_mut<T>(&self) -> Result<Mut<'_, T>, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.exclusive()?;
            let data = (*inner.data.get()).downcast_mut::<T>()?;
            Ok(Mut::from_raw(data, guard))
        }
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
    T: any::Any + fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let inner = self.inner.as_ref();
            let mut debug = fmt.debug_struct("Shared");

            debug.field("access", &inner.access);
            debug.field("count", &inner.count.get());

            if !inner.access.is_shared() {
                debug.field("data", &any::type_name::<T>());
            } else {
                debug.field("data", &&*inner.data.get());
            }

            debug.finish()
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

        if count == 0 || count == usize::max_value() {
            process::abort();
        }

        let count = count + 1;
        (*this).count.set(count);
    }

    /// Decrement the reference count in inner, and free the underlying data if
    /// it has reached zero.
    ///
    /// # Safety
    ///
    /// Caller needs to ensure that `this` is a valid pointer.
    unsafe fn dec(this: *mut Self) {
        let count = (*this).count.get();

        if count == 0 {
            process::abort();
        }

        let count = count - 1;
        (*this).count.set(count);

        if count != 0 {
            return;
        }

        if (*this).access.is_taken() {
            // NB: This prevents the inner `T` from being dropped in case it
            // has already been taken (as indicated by `is_taken`).
            //
            // If it has been taken, the shared box contains invalid memory.
            let _ = std::mem::transmute::<_, Box<SharedBox<ManuallyDrop<T>>>>(Box::from_raw(this));
        } else {
            // NB: At the point of the final drop, no on else should be using
            // this.
            debug_assert!((*this).access.is_exclusive());
            let _ = Box::from_raw(this);
        }
    }
}

type DropFn = unsafe fn(*const ());

struct RawSharedBox {
    data: *const (),
    drop_fn: DropFn,
}

impl RawSharedBox {
    /// Construct a raw inner from an existing inner value.
    ///
    /// # Safety
    ///
    /// Should only be constructed over a pointer that is lively owned.
    fn from_inner<T>(inner: ptr::NonNull<SharedBox<T>>) -> Self {
        return Self {
            data: inner.as_ptr() as *const (),
            drop_fn: drop_fn_impl::<T>,
        };

        unsafe fn drop_fn_impl<T>(data: *const ()) {
            SharedBox::dec(data as *mut () as *mut SharedBox<T>);
        }
    }
}

impl Drop for RawSharedBox {
    fn drop(&mut self) {
        // Safety: type and referential safety is guaranteed at construction
        // time, since all constructors are unsafe.
        unsafe {
            (self.drop_fn)(self.data);
        }
    }
}

/// A strong reference to the given type.
pub struct StrongRef<T: ?Sized> {
    data: *const T,
    guard: RawRefGuard,
    inner: RawSharedBox,
    _marker: marker::PhantomData<T>,
}

impl<T: ?Sized> StrongRef<T> {
    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    ///
    /// The returned pointer also must not outlive the VM that produced.
    /// Nor a call to clear the VM using [clear], since this will free up the
    /// data being referenced.
    ///
    /// [clear]: [crate::Vm::clear]
    pub fn into_raw(this: Self) -> (*const T, RawStrongRefGuard) {
        let guard = RawStrongRefGuard {
            _guard: this.guard,
            _inner: this.inner,
        };

        (this.data, guard)
    }
}

impl<T: ?Sized> ops::Deref for StrongRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: ?Sized> fmt::Debug for StrongRef<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

/// A raw guard to a [StrongRef].
pub struct RawStrongRefGuard {
    _guard: RawRefGuard,
    _inner: RawSharedBox,
}

/// A strong mutable reference to the given type.
pub struct StrongMut<T: ?Sized> {
    data: *mut T,
    guard: RawMutGuard,
    inner: RawSharedBox,
    _marker: marker::PhantomData<T>,
}

impl<T: ?Sized> StrongMut<T> {
    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    ///
    /// The returned pointer also must not outlive the VM that produced.
    /// Nor a call to clear the VM using [clear], since this will free up the
    /// data being referenced.
    ///
    /// [clear]: [crate::Vm::clear]
    pub fn into_raw(this: Self) -> (*mut T, RawStrongMutGuard) {
        let guard = RawStrongMutGuard {
            _guard: this.guard,
            _inner: this.inner,
        };

        (this.data, guard)
    }
}

impl<T: ?Sized> ops::Deref for StrongMut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T: ?Sized> ops::DerefMut for StrongMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<T: ?Sized> fmt::Debug for StrongMut<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

/// A raw guard to a [StrongRef].
pub struct RawStrongMutGuard {
    _guard: RawMutGuard,
    _inner: RawSharedBox,
}

#[cfg(test)]
mod tests {
    use crate::{Any, Shared};

    #[derive(Debug)]
    struct Foo(isize);

    #[test]
    fn test_leak_references() {
        let thing = Shared::new(Any::new(Foo(0)));
        let _ = thing.take().unwrap();
    }
}
