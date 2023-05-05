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
    (impl <$($p:ident),*> $ty:ty => $static_type:expr) => {
        impl<$($p,)*> $crate::runtime::TypeOf for $ty {
            #[inline]
            fn type_hash() -> $crate::Hash {
                $static_type.hash
            }

            #[inline]
            fn type_info() -> $crate::runtime::TypeInfo {
                $crate::runtime::TypeInfo::StaticType($static_type)
            }
        }

        impl<$($p,)*> $crate::runtime::MaybeTypeOf for $ty {
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
        $macro!(1, A a 1);
        $macro!(2, A a 1, B b 2);
        $macro!(3, A a 1, B b 2, C c 3);
        $macro!(4, A a 1, B b 2, C c 3, D d 4);
        $macro!(5, A a 1, B b 2, C c 3, D d 4, E e 5);
        $macro!(6, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6);
        $macro!(7, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7);
        $macro!(8, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8);
        $macro!(9, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9);
        $macro!(10, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9, J j 10);
        $macro!(11, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9, J j 10, K k 11);
        $macro!(12, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9, J j 10, K k 11, L l 12);
        $macro!(13, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9, J j 10, K k 11, L l 12, M m 13);
        $macro!(14, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9, J j 10, K k 11, L l 12, M m 13, N n 14);
        $macro!(15, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9, J j 10, K k 11, L l 12, M m 13, N n 14, O o 15);
        $macro!(16, A a 1, B b 2, C c 3, D d 4, E e 5, F f 6, G g 7, H h 8, I i 9, J j 10, K k 11, L l 12, M m 13, N n 14, O o 15, P p 16);
    };
}

macro_rules! cfg_emit {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "emit")]
            #[cfg_attr(docsrs, doc(cfg(feature = "emit")))]
            $item
        )*
    }
}

macro_rules! cfg_workspace {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "workspace")]
            #[cfg_attr(docsrs, doc(cfg(feature = "workspace")))]
            $item
        )*
    }
}
