use crate as rune;

use core::cmp::{Eq, Ordering, PartialEq};
use core::hash;
use core::ops::ControlFlow;

use crate::alloc::{self, HashMap};
use crate::runtime as rt;
use crate::runtime::{RawStr, TypeInfo};
use crate::Hash;

/// Static type information.
#[derive(Debug)]
#[repr(C)]
pub struct StaticType {
    /// The name of the static type.
    pub(crate) name: RawStr,
    /// The hash of the static type.
    pub(crate) hash: Hash,
}

impl StaticType {
    #[inline]
    pub(crate) fn type_info(&'static self) -> TypeInfo {
        TypeInfo::StaticType(self)
    }
}

impl PartialEq for &'static StaticType {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for &'static StaticType {}

impl PartialEq<Hash> for &'static StaticType {
    fn eq(&self, other: &Hash) -> bool {
        self.hash == *other
    }
}

impl hash::Hash for &'static StaticType {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

/// Hash for `::std::u8`.
pub(crate) const BYTE_HASH: Hash = ::rune_macros::hash!(::std::u8);

/// The specialized type information for a byte type.
pub(crate) static BYTE: &StaticType = &StaticType {
    name: RawStr::from_str("u8"),
    hash: BYTE_HASH,
};

impl_static_type!(u8 => BYTE);

/// The specialized type information for a bool type.
pub(crate) static BOOL: &StaticType = &StaticType {
    name: RawStr::from_str("bool"),
    hash: ::rune_macros::hash!(::std::bool),
};

impl_static_type!(bool => BOOL);

/// The specialized type information for a char type.
pub(crate) static CHAR: &StaticType = &StaticType {
    name: RawStr::from_str("char"),
    hash: ::rune_macros::hash!(::std::char),
};

impl_static_type!(char => CHAR);

/// Hash for `::std::i64`.
pub(crate) const INTEGER_HASH: Hash = ::rune_macros::hash!(::std::i64);

/// The specialized type information for a integer type.
pub(crate) static INTEGER: &StaticType = &StaticType {
    name: RawStr::from_str("i64"),
    hash: INTEGER_HASH,
};

impl_static_type!(i8 => INTEGER);
// NB: u8 is its own type BYTE.
impl_static_type!(u16 => INTEGER);
impl_static_type!(i16 => INTEGER);
impl_static_type!(u32 => INTEGER);
impl_static_type!(i32 => INTEGER);
impl_static_type!(u64 => INTEGER);
impl_static_type!(i64 => INTEGER);
impl_static_type!(u128 => INTEGER);
impl_static_type!(i128 => INTEGER);
impl_static_type!(usize => INTEGER);
impl_static_type!(isize => INTEGER);

/// Hash for `::std::f64`.
pub(crate) const FLOAT_HASH: Hash = ::rune_macros::hash!(::std::f64);

pub(crate) static FLOAT: &StaticType = &StaticType {
    name: RawStr::from_str("f64"),
    hash: FLOAT_HASH,
};

impl_static_type!(f32 => FLOAT);
impl_static_type!(f64 => FLOAT);

pub(crate) static STRING: &StaticType = &StaticType {
    name: RawStr::from_str("String"),
    hash: ::rune_macros::hash!(::std::string::String),
};

#[cfg(feature = "alloc")]
impl_static_type!(::rust_alloc::string::String => STRING);
impl_static_type!(alloc::String => STRING);
impl_static_type!(alloc::Box<str> => STRING);
impl_static_type!(str => STRING);

pub(crate) static BYTES: &StaticType = &StaticType {
    name: RawStr::from_str("Bytes"),
    hash: ::rune_macros::hash!(::std::bytes::Bytes),
};

impl_static_type!([u8] => BYTES);

pub(crate) static VEC: &StaticType = &StaticType {
    name: RawStr::from_str("Vec"),
    hash: ::rune_macros::hash!(::std::vec::Vec),
};

impl_static_type!([rt::Value] => VEC);
#[cfg(feature = "alloc")]
impl_static_type!(impl<T> ::rust_alloc::vec::Vec<T> => VEC);
impl_static_type!(impl<T> alloc::Vec<T> => VEC);
impl_static_type!(impl<T> rt::VecTuple<T> => VEC);

pub(crate) static TUPLE: &StaticType = &StaticType {
    name: RawStr::from_str("Tuple"),
    hash: ::rune_macros::hash!(::std::tuple::Tuple),
};

