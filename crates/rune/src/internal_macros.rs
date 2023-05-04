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
    ($macro:tt) => {
        $macro! {
            {A, a, 16},
            {B, b, 15},
            {C, c, 14},
            {D, d, 13},
            {E, e, 12},
            {F, f, 11},
            {G, g, 10},
            {H, h, 9},
            {I, i, 8},
            {J, j, 7},
            {K, k, 6},
            {L, l, 5},
            {M, m, 4},
            {N, n, 3},
            {O, o, 2},
            {P, p, 1},
        }
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
