use crate as rune;

use core::cmp::{Eq, Ordering, PartialEq};
use core::hash;
use core::ops::ControlFlow;

use crate::alloc::clone::TryClone;
use crate::alloc::{self, HashMap};
use crate::runtime as rt;
use crate::runtime::TypeInfo;
use crate::{Hash, Item};

/// Static type information.
#[derive(TryClone, Debug, Clone, Copy)]
pub struct StaticType {
    /// The name of the static type.
    #[try_clone(copy)]
    pub(crate) item: &'static Item,
    /// The hash of the static type.
    #[try_clone(copy)]
    pub(crate) hash: Hash,
}

impl StaticType {
    #[inline]
    pub(crate) fn type_info(self) -> TypeInfo {
        TypeInfo::static_type(self)
    }
}

impl PartialEq for StaticType {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for StaticType {}

impl PartialEq<Hash> for StaticType {
    fn eq(&self, other: &Hash) -> bool {
        self.hash == *other
    }
}

impl hash::Hash for StaticType {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

macro_rules! any_type {
    (
        $(
            $(#[$($meta:meta)*])*
            $path:path {
                $(
                    $(#[$($impl_meta:meta)*])*
                    impl $(<$($p:ident),*>)? for $ty:ty;
                )*
            }
        )*
    ) => {
        $(
            $(
                $(#[$($impl_meta)*])*
                impl $(<$($p,)*>)* $crate::TypeHash for $ty {
                    const HASH: $crate::Hash = ::rune_macros::hash!($path);
                }

                $(#[$($impl_meta)*])*
                impl $(<$($p,)*>)* $crate::runtime::TypeOf for $ty
                where
                    $(
                        $($p: $crate::runtime::MaybeTypeOf,)*
                    )*
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

                $(#[$($impl_meta)*])*
                impl $(<$($p,)*>)* $crate::runtime::MaybeTypeOf for $ty
                where
                    $(
                        $($p: $crate::runtime::MaybeTypeOf,)*
                    )*
                {
                    #[inline]
                    fn maybe_type_of() -> $crate::alloc::Result<$crate::compile::meta::DocType> {
                        Ok($crate::compile::meta::DocType::new(<$ty as $crate::TypeHash>::HASH))
                    }
                }
            )*
        )*
    }
}

macro_rules! static_type {
    (
        $(
            $(#[$($meta:meta)*])*
            $vis:vis static [$name:ident, $hash:ident] = $path:path {
                $(
                    $(#[$($impl_meta:meta)*])*
                    impl $(<$($p:ident),*>)? for $ty:ty;
                )*
            }
        )*
    ) => {
        $(
            $vis const $hash: Hash = ::rune_macros::hash!($path);

            $(#[$($meta)*])*
            $vis const $name: StaticType = StaticType {
                item: ::rune_macros::item!($path),
                hash: $hash,
            };

            $(
                $(#[$($impl_meta)*])*
                impl_static_type!(impl $(<$($p),*>)* for $ty, $name, $hash);
            )*
        )*
    }
}

any_type! {
    /// The specialized type information for a bool type.
    ::std::bool {
        impl for bool;
    }

    /// The specialized type information for a char type.
    ::std::char {
        impl for char;
    }

    /// The specialized type information for a integer type.
    ::std::i64 {
        impl for i8;
        impl for i16;
        impl for i32;
        impl for i64;
        impl for i128;
        impl for isize;
    }

    /// The specialized type information for an unsigned integer type.
    ::std::u64 {
        impl for u8;
        impl for u16;
        impl for u32;
        impl for u64;
        impl for u128;
        impl for usize;
    }

    /// The specialized type information for a float type.
    ::std::f64 {
        impl for f32;
        impl for f64;
    }

    ::std::ops::ControlFlow {
        impl<C, B> for ControlFlow<C, B>;
    }

    /// The specialized type information for the [`Bytes`] type.
    ::std::bytes::Bytes {
        impl for [u8];
    }

    ::std::cmp::Ordering {
        impl for Ordering;
    }

    /// The specialized type information for a float type.
    ::std::string::String {
        #[cfg(feature = "alloc")]
        #[cfg_attr(rune_docsrs, doc(cfg(feature = "alloc")))]
        impl for ::rust_alloc::string::String;
        impl for alloc::String;
        impl for alloc::Box<str>;
        impl for str;
    }

    /// The specialized type information for the [`Vec`] type.
    ::std::vec::Vec {
        impl for [rt::Value];
        #[cfg(feature = "alloc")]
        #[cfg_attr(rune_docsrs, doc(cfg(feature = "alloc")))]
        impl<T> for ::rust_alloc::vec::Vec<T>;
        impl<T> for alloc::Vec<T>;
        impl<T> for rt::VecTuple<T>;
    }
}

static_type! {
    /// The specialized type information for the [`Tuple`] type.
    pub(crate) static [TUPLE, TUPLE_HASH] = ::std::tuple::Tuple {
        impl for rt::OwnedTuple;
    }

    /// The specialized type information for the [`Object`] type.
    pub(crate) static [OBJECT, OBJECT_HASH] = ::std::object::Object {
        impl for rt::Struct;
        impl<T> for HashMap<::rust_alloc::string::String, T>;
        impl<T> for HashMap<alloc::String, T>;

        #[cfg(feature = "std")]
        #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
        impl<T> for ::std::collections::HashMap<::rust_alloc::string::String, T>;

        #[cfg(feature = "std")]
        #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
        impl<T> for ::std::collections::HashMap<alloc::String, T>;
    }

    pub(crate) static [FUTURE, FUTURE_HASH] = ::std::future::Future {}

    pub(crate) static [GENERATOR, GENERATOR_HASH] = ::std::ops::generator::Generator {}

    pub(crate) static [STREAM, STREAM_HASH] = ::std::stream::Stream {}

    pub(crate) static [RESULT, RESULT_HASH] = ::std::result::Result {
        impl<T, E> for Result<T, E>;
    }

    pub(crate) static [OPTION, OPTION_HASH] = ::std::option::Option {
        impl<T> for Option<T>;
    }

    pub(crate) static [FUNCTION, FUNCTION_HASH] = ::std::ops::Function {}

    pub(crate) static [TYPE, TYPE_HASH] = ::std::any::Type {
        impl for rt::Type;
    }
}
