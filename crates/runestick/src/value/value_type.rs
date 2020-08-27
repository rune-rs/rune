use crate::hash::Hash;
use std::any::TypeId;

/// The type of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueType {
    /// An value indicating nothing.
    Unit,
    /// A boolean.
    Bool,
    /// A character.
    Char,
    /// A byte.
    Byte,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A byte array.
    Bytes,
    /// A string.
    String,
    /// An vector of dynamic values.
    Vec,
    /// An tuple of dynamic values.
    Tuple,
    /// An object of dynamic values.
    Object,
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
    /// A typed tuple.
    TypedTuple {
        /// The type hash corresponding to the type.
        hash: Hash,
    },
    /// A typed tuple variant.
    VariantTuple {
        /// The type hash of the enum the variant belongs to.
        enum_hash: Hash,
        /// The type hash of the variant.
        hash: Hash,
    },
    /// A typed object.
    TypedObject {
        /// The type hash corresponding to the type.
        hash: Hash,
    },
    /// A typed object variant.
    VariantObject {
        /// The type hash of the enum the variant belongs to.
        enum_hash: Hash,
        /// The type hash of the variant.
        hash: Hash,
    },
}

#[cfg(test)]
mod tests {
    use super::ValueType;

    #[test]
    fn test_size() {
        assert_eq! {
            std::mem::size_of::<ValueType>(),
            24,
        };
    }
}
