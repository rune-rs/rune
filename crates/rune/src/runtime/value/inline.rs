use core::any;
use core::cmp::Ordering;
use core::fmt;
use core::hash::Hash as _;

use musli::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate as rune;
use crate::runtime::{
    Hasher, OwnedTuple, Protocol, RuntimeError, Type, TypeInfo, VmErrorKind, VmIntegerRepr,
};
use crate::{Hash, TypeHash};

/// An inline value.
#[derive(Clone, Copy, Encode, Decode, Deserialize, Serialize)]
pub enum Inline {
    /// An empty value.
    ///
    /// Note that this value *can not* be instantiated. Internally any
    /// operations over it will result in a type error, even when operating with
    /// itself.
    ///
    /// Some operations will return a "falsy" value, like type checks.
    Empty,
    /// The unit value.
    Unit,
    /// A boolean.
    Bool(bool),
    /// A character.
    Char(char),
    /// A number.
    Signed(i64),
    /// An unsigned number.
    Unsigned(u64),
    /// A float.
    Float(f64),
    /// A type hash. Describes a type in the virtual machine.
    Type(Type),
    /// Ordering.
    Ordering(
        #[musli(with = crate::musli::ordering)]
        #[serde(with = "crate::serde::ordering")]
        Ordering,
    ),
    /// A type hash.
    Hash(Hash),
}

