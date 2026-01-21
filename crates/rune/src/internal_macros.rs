/// Helper to borrow out a [ResolveContext][crate::parse::ResolveContext].
macro_rules! __resolve_context {
    ($q:expr) => {
        $crate::parse::ResolveContext {
            sources: $q.sources,
            storage: $q.storage,
            scratch: &$q.inner.scratch,
        }
    };
}

pub(crate) use __resolve_context as resolve_context;

/// Call the given macro with repeated type arguments and counts.
macro_rules! repeat_macro {
    ($macro:ident) => {
        $macro!(0);
        $macro!(1, A a 0);
        $macro!(2, A a 0, B b 1);
        $macro!(3, A a 0, B b 1, C c 2);
        $macro!(4, A a 0, B b 1, C c 2, D d 3);
        #[cfg(not(test))]
        $macro!(5, A a 0, B b 1, C c 2, D d 3, E e 4);
        #[cfg(not(test))]
        $macro!(6, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5);
        #[cfg(not(test))]
        $macro!(7, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6);
        #[cfg(not(test))]
        $macro!(8, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7);
        #[cfg(not(test))]
        $macro!(9, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8);
        #[cfg(not(test))]
        $macro!(10, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9);
        #[cfg(not(test))]
        $macro!(11, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10);
        #[cfg(not(test))]
        $macro!(12, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11);
        #[cfg(not(test))]
        $macro!(13, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12);
        #[cfg(not(test))]
        $macro!(14, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12, N n 13);
        #[cfg(not(test))]
        $macro!(15, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12, N n 13, O o 14);
        #[cfg(not(test))]
        $macro!(16, A a 0, B b 1, C c 2, D d 3, E e 4, F f 5, G g 6, H h 7, I i 8, J j 9, K k 10, L l 11, M m 12, N n 13, O o 14, P p 15);
    };
}

macro_rules! assert_impl {
    ($ty:ty: $first_trait:ident $(+ $rest_trait:ident)*) => {
        #[cfg(test)]
        const _: () = const {
            const fn assert_traits<T>() where T: $first_trait $(+ $rest_trait)* {}
            assert_traits::<$ty>();
        };
    };
}

