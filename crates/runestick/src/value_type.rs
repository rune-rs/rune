use crate::{Hash, StaticType};
use std::any::TypeId;
use std::cmp;
use std::fmt;
use std::hash;

const HASH_TYPE: usize = 0;
const ANY_TYPE: usize = 1;

/// The type of an entry.
#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// The type hash of a type.
    Type(Hash),
    /// Reference to a foreign type.
    Any(TypeId),
}

impl cmp::PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        if let (Self::Any(a), Self::Any(b)) = (self, other) {
            return a == b;
        }

        let a = match self {
            Self::StaticType(ty) => ty.hash,
            Self::Type(hash) => *hash,
            _ => return false,
        };

        let b = match self {
            Self::StaticType(ty) => ty.hash,
            Self::Type(hash) => *hash,
            _ => return false,
        };

        a == b
    }
}

impl cmp::Eq for ValueType {}

impl hash::Hash for ValueType {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::StaticType(ty) => {
                HASH_TYPE.hash(state);
                ty.hash.hash(state)
            }
            Self::Type(hash) => {
                HASH_TYPE.hash(state);
                hash.hash(state)
            }
            Self::Any(type_id) => {
                ANY_TYPE.hash(state);
                type_id.hash(state);
            }
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticType(ty) => write!(f, "type({})", ty.name),
            Self::Type(hash) => write!(f, "type({})", hash),
            Self::Any(type_id) => write!(f, "any({:?})", type_id),
        }
    }
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