impl Inline {
    pub(crate) fn as_integer<T>(self) -> Result<T, RuntimeError>
    where
        T: TryFrom<u64> + TryFrom<i64>,
    {
        match self {
            Inline::Unsigned(value) => match value.try_into() {
                Ok(number) => Ok(number),
                Err(..) => Err(RuntimeError::new(
                    VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(value),
                        to: any::type_name::<T>(),
                    },
                )),
            },
            Inline::Signed(value) => match value.try_into() {
                Ok(number) => Ok(number),
                Err(..) => Err(RuntimeError::new(
                    VmErrorKind::ValueToIntegerCoercionError {
                        from: VmIntegerRepr::from(value),
                        to: any::type_name::<T>(),
                    },
                )),
            },
            ref value => Err(RuntimeError::new(VmErrorKind::ExpectedNumber {
                actual: value.type_info(),
            })),
        }
    }

    /// Perform a partial equality check over two inline values.
    pub(crate) fn partial_eq(&self, other: &Self) -> Result<bool, RuntimeError> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => Ok(true),
            (Inline::Bool(a), Inline::Bool(b)) => Ok(*a == *b),
            (Inline::Char(a), Inline::Char(b)) => Ok(*a == *b),
            (Inline::Signed(a), Inline::Signed(b)) => Ok(*a == *b),
            (Inline::Signed(a), rhs) => Ok(*a == rhs.as_integer::<i64>()?),
            (Inline::Unsigned(a), Inline::Unsigned(b)) => Ok(*a == *b),
            (Inline::Unsigned(a), rhs) => Ok(*a == rhs.as_integer::<u64>()?),
            (Inline::Float(a), Inline::Float(b)) => Ok(*a == *b),
            (Inline::Type(a), Inline::Type(b)) => Ok(*a == *b),
            (Inline::Ordering(a), Inline::Ordering(b)) => Ok(*a == *b),
            (Inline::Hash(a), Inline::Hash(b)) => Ok(*a == *b),
            (lhs, rhs) => Err(RuntimeError::from(
                VmErrorKind::UnsupportedBinaryOperation {
                    op: Protocol::PARTIAL_EQ.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                },
            )),
        }
    }

    /// Perform a total equality check over two inline values.
    pub(crate) fn eq(&self, other: &Self) -> Result<bool, RuntimeError> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => Ok(true),
            (Inline::Bool(a), Inline::Bool(b)) => Ok(*a == *b),
            (Inline::Char(a), Inline::Char(b)) => Ok(*a == *b),
            (Inline::Unsigned(a), Inline::Unsigned(b)) => Ok(*a == *b),
            (Inline::Signed(a), Inline::Signed(b)) => Ok(*a == *b),
            (Inline::Float(a), Inline::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return Err(RuntimeError::new(VmErrorKind::IllegalFloatComparison {
                        lhs: *a,
                        rhs: *b,
                    }));
                };

                Ok(matches!(ordering, Ordering::Equal))
            }
            (Inline::Type(a), Inline::Type(b)) => Ok(*a == *b),
            (Inline::Ordering(a), Inline::Ordering(b)) => Ok(*a == *b),
            (Inline::Hash(a), Inline::Hash(b)) => Ok(*a == *b),
            (lhs, rhs) => Err(RuntimeError::new(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::EQ.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            })),
        }
    }

    /// Partial comparison implementation for inline.
    pub(crate) fn partial_cmp(&self, other: &Self) -> Result<Option<Ordering>, RuntimeError> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => Ok(Some(Ordering::Equal)),
            (Inline::Bool(lhs), Inline::Bool(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (Inline::Char(lhs), Inline::Char(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (Inline::Unsigned(lhs), Inline::Unsigned(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (Inline::Unsigned(lhs), rhs) => {
                let rhs = rhs.as_integer::<u64>()?;
                Ok(lhs.partial_cmp(&rhs))
            }
            (Inline::Signed(lhs), Inline::Signed(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (Inline::Signed(lhs), rhs) => {
                let rhs = rhs.as_integer::<i64>()?;
                Ok(lhs.partial_cmp(&rhs))
            }
            (Inline::Float(lhs), Inline::Float(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (Inline::Type(lhs), Inline::Type(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (Inline::Ordering(lhs), Inline::Ordering(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (Inline::Hash(lhs), Inline::Hash(rhs)) => Ok(lhs.partial_cmp(rhs)),
            (lhs, rhs) => Err(RuntimeError::from(
                VmErrorKind::UnsupportedBinaryOperation {
                    op: Protocol::PARTIAL_CMP.name,
                    lhs: lhs.type_info(),
                    rhs: rhs.type_info(),
                },
            )),
        }
    }

    /// Total comparison implementation for inline.
    pub(crate) fn cmp(&self, other: &Self) -> Result<Ordering, RuntimeError> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => Ok(Ordering::Equal),
            (Inline::Bool(a), Inline::Bool(b)) => Ok(a.cmp(b)),
            (Inline::Char(a), Inline::Char(b)) => Ok(a.cmp(b)),
            (Inline::Unsigned(a), Inline::Unsigned(b)) => Ok(a.cmp(b)),
            (Inline::Signed(a), Inline::Signed(b)) => Ok(a.cmp(b)),
            (Inline::Float(a), Inline::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return Err(RuntimeError::new(VmErrorKind::IllegalFloatComparison {
                        lhs: *a,
                        rhs: *b,
                    }));
                };

                Ok(ordering)
            }
            (Inline::Type(a), Inline::Type(b)) => Ok(a.cmp(b)),
            (Inline::Ordering(a), Inline::Ordering(b)) => Ok(a.cmp(b)),
            (Inline::Hash(a), Inline::Hash(b)) => Ok(a.cmp(b)),
            (lhs, rhs) => Err(RuntimeError::new(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::CMP.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            })),
        }
    }

    /// Hash an inline value.
    pub(crate) fn hash(&self, hasher: &mut Hasher) -> Result<(), RuntimeError> {
        match self {
            Inline::Unsigned(value) => {
                value.hash(hasher);
            }
            Inline::Signed(value) => {
                value.hash(hasher);
            }
            // Care must be taken whan hashing floats, to ensure that `hash(v1)
            // === hash(v2)` if `eq(v1) === eq(v2)`. Hopefully we accomplish
            // this by rejecting NaNs and rectifying subnormal values of zero.
            Inline::Float(value) => {
                if value.is_nan() {
                    return Err(RuntimeError::new(VmErrorKind::IllegalFloatOperation {
                        value: *value,
                    }));
                }

                let zero = *value == 0.0;
                let value = ((zero as u8 as f64) * 0.0 + (!zero as u8 as f64) * *value).to_bits();
                value.hash(hasher);
            }
            operand => {
                return Err(RuntimeError::new(VmErrorKind::UnsupportedUnaryOperation {
                    op: Protocol::HASH.name,
                    operand: operand.type_info(),
                }));
            }
        }

        Ok(())
    }
}

impl fmt::Debug for Inline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Inline::Empty => write!(f, "<empty>"),
            Inline::Unit => write!(f, "()"),
            Inline::Bool(value) => value.fmt(f),
            Inline::Char(value) => value.fmt(f),
            Inline::Unsigned(value) => value.fmt(f),
            Inline::Signed(value) => value.fmt(f),
            Inline::Float(value) => value.fmt(f),
            Inline::Type(value) => value.fmt(f),
            Inline::Ordering(value) => value.fmt(f),
            Inline::Hash(value) => value.fmt(f),
        }
    }
}

impl Inline {
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Inline::Empty => TypeInfo::empty(),
            Inline::Unit => TypeInfo::any::<OwnedTuple>(),
            Inline::Bool(..) => TypeInfo::named::<bool>(),
            Inline::Char(..) => TypeInfo::named::<char>(),
            Inline::Unsigned(..) => TypeInfo::named::<u64>(),
            Inline::Signed(..) => TypeInfo::named::<i64>(),
            Inline::Float(..) => TypeInfo::named::<f64>(),
            Inline::Type(..) => TypeInfo::named::<Type>(),
            Inline::Ordering(..) => TypeInfo::named::<Ordering>(),
            Inline::Hash(..) => TypeInfo::named::<Hash>(),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub(crate) fn type_hash(&self) -> Hash {
        match self {
            Inline::Empty => crate::hash!(::std::empty::Empty),
            Inline::Unit => OwnedTuple::HASH,
            Inline::Bool(..) => bool::HASH,
            Inline::Char(..) => char::HASH,
            Inline::Signed(..) => i64::HASH,
            Inline::Unsigned(..) => u64::HASH,
            Inline::Float(..) => f64::HASH,
            Inline::Type(..) => Type::HASH,
            Inline::Ordering(..) => Ordering::HASH,
            Inline::Hash(..) => Hash::HASH,
        }
    }
}
