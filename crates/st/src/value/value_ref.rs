use crate::external::External;
use crate::vm::Ref;

#[derive(Debug)]
/// A value peeked out of the stack.
pub enum ValueRef<'a> {
    /// An empty unit.
    Unit,
    /// A string.
    String(Ref<'a, String>),
    /// An array.
    Array(Vec<ValueRef<'a>>),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// Reference to an external type.
    External(Ref<'a, dyn External>),
}
