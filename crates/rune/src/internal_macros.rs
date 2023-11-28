/// Helper to borrow out a [ResolveContext][crate::parse::ResolveContext].
macro_rules! resolve_context {
    ($q:expr) => {
        $crate::parse::ResolveContext {
            sources: $q.sources,
            storage: $q.storage,
        }
    };
}

/// Build an implementation of `TypeOf` basic of a static type.
macro_rules! impl_static_type {
    (impl <$($p:ident),*> $ty:ty => $static_type:expr $(, where $($where:tt)+)?) => {
        impl<$($p,)*> $crate::runtime::TypeOf for $ty $(where $($where)+)* {
            #[inline]
            fn type_hash() -> $crate::Hash {
                $static_type.hash
            }

            #[inline]
            fn type_info() -> $crate::runtime::TypeInfo {
                $crate::runtime::TypeInfo::StaticType($static_type)
            }
        }

        impl<$($p,)*> $crate::runtime::MaybeTypeOf for $ty $(where $($where)+)* {
            #[inline]
            fn maybe_type_of() -> Option<$crate::runtime::FullTypeOf> {
                Some(<$ty as $crate::runtime::TypeOf>::type_of())
            }
        }
    };

    ($ty:ty => $static_type:expr) => {
        impl_static_type!(impl <> $ty => $static_type);
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

macro_rules! cfg_emit {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "emit")]
            #[cfg_attr(rune_docsrs, doc(cfg(feature = "emit")))]
            $item
        )*
    }
}

macro_rules! cfg_workspace {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "workspace")]
            #[cfg_attr(rune_docsrs, doc(cfg(feature = "workspace")))]
            $item
        )*
    }
}

macro_rules! cfg_cli {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "cli")]
            #[cfg_attr(rune_docsrs, doc(cfg(feature = "cli")))]
            $item
        )*
    }
}

macro_rules! cfg_doc {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "doc")]
            #[cfg_attr(rune_docsrs, doc(cfg(feature = "doc")))]
            $item
        )*
    }
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

/// Implements a set of common value conversions.
macro_rules! from_value {
    ($ty:ty, $into:ident) => {
        impl $crate::runtime::FromValue for $ty {
            fn from_value(value: Value) -> $crate::runtime::VmResult<Self> {
                let value = vm_try!(value.$into());
                let value = vm_try!(value.take());
                $crate::runtime::VmResult::Ok(value)
            }
        }

        impl $crate::runtime::UnsafeToRef for $ty {
            type Guard = $crate::runtime::RawRef;

            unsafe fn unsafe_to_ref<'a>(
                value: $crate::runtime::Value,
            ) -> $crate::runtime::VmResult<(&'a Self, Self::Guard)> {
                let value = vm_try!(value.$into());
                let value = vm_try!(value.into_ref());
                let (value, guard) = $crate::runtime::Ref::into_raw(value);
                $crate::runtime::VmResult::Ok((value.as_ref(), guard))
            }
        }

        impl $crate::runtime::UnsafeToMut for $ty {
            type Guard = $crate::runtime::RawMut;

            unsafe fn unsafe_to_mut<'a>(
                value: $crate::runtime::Value,
            ) -> $crate::runtime::VmResult<(&'a mut Self, Self::Guard)> {
                let value = vm_try!(value.$into());
                let value = vm_try!(value.into_mut());
                let (mut value, guard) = $crate::runtime::Mut::into_raw(value);
                $crate::runtime::VmResult::Ok((value.as_mut(), guard))
            }
        }

        impl $crate::runtime::FromValue for $crate::runtime::Shared<$ty> {
            #[inline]
            fn from_value(value: $crate::runtime::Value) -> $crate::runtime::VmResult<Self> {
                value.$into()
            }
        }

        impl $crate::runtime::FromValue for $crate::runtime::Ref<$ty> {
            fn from_value(value: Value) -> $crate::runtime::VmResult<Self> {
                let value = vm_try!(value.$into());
                let value = vm_try!(value.into_ref());
                $crate::runtime::VmResult::Ok(value)
            }
        }

        impl $crate::runtime::FromValue for $crate::runtime::Mut<$ty> {
            fn from_value(value: Value) -> VmResult<Self> {
                let value = vm_try!(value.$into());
                let value = vm_try!(value.into_mut());
                $crate::runtime::VmResult::Ok(value)
            }
        }
    };
}