impl_static_type!(rt::OwnedTuple => TUPLE);

pub(crate) static OBJECT: &StaticType = &StaticType {
    name: RawStr::from_str("Object"),
    hash: ::rune_macros::hash!(::std::object::Object),
};

impl_static_type!(rt::Struct => OBJECT);
impl_static_type!(impl<T> HashMap<::rust_alloc::string::String, T> => OBJECT);
impl_static_type!(impl<T> HashMap<alloc::String, T> => OBJECT);

cfg_std! {
    impl_static_type!(impl<T> ::std::collections::HashMap<::rust_alloc::string::String, T> => OBJECT);
    impl_static_type!(impl<T> ::std::collections::HashMap<alloc::String, T> => OBJECT);
}

pub(crate) static RANGE_FROM: &StaticType = &StaticType {
    name: RawStr::from_str("RangeFrom"),
    hash: ::rune_macros::hash!(::std::ops::RangeFrom),
};

pub(crate) static RANGE_FULL: &StaticType = &StaticType {
    name: RawStr::from_str("RangeFull"),
    hash: ::rune_macros::hash!(::std::ops::RangeFull),
};

pub(crate) static RANGE_INCLUSIVE: &StaticType = &StaticType {
    name: RawStr::from_str("RangeInclusive"),
    hash: ::rune_macros::hash!(::std::ops::RangeInclusive),
};

pub(crate) static RANGE_TO_INCLUSIVE: &StaticType = &StaticType {
    name: RawStr::from_str("RangeToInclusive"),
    hash: ::rune_macros::hash!(::std::ops::RangeToInclusive),
};

pub(crate) static RANGE_TO: &StaticType = &StaticType {
    name: RawStr::from_str("RangeTo"),
    hash: ::rune_macros::hash!(::std::ops::RangeTo),
};

pub(crate) static RANGE: &StaticType = &StaticType {
    name: RawStr::from_str("Range"),
    hash: ::rune_macros::hash!(::std::ops::Range),
};

pub(crate) static CONTROL_FLOW: &StaticType = &StaticType {
    name: RawStr::from_str("ControlFlow"),
    hash: ::rune_macros::hash!(::std::ops::ControlFlow),
};

impl_static_type!(impl<C, B> ControlFlow<C, B> => CONTROL_FLOW);

pub(crate) static FUTURE: &StaticType = &StaticType {
    name: RawStr::from_str("Future"),
    hash: ::rune_macros::hash!(::std::future::Future),
};

pub(crate) static GENERATOR: &StaticType = &StaticType {
    name: RawStr::from_str("Generator"),
    hash: ::rune_macros::hash!(::std::ops::generator::Generator),
};

pub(crate) static GENERATOR_STATE: &StaticType = &StaticType {
    name: RawStr::from_str("GeneratorState"),
    hash: ::rune_macros::hash!(::std::ops::generator::GeneratorState),
};

pub(crate) static STREAM: &StaticType = &StaticType {
    name: RawStr::from_str("Stream"),
    hash: ::rune_macros::hash!(::std::stream::Stream),
};

pub(crate) static RESULT: &StaticType = &StaticType {
    name: RawStr::from_str("Result"),
    hash: ::rune_macros::hash!(::std::result::Result),
};

impl_static_type!(impl<T, E> Result<T, E> => RESULT);

pub(crate) static OPTION: &StaticType = &StaticType {
    name: RawStr::from_str("Option"),
    hash: ::rune_macros::hash!(::std::option::Option),
};

impl_static_type!(impl<T> Option<T> => OPTION);

pub(crate) static FUNCTION: &StaticType = &StaticType {
    name: RawStr::from_str("Function"),
    hash: ::rune_macros::hash!(::std::ops::Function),
};

pub(crate) static FORMAT: &StaticType = &StaticType {
    name: RawStr::from_str("Format"),
    hash: ::rune_macros::hash!(::std::fmt::Format),
};

pub(crate) static ORDERING: &StaticType = &StaticType {
    name: RawStr::from_str("Ordering"),
    hash: ::rune_macros::hash!(::std::cmp::Ordering),
};

impl_static_type!(Ordering => ORDERING);

pub(crate) static TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Type"),
    hash: ::rune_macros::hash!(::std::any::Type),
};

impl_static_type!(rt::Type => TYPE);
