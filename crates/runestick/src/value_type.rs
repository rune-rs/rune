use crate::{Hash, StaticType};
use std::cmp;
use std::fmt;
use std::hash;

/// The type of an entry.
#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// The type hash of a type.
    Type(Hash),
}

impl ValueType {
    /// Treat the value type as a type hash.
    pub fn as_type_hash(&self) -> Hash {
        match self {
            Self::StaticType(ty) => ty.hash,
            Self::Type(hash) => *hash,
        }
    }
}

impl cmp::PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::StaticType(a), b) => match b {
                Self::StaticType(b) => a.eq(b),
                Self::Type(b) => &a.hash == b,
            },
            (Self::Type(a), b) => match b {
                Self::StaticType(b) => a == &b.hash,
                Self::Type(b) => a == b,
            },
        }
    }
}

impl cmp::PartialEq<Hash> for ValueType {
    fn eq(&self, other: &Hash) -> bool {
        match self {
            Self::StaticType(a) => &a.hash == other,
            Self::Type(hash) => hash == other,
        }
    }
}

impl cmp::Eq for ValueType {}

impl hash::Hash for ValueType {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::StaticType(ty) => ty.hash.hash(state),
            Self::Type(hash) => hash.hash(state),
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticType(ty) => write!(f, "type({})", ty.name),
            Self::Type(hash) => write!(f, "type({})", hash),
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
