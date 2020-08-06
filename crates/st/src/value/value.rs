use crate::any::Any;
use crate::collections::HashMap;
use crate::hash::Hash;

#[derive(Debug)]
/// A value peeked out of the stack.
pub enum Value {
    /// An empty unit.
    Unit,
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object.
    Object(HashMap<String, Value>),
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
}
