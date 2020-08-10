use crate::any::Any;
use crate::collections::HashMap;
use crate::future::Future;
use crate::hash::Hash;

#[derive(Debug)]
/// A value peeked out of the stack.
pub enum OwnedValue {
    /// An empty unit.
    Unit,
    /// A string.
    String(String),
    /// A vector.
    Vec(Vec<OwnedValue>),
    /// A tuple.
    Tuple(Box<[OwnedValue]>),
    /// An object.
    Object(HashMap<String, OwnedValue>),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// Reference to an external type.
    External(Any),
    /// A type to a different value.
    Type(Hash),
    /// A function.
    Fn(Hash),
    /// A future in the virtual machine.
    Future(Future),
    /// An optional value.
    Option(Option<Box<OwnedValue>>),
    /// A result value.
    Result(Result<Box<OwnedValue>, Box<OwnedValue>>),
}