/// Asynchronous helper to perform the try operation over an asynchronous
/// `Result`.
#[macro_export]
#[doc(hidden)]
macro_rules! __async_vm_try {
    ($expr:expr) => {
        match $expr {
            ::core::result::Result::Ok(value) => value,
            ::core::result::Result::Err(err) => {
                return ::core::task::Poll::Ready(::core::result::Result::Err(
                    ::core::convert::From::from(err),
                ));
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __vm_error {
    ($ty:ty) => {
        impl<T> $crate::runtime::MaybeTypeOf for Result<T, $ty>
        where
            T: $crate::runtime::MaybeTypeOf,
        {
            #[inline]
            fn maybe_type_of() -> $crate::alloc::Result<$crate::compile::meta::DocType> {
                <T as $crate::runtime::MaybeTypeOf>::maybe_type_of()
            }
        }

        impl<T> $crate::runtime::IntoReturn for Result<T, $ty>
        where
            T: $crate::runtime::ToValue,
        {
            #[inline]
            fn into_return(self) -> Result<$crate::runtime::Value, $crate::runtime::VmError> {
                match self {
                    Ok(value) => Ok(value.to_value()?),
                    Err(error) => Err($crate::runtime::VmError::from(error)),
                }
            }
        }
    };
}

macro_rules! __declare_dyn_fn {
    (
        struct $vtable:ident;

        $(#[doc = $doc:literal])*
        $vis:vis struct $name:ident {
            fn call($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty;
        }
    ) => {
        /// The vtable for a function handler.
        struct $vtable {
            call: unsafe fn(ptr: *const () $(, $arg: $ty)*) -> $ret,
            drop: unsafe fn(ptr: *const ()),
            clone: unsafe fn(ptr: *const ()) -> *const (),
        }

        $(#[doc = $doc])*
        $vis struct $name {
            ptr: ::core::ptr::NonNull<()>,
            vtable: &'static $vtable,
        }

        impl $name {
            #[inline]
            pub(crate) fn new<F>(f: F) -> $crate::alloc::Result<Self>
            where
                F: Fn($($ty),*) -> $ret + Send + Sync + 'static,
            {
                use $crate::sync::Arc;
                use $crate::alloc::alloc::Global;

                fn call_impl<F>(
                    ptr: *const (),
                    $($arg: $ty,)*
                ) -> $ret
                where
                    F: Fn($($ty),*) -> $ret + Send + Sync + 'static,
                {
                    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
                    unsafe { (*ptr.cast::<F>())($($arg),*) }
                }

                fn clone_impl<F>(ptr: *const ()) -> *const () {
                    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
                    unsafe {
                        let ptr = ptr.cast::<F>();
                        // Prevent the constructed Arc from being dropped, which would decrease
                        // its reference count.
                        let arc = ::core::mem::ManuallyDrop::new(Arc::<F>::from_raw_in(ptr, Global));
                        let arc = (*arc).clone();
                        let (ptr, Global) = Arc::into_raw_with_allocator(arc);
                        ptr.cast()
                    }
                }

                fn drop_impl<F>(ptr: *const ()) {
                    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
                    unsafe {
                        let ptr = ptr.cast::<F>();
                        drop(Arc::<F, Global>::from_raw_in(ptr, Global));
                    }
                }

                let arc = Arc::try_new(f)?;
                let (ptr, Global) = Arc::into_raw_with_allocator(arc);
                let ptr = unsafe { ::core::ptr::NonNull::new_unchecked(ptr.cast_mut().cast()) };
                let vtable = &$vtable {
                    call: call_impl::<F>,
                    drop: drop_impl::<F>,
                    clone: clone_impl::<F>,
                };

                Ok(Self { ptr, vtable })
            }

            /// Call the function handler through the raw type-erased API.
            #[inline]
            $vis fn call(
                &self,
                $($arg: $ty,)*
            ) -> $ret {
                // SAFETY: The pointer is guaranteed to be valid and the vtable is static.
                unsafe { (self.vtable.call)(self.ptr.as_ptr().cast_const() $(, $arg)*) }
            }
        }

        unsafe impl Send for $name {}
        unsafe impl Sync for $name {}

        impl Drop for $name {
            #[inline]
            fn drop(&mut self) {
                // SAFETY: The pointer is guaranteed to be valid and the vtable is static.
                unsafe { (self.vtable.drop)(self.ptr.as_ptr().cast_const()) }
            }
        }

        impl Clone for $name {
            #[inline]
            fn clone(&self) -> Self {
                // SAFETY: The pointer is valid and the vtable is static.
                let ptr = unsafe {
                    let ptr = self.ptr.as_ptr().cast_const();
                    let ptr = (self.vtable.clone)(ptr);
                    ::core::ptr::NonNull::new_unchecked(ptr.cast_mut())
                };

                Self {
                    ptr,
                    vtable: self.vtable,
                }
            }
        }

        impl $crate::alloc::clone::TryClone for $name {
            #[inline]
            fn try_clone(&self) -> $crate::alloc::Result<Self> {
                Ok(self.clone())
            }
        }

        impl ::core::fmt::Pointer for $name {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Pointer::fmt(&self.ptr, f)
            }
        }
    }
}

macro_rules! __declare_dyn_trait {
    (
        $(#[$($vtable_meta:meta)*])*
        $vtable_vis:vis struct $vtable:ident;

        $(#[$($impl_meta:meta)*])*
        $impl_vis:vis struct $impl:ident;

        $(#[$($trait_meta:meta)*])*
        $vis:vis trait $name:ident {
            $(
                $(#[$($fn_meta:meta)*])*
                fn $fn:ident(&self, $($arg:ident: $ty:ty),* $(,)?) -> $ret:ty;
            )*
        }
    ) => {
        $(#[$($vtable_meta)*])*
        $vtable_vis struct $vtable {
            $($fn: unsafe fn(ptr: *const () $(, $arg: $ty)*) -> $ret,)*
            drop: unsafe fn(ptr: *const ()),
            clone: unsafe fn(ptr: *const ()) -> *const (),
        }

        $(#[$($trait_meta)*])*
        $vis trait $name {
            $(
                $(#[$($fn_meta)*])*
                fn $fn(&self, $($arg: $ty),*) -> $ret;
            )*
        }

        $(#[$($impl_meta)*])*
        $impl_vis struct $impl {
            ptr: ::core::ptr::NonNull<()>,
            vtable: &'static $vtable,
        }

        impl $impl {
            /// Construct a new wrapper for the underlying trait.
            #[inline]
            $impl_vis fn new<F>(f: F) -> $crate::alloc::Result<Self>
            where
                F: $name + Send + Sync + 'static,
            {
                use $crate::sync::Arc;
                use $crate::alloc::alloc::Global;

                $(
                    fn $fn<F>(
                        ptr: *const (),
                        $($arg: $ty,)*
                    ) -> $ret
                    where
                        F: $name + Send + Sync + 'static,
                    {
                        // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
                        unsafe { (*ptr.cast::<F>()).$fn($($arg),*) }
                    }
                )*

                fn clone_impl<F>(ptr: *const ()) -> *const () {
                    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
                    unsafe {
                        let ptr = ptr.cast::<F>();
                        // Prevent the constructed Arc from being dropped, which would decrease
                        // its reference count.
                        let arc = ::core::mem::ManuallyDrop::new(Arc::<F>::from_raw_in(ptr, Global));
                        let arc = (*arc).clone();
                        let (ptr, Global) = Arc::into_raw_with_allocator(arc);
                        ptr.cast()
                    }
                }

                fn drop_impl<F>(ptr: *const ()) {
                    // SAFETY: We've ensured the interior value is a valid pointer to `F` due to construction.
                    unsafe {
                        let ptr = ptr.cast::<F>();
                        drop(Arc::<F, Global>::from_raw_in(ptr, Global));
                    }
                }

                let arc = Arc::try_new(f)?;
                let (ptr, Global) = Arc::into_raw_with_allocator(arc);
                let ptr = unsafe { ::core::ptr::NonNull::new_unchecked(ptr.cast_mut().cast()) };
                let vtable = &$vtable {
                    $($fn: $fn::<F>,)*
                    drop: drop_impl::<F>,
                    clone: clone_impl::<F>,
                };

                Ok(Self { ptr, vtable })
            }

            $(
                $(#[$($fn_meta)*])*
                #[inline]
                $impl_vis fn $fn(
                    &self,
                    $($arg: $ty,)*
                ) -> $ret {
                    // SAFETY: The pointer is guaranteed to be valid and the vtable is static.
                    unsafe { (self.vtable.$fn)(self.ptr.as_ptr().cast_const() $(, $arg)*) }
                }
            )*
        }

        unsafe impl Send for $impl {}
        unsafe impl Sync for $impl {}

        impl Drop for $impl {
            #[inline]
            fn drop(&mut self) {
                // SAFETY: The pointer is guaranteed to be valid and the vtable is static.
                unsafe { (self.vtable.drop)(self.ptr.as_ptr().cast_const()) }
            }
        }

        impl Clone for $impl {
            #[inline]
            fn clone(&self) -> Self {
                // SAFETY: The pointer is valid and the vtable is static.
                let ptr = unsafe {
                    let ptr = self.ptr.as_ptr().cast_const();
                    let ptr = (self.vtable.clone)(ptr);
                    ::core::ptr::NonNull::new_unchecked(ptr.cast_mut())
                };

                Self {
                    ptr,
                    vtable: self.vtable,
                }
            }
        }

        impl $crate::alloc::clone::TryClone for $impl {
            #[inline]
            fn try_clone(&self) -> $crate::alloc::Result<Self> {
                Ok(self.clone())
            }
        }

        impl ::core::fmt::Pointer for $impl {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Pointer::fmt(&self.ptr, f)
            }
        }
    }
}

#[doc(inline)]
pub(crate) use __async_vm_try as async_vm_try;

#[doc(inline)]
pub(crate) use __vm_error as vm_error;

#[doc(inline)]
pub(crate) use __declare_dyn_fn as declare_dyn_fn;

#[doc(inline)]
pub(crate) use __declare_dyn_trait as declare_dyn_trait;
