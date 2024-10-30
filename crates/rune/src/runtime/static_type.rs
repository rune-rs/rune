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
        TypeInfo::static_type(self)
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

impl_static_type!(u8, BYTE, BYTE_HASH);

pub(crate) const BOOL_HASH: Hash = ::rune_macros::hash!(::std::bool);

/// The specialized type information for a bool type.
pub(crate) static BOOL: &StaticType = &StaticType {
    name: RawStr::from_str("bool"),
    hash: BOOL_HASH,
};

impl_static_type!(bool, BOOL, BOOL_HASH);

pub(crate) const CHAR_HASH: Hash = ::rune_macros::hash!(::std::char);

/// The specialized type information for a char type.
pub(crate) static CHAR: &StaticType = &StaticType {
    name: RawStr::from_str("char"),
    hash: CHAR_HASH,
};

impl_static_type!(char, CHAR, CHAR_HASH);

/// Hash for `::std::i64`.
pub(crate) const SIGNED_HASH: Hash = ::rune_macros::hash!(::std::i64);

/// The specialized type information for a integer type.
pub(crate) static SIGNED: &StaticType = &StaticType {
    name: RawStr::from_str("i64"),
    hash: SIGNED_HASH,
};

impl_static_type!(i8, SIGNED, SIGNED_HASH);
impl_static_type!(i16, SIGNED, SIGNED_HASH);
impl_static_type!(i32, SIGNED, SIGNED_HASH);
impl_static_type!(i64, SIGNED, SIGNED_HASH);
impl_static_type!(i128, SIGNED, SIGNED_HASH);
impl_static_type!(isize, SIGNED, SIGNED_HASH);

/// Hash for `::std::u64`.
pub(crate) const UNSIGNED_HASH: Hash = ::rune_macros::hash!(::std::u64);

/// The specialized type information for an unsigned integer type.
pub(crate) static UNSIGNED: &StaticType = &StaticType {
    name: RawStr::from_str("u64"),
    hash: UNSIGNED_HASH,
};

// NB: u8 is its own type BYTE.
impl_static_type!(u16, UNSIGNED, UNSIGNED_HASH);
impl_static_type!(u32, UNSIGNED, UNSIGNED_HASH);
impl_static_type!(u64, UNSIGNED, UNSIGNED_HASH);
impl_static_type!(u128, UNSIGNED, UNSIGNED_HASH);
impl_static_type!(usize, UNSIGNED, UNSIGNED_HASH);

/// Hash for `::std::f64`.
pub(crate) const FLOAT_HASH: Hash = ::rune_macros::hash!(::std::f64);

pub(crate) static FLOAT: &StaticType = &StaticType {
    name: RawStr::from_str("f64"),
    hash: FLOAT_HASH,
};

impl_static_type!(f32, FLOAT, FLOAT_HASH);
impl_static_type!(f64, FLOAT, FLOAT_HASH);

pub(crate) const STRING_HASH: Hash = ::rune_macros::hash!(::std::string::String);
pub(crate) static STRING: &StaticType = &StaticType {
    name: RawStr::from_str("String"),
    hash: STRING_HASH,
};

#[cfg(feature = "alloc")]
impl_static_type!(::rust_alloc::string::String, STRING, STRING_HASH);
impl_static_type!(alloc::String, STRING, STRING_HASH);
impl_static_type!(alloc::Box<str>, STRING, STRING_HASH);
impl_static_type!(str, STRING, STRING_HASH);

pub(crate) const BYTES_HASH: Hash = ::rune_macros::hash!(::std::bytes::Bytes);

pub(crate) static BYTES: &StaticType = &StaticType {
    name: RawStr::from_str("Bytes"),
    hash: BYTES_HASH,
};

impl_static_type!([u8], BYTES, BYTES_HASH);

pub(crate) const VEC_HASH: Hash = ::rune_macros::hash!(::std::vec::Vec);

pub(crate) static VEC: &StaticType = &StaticType {
    name: RawStr::from_str("Vec"),
    hash: VEC_HASH,
};

impl_static_type!([rt::Value], VEC, VEC_HASH);
#[cfg(feature = "alloc")]
impl_static_type!(impl<T> ::rust_alloc::vec::Vec<T>, VEC, VEC_HASH);
impl_static_type!(impl<T> alloc::Vec<T>, VEC, VEC_HASH);
impl_static_type!(impl<T> rt::VecTuple<T>, VEC, VEC_HASH);

pub(crate) const TUPLE_HASH: Hash = ::rune_macros::hash!(::std::tuple::Tuple);

pub(crate) static TUPLE: &StaticType = &StaticType {
    name: RawStr::from_str("Tuple"),
    hash: TUPLE_HASH,
};

impl_static_type!(rt::OwnedTuple, TUPLE, TUPLE_HASH);

pub(crate) const OBJECT_HASH: Hash = ::rune_macros::hash!(::std::object::Object);

pub(crate) static OBJECT: &StaticType = &StaticType {
    name: RawStr::from_str("Object"),
    hash: OBJECT_HASH,
};

impl_static_type!(rt::Struct, OBJECT, OBJECT_HASH);
impl_static_type!(impl<T> HashMap<::rust_alloc::string::String, T>, OBJECT, OBJECT_HASH);
impl_static_type!(impl<T> HashMap<alloc::String, T>, OBJECT, OBJECT_HASH);

