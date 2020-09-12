/// Build an implementation of `TypeOf` basic of a static type.
macro_rules! impl_static_type {
    (impl <$($p:ident),*> $ty:ty => $static_type:expr) => {
        impl<$($p,)*> $crate::TypeOf for $ty {
            fn type_of() -> $crate::Type {
                $crate::Type::from($static_type)
            }

            fn type_info() -> $crate::TypeInfo {
                $crate::TypeInfo::StaticType($static_type)
            }
        }
    };

    ($ty:ty => $static_type:expr) => {
        impl $crate::TypeOf for $ty {
            fn type_of() -> $crate::Type {
                $crate::Type::from($static_type)
            }

            fn type_info() -> $crate::TypeInfo {
                $crate::TypeInfo::StaticType($static_type)
            }
        }
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
