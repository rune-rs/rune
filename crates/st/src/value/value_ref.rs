use crate::any::Any;
use crate::collections::HashMap;
use crate::hash::Hash;
use crate::vm::Ref;

#[derive(Debug)]
/// A value peeked out of the stack.
pub enum ValueRef<'vm> {
    /// An empty value indicating nothing.
    Unit,
    /// A string.
    String(Ref<'vm, String>),
    /// An array.
    Array(Vec<ValueRef<'vm>>),
    /// An object.
    Object(HashMap<String, ValueRef<'vm>>),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// Reference to an external type.
    External(Ref<'vm, dyn Any>),
    /// Reference to a value type.
    Type(Hash),
    /// A function.
    Fn(Hash),
}
