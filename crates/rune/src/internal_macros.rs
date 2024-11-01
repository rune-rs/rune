/// Helper to borrow out a [ResolveContext][crate::parse::ResolveContext].
macro_rules! resolve_context {
    ($q:expr) => {
        $crate::parse::ResolveContext {
            sources: $q.sources,
            storage: $q.storage,
        }
    };
}

macro_rules! impl_any_type {
    (impl $(<$($p:ident),*>)? for $ty:ty, $path:path) => {
        impl $(<$($p,)*>)* $crate::TypeHash for $ty {
            const HASH: $crate::Hash = ::rune_macros::hash!($path);
        }

        impl $(<$($p,)*>)* $crate::runtime::TypeOf for $ty
        where
            $($($p: $crate::runtime::MaybeTypeOf,)*)*
        {
            const STATIC_TYPE_INFO: $crate::runtime::StaticTypeInfo = $crate::runtime::StaticTypeInfo::any_type_info(
                $crate::runtime::AnyTypeInfo::new(
                    {
                        fn full_name(f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                            write!(f, "{}", ::rune_macros::item!($path))
                        }

                        full_name
                    },
                    <Self as $crate::TypeHash>::HASH,
                )
            );
        }

        impl $(<$($p,)*>)* $crate::runtime::MaybeTypeOf for $ty
        where
            $($($p: $crate::runtime::MaybeTypeOf,)*)*
        {
            #[inline]
            fn maybe_type_of() -> $crate::alloc::Result<$crate::compile::meta::DocType> {
                Ok($crate::compile::meta::DocType::new(<$ty as $crate::TypeHash>::HASH))
            }
        }
    }
}

/// Build an implementation of `TypeOf` basic of a static type.
macro_rules! impl_static_type {
    (impl $(<$($p:ident),*>)? for $ty:ty, $name:ident, $hash:ident) => {
        impl $(<$($p,)*>)* $crate::TypeHash for $ty {
            const HASH: $crate::Hash = $crate::runtime::static_type::$hash;
        }

        impl $(<$($p,)*>)* $crate::runtime::TypeOf for $ty
        where
            $(
                $($p: $crate::runtime::MaybeTypeOf,)*
            )*
        {
            const STATIC_TYPE_INFO: $crate::runtime::StaticTypeInfo = $crate::runtime::StaticTypeInfo::static_type($crate::runtime::static_type::$name);
        }

        impl $(<$($p,)*>)* $crate::runtime::MaybeTypeOf for $ty
        where
            $(
                $($p: $crate::runtime::MaybeTypeOf,)*
            )*
        {
            #[inline]
            fn maybe_type_of() -> $crate::alloc::Result<$crate::compile::meta::DocType> {
                $crate::compile::meta::DocType::with_generics(
                    <$ty as $crate::TypeHash>::HASH,
                    [$($(<$p as $crate::runtime::MaybeTypeOf>::maybe_type_of()?),*)*]
                )
            }
        }
    };
}

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

macro_rules! cfg_std {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "std")]
            #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
            $item
        )*
    }
}

macro_rules! from_value_ref {
    ($ty:ty, $into_ref:ident, $into_mut:ident, $into:ident) => {
        impl $crate::runtime::UnsafeToRef for $ty {
            type Guard = $crate::runtime::RawAnyGuard;

            unsafe fn unsafe_to_ref<'a>(
                value: $crate::runtime::Value,
            ) -> $crate::runtime::VmResult<(&'a Self, Self::Guard)> {
                let value = vm_try!(value.$into_ref());
                let (value, guard) = $crate::runtime::Ref::into_raw(value);
                $crate::runtime::VmResult::Ok((value.as_ref(), guard))
            }
        }

        impl $crate::runtime::UnsafeToMut for $ty {
            type Guard = $crate::runtime::RawAnyGuard;

            unsafe fn unsafe_to_mut<'a>(
                value: $crate::runtime::Value,
            ) -> $crate::runtime::VmResult<(&'a mut Self, Self::Guard)> {
                let value = vm_try!(value.$into_mut());
                let (mut value, guard) = $crate::runtime::Mut::into_raw(value);
                $crate::runtime::VmResult::Ok((value.as_mut(), guard))
            }
        }

        impl $crate::runtime::FromValue for $crate::runtime::Ref<$ty> {
            fn from_value(value: Value) -> Result<Self, $crate::runtime::RuntimeError> {
                let value = value.$into_ref()?;
                Ok(value)
            }
        }

        impl $crate::runtime::FromValue for $crate::runtime::Mut<$ty> {
            fn from_value(value: Value) -> Result<Self, $crate::runtime::RuntimeError> {
                let value = value.$into_mut()?;
                Ok(value)
            }
        }
    };
}

/// Implements a set of common value conversions.
macro_rules! from_value2 {
    ($ty:ty, $into_ref:ident, $into_mut:ident, $into:ident) => {
        impl $crate::runtime::FromValue for $ty {
            fn from_value(value: Value) -> Result<Self, $crate::runtime::RuntimeError> {
                let value = value.$into()?;
                Ok(value)
            }
        }

        from_value_ref!($ty, $into_ref, $into_mut, $into);
    };
}