cfg_std! {
    impl_static_type!(impl<T> ::std::collections::HashMap<::rust_alloc::string::String, T>, OBJECT, OBJECT_HASH);
    impl_static_type!(impl<T> ::std::collections::HashMap<alloc::String, T>, OBJECT, OBJECT_HASH);
}

pub(crate) const RANGE_FROM_HASH: Hash = ::rune_macros::hash!(::std::ops::RangeFrom);

pub(crate) static RANGE_FROM: &StaticType = &StaticType {
    name: RawStr::from_str("RangeFrom"),
    hash: RANGE_FROM_HASH,
};

pub(crate) const RANGE_FULL_HASH: Hash = ::rune_macros::hash!(::std::ops::RangeFull);

pub(crate) static RANGE_FULL: &StaticType = &StaticType {
    name: RawStr::from_str("RangeFull"),
    hash: RANGE_FULL_HASH,
};

pub(crate) const RANGE_INCLUSIVE_HASH: Hash = ::rune_macros::hash!(::std::ops::RangeInclusive);

pub(crate) static RANGE_INCLUSIVE: &StaticType = &StaticType {
    name: RawStr::from_str("RangeInclusive"),
    hash: RANGE_INCLUSIVE_HASH,
};

pub(crate) const RANGE_TO_INCLUSIVE_HASH: Hash = ::rune_macros::hash!(::std::ops::RangeToInclusive);

pub(crate) static RANGE_TO_INCLUSIVE: &StaticType = &StaticType {
    name: RawStr::from_str("RangeToInclusive"),
    hash: RANGE_TO_INCLUSIVE_HASH,
};

pub(crate) const RANGE_TO_HASH: Hash = ::rune_macros::hash!(::std::ops::RangeTo);

pub(crate) static RANGE_TO: &StaticType = &StaticType {
    name: RawStr::from_str("RangeTo"),
    hash: RANGE_TO_HASH,
};

pub(crate) const RANGE_HASH: Hash = ::rune_macros::hash!(::std::ops::Range);

pub(crate) static RANGE: &StaticType = &StaticType {
    name: RawStr::from_str("Range"),
    hash: RANGE_HASH,
};

pub(crate) const CONTROL_FLOW_HASH: Hash = ::rune_macros::hash!(::std::ops::ControlFlow);

pub(crate) static CONTROL_FLOW: &StaticType = &StaticType {
    name: RawStr::from_str("ControlFlow"),
    hash: CONTROL_FLOW_HASH,
};

impl_static_type!(impl<C, B> ControlFlow<C, B>, CONTROL_FLOW, CONTROL_FLOW_HASH);

pub(crate) const FUTURE_HASH: Hash = ::rune_macros::hash!(::std::future::Future);
pub(crate) static FUTURE: &StaticType = &StaticType {
    name: RawStr::from_str("Future"),
    hash: FUTURE_HASH,
};

pub(crate) const GENERATOR_HASH: Hash = ::rune_macros::hash!(::std::ops::generator::Generator);
pub(crate) static GENERATOR: &StaticType = &StaticType {
    name: RawStr::from_str("Generator"),
    hash: GENERATOR_HASH,
};

pub(crate) const GENERATOR_STATE_HASH: Hash =
    ::rune_macros::hash!(::std::ops::generator::GeneratorState);
pub(crate) static GENERATOR_STATE: &StaticType = &StaticType {
    name: RawStr::from_str("GeneratorState"),
    hash: GENERATOR_STATE_HASH,
};

pub(crate) const STREAM_HASH: Hash = ::rune_macros::hash!(::std::stream::Stream);
pub(crate) static STREAM: &StaticType = &StaticType {
    name: RawStr::from_str("Stream"),
    hash: STREAM_HASH,
};

pub(crate) const RESULT_HASH: Hash = ::rune_macros::hash!(::std::result::Result);

pub(crate) static RESULT: &StaticType = &StaticType {
    name: RawStr::from_str("Result"),
    hash: RESULT_HASH,
};

impl_static_type!(impl<T, E> Result<T, E>, RESULT, RESULT_HASH);

pub(crate) const OPTION_HASH: Hash = ::rune_macros::hash!(::std::option::Option);

pub(crate) static OPTION: &StaticType = &StaticType {
    name: RawStr::from_str("Option"),
    hash: OPTION_HASH,
};

impl_static_type!(impl<T> Option<T>, OPTION, OPTION_HASH);

pub(crate) const FUNCTION_HASH: Hash = ::rune_macros::hash!(::std::ops::Function);
pub(crate) static FUNCTION: &StaticType = &StaticType {
    name: RawStr::from_str("Function"),
    hash: FUNCTION_HASH,
};

pub(crate) const FORMAT_HASH: Hash = ::rune_macros::hash!(::std::fmt::Format);
pub(crate) static FORMAT: &StaticType = &StaticType {
    name: RawStr::from_str("Format"),
    hash: FORMAT_HASH,
};

pub(crate) const ORDERING_HASH: Hash = ::rune_macros::hash!(::std::cmp::Ordering);
pub(crate) static ORDERING: &StaticType = &StaticType {
    name: RawStr::from_str("Ordering"),
    hash: ORDERING_HASH,
};

impl_static_type!(Ordering, ORDERING, ORDERING_HASH);

pub(crate) const HASH: Hash = ::rune_macros::hash!(::std::any::Type);
pub(crate) static TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Type"),
    hash: HASH,
};

impl_static_type!(rt::Type, TYPE, HASH);
