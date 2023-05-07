use core::cmp;
use core::hash;

use crate::no_std::collections::HashMap;
use crate::no_std::prelude::*;
use crate::no_std::vec;

use crate::runtime as rt;
use crate::runtime::RawStr;
use crate::Hash;

/// Static type information.
#[derive(Debug)]
#[repr(C)]
pub struct StaticType {
    /// The name of the static type.
    pub name: RawStr,
    /// The hash of the static type.
    pub hash: Hash,
}

impl cmp::PartialEq for &'static StaticType {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl cmp::Eq for &'static StaticType {}

impl hash::Hash for &'static StaticType {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state)
    }
}

/// The specialized type information for a unit.
pub static UNIT_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("unit"),
    // hash for ::std::unit
    hash: Hash::new(0x9f40107c53277b0c),
};

impl_static_type!(() => UNIT_TYPE);
impl_static_type!(rt::UnitStruct => UNIT_TYPE);

/// The specialized type information for a byte type.
pub static BYTE_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("byte"),
    // hash for ::std::byte
    hash: Hash::new(0x1ad282944d94f765),
};

impl_static_type!(u8 => BYTE_TYPE);

/// The specialized type information for a bool type.
pub static BOOL_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("bool"),
    // hash for ::std::bool
    hash: Hash::new(0x981333df5abb043f),
};

impl_static_type!(bool => BOOL_TYPE);

/// The specialized type information for a char type.
pub static CHAR_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("char"),
    // hash for ::std::char
    hash: Hash::new(0x214e0b95f9831430),
};

impl_static_type!(char => CHAR_TYPE);

/// The specialized type information for a integer type.
pub static INTEGER_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("int"),
    // hash for ::std::int
    hash: Hash::new(0x226062cd2b8b5ba),
};

impl_static_type!(i8 => INTEGER_TYPE);
impl_static_type!(u16 => INTEGER_TYPE);
impl_static_type!(i16 => INTEGER_TYPE);
impl_static_type!(u32 => INTEGER_TYPE);
impl_static_type!(i32 => INTEGER_TYPE);
impl_static_type!(u64 => INTEGER_TYPE);
impl_static_type!(i64 => INTEGER_TYPE);
impl_static_type!(u128 => INTEGER_TYPE);
impl_static_type!(i128 => INTEGER_TYPE);
impl_static_type!(usize => INTEGER_TYPE);
impl_static_type!(isize => INTEGER_TYPE);

/// The specialized type information for a float type.
pub static FLOAT_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("float"),
    // hash for ::std::float
    hash: Hash::new(0xb75367086ae66d8b),
};

impl_static_type!(f32 => FLOAT_TYPE);
impl_static_type!(f64 => FLOAT_TYPE);

/// The specialized type information for a string type.
pub static STRING_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("String"),
    // hash for ::std::string::String
    hash: Hash::new(0x2f4720d36d2b70c),
};

impl_static_type!(String => STRING_TYPE);
impl_static_type!(str => STRING_TYPE);

/// The specialized type information for a bytes type.
pub static BYTES_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Bytes"),
    // hash for ::std::bytes::Bytes
    hash: Hash::new(0x3470d9498e601529),
};

impl_static_type!(rt::Bytes => BYTES_TYPE);
impl_static_type!([u8] => BYTES_TYPE);

/// The specialized type information for a vector type.
pub static VEC_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Vec"),
    // hash for ::std::vec::Vec
    hash: Hash::new(0xc11b1e769aea94f2),
};

impl_static_type!(rt::Vec => VEC_TYPE);
impl_static_type!(impl<T> vec::Vec<T> => VEC_TYPE);
impl_static_type!([rt::Value] => VEC_TYPE);
impl_static_type!(impl<T> rt::VecTuple<T> => VEC_TYPE);

/// The specialized type information for an anonymous tuple type.
pub static TUPLE_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Tuple"),
    // hash for ::std::Tuple
    hash: Hash::new(0xf94a979fcb406fde),
};

impl_static_type!(rt::Tuple => TUPLE_TYPE);

/// The specialized type information for an anonymous object type.
pub static OBJECT_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Object"),
    // hash for ::std::object::Object
    hash: Hash::new(0xd080f2e951218dde),
};

impl_static_type!(rt::Object => OBJECT_TYPE);
impl_static_type!(rt::Struct => OBJECT_TYPE);

/// The specialized type information for the range type.
pub static RANGE_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Range"),
    // hash for ::std::ops::Range
    hash: Hash::new(0x700a34bcd6630cba),
};

impl_static_type!(rt::Range => RANGE_TYPE);

/// The specialized type information for a future type.
pub static FUTURE_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Future"),
    // hash for ::std::future::Future
    hash: Hash::new(0x157cfe667ac47042),
};

impl_static_type!(rt::Future => FUTURE_TYPE);

/// The specialized type information for a generator type.
pub static GENERATOR_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Generator"),
    // hash for ::std::generator::Generator
    hash: Hash::new(0x9041ff127bcec639),
};

impl_static_type!(rt::Generator<rt::Vm> => GENERATOR_TYPE);

/// The specialized type information for a generator state type.
pub static GENERATOR_STATE_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("GeneratorState"),
    // hash for ::std::generator::GeneratorState
    hash: Hash::new(0xae44122e30ae33ae),
};

impl_static_type!(rt::GeneratorState => GENERATOR_STATE_TYPE);

/// The specialized type information for the `Stream` type.
pub static STREAM_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Stream"),
    // hash for ::std::stream::Stream
    hash: Hash::new(0xd849ef81ff581a21),
};

impl_static_type!(rt::Stream<rt::Vm> => STREAM_TYPE);

/// The specialized type information for a result type.
pub static RESULT_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Result"),
    // hash for ::std::result::Result
    hash: Hash::new(0x1978eae6b50a98ef),
};

impl_static_type!(impl<T, E> Result<T, E> => RESULT_TYPE);

/// The specialized type information for a option type.
pub static OPTION_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Option"),
    // hash for ::std::option::Option
    hash: Hash::new(0xc0958f246e193e78),
};

impl_static_type!(impl<T> Option<T> => OPTION_TYPE);

/// The specialized type information for a function pointer type.
pub static FUNCTION_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Function"),
    // hash for ::std::ops::Function
    hash: Hash::new(0x20b8050151a2855),
};

impl_static_type!(rt::Function => FUNCTION_TYPE);
impl_static_type!(impl<T> HashMap<String, T> => OBJECT_TYPE);

/// The specialized type information for a fmt spec types.
pub static FORMAT_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Format"),
    // hash for ::std::fmt::Format
    hash: Hash::new(0xc331a83f0ad5a659),
};

impl_static_type!(rt::Format => FORMAT_TYPE);

/// The specialized type information for the iterator type.
pub static ITERATOR_TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Iterator"),
    // hash for ::std::iter::Iterator
    hash: Hash::new(0xe08fbd4d99f308e9),
};

impl_static_type!(rt::Iterator => ITERATOR_TYPE);

/// The specialized type information for type objects.
pub static TYPE: &StaticType = &StaticType {
    name: RawStr::from_str("Type"),
    // hash for ::std::Type
    hash: Hash::new(0xe14fc50ece26203),
};
