use crate::hash::Hash;
use std::any::TypeId;

/// The type of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    /// An value indicating nothing.
    Unit,
    /// A string.
    String,
    /// An vector of dynamic values.
    Vec,
    /// An tuple of dynamic values.
    Tuple,
    /// An object of dynamic values.
    Object,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Bool,
    /// A character.
    Char,
    /// Reference to a foreign type.
    External(TypeId),
    /// The type of type values.
    Type,
    /// A pointer to a value on the stack.
    Ptr,
    /// A function pointer.
    Fn(Hash),
    /// A future.
    Future,
    /// An optional value.
    Option,
    /// A result value.
    Result,
}

#[cfg(test)]
mod tests {
    use super::ValueType;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<ValueType>(),
            16,
        };
    }
}
