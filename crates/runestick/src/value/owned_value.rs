use crate::any::Any;
use crate::bytes::Bytes;
use crate::collections::HashMap;
use crate::future::Future;
use crate::hash::Hash;
use crate::value::Object;

/// A object value taken from the virtual machine.
#[derive(Debug)]
pub struct OwnedTypedObject {
    /// The type of the object.
    pub ty: Hash,
    /// The content of the object.
    pub object: Object<OwnedValue>,
}

/// A tuple value taken from the virtual machine.
#[derive(Debug)]
pub struct OwnedTypedTuple {
    /// The type of the tuple.
    pub ty: Hash,
    /// The content of the tuple.
    pub tuple: Box<[OwnedValue]>,
}

/// A value taken from the virtual machine.
#[derive(Debug)]
pub enum OwnedValue {
    /// An empty unit.
    Unit,
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// A byte.
    Byte(u8),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A string.
    String(String),
    /// A byte array.
    Bytes(Bytes),
    /// A vector.
    Vec(Vec<OwnedValue>),
    /// A tuple.
    Tuple(Box<[OwnedValue]>),
    /// An object.
    Object(HashMap<String, OwnedValue>),
    /// Reference to an external type.
    External(Any),
    /// A type to a different value.
    Type(Hash),
    /// A future in the virtual machine.
    Future(Future),
    /// An optional value.
    Option(Option<Box<OwnedValue>>),
    /// A result value.
    Result(Result<Box<OwnedValue>, Box<OwnedValue>>),
    /// A typed object.
    TypedObject(OwnedTypedObject),
    /// A typed tuple.
    TypedTuple(OwnedTypedTuple),
}
