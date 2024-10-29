use core::cmp::Ordering;
use core::fmt;

use musli::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::hash::Hash;
use crate::runtime::{static_type, Protocol, Type, TypeInfo, VmErrorKind, VmResult};

use super::err;

/// An inline value.
#[derive(Clone, Copy, Encode, Decode, Deserialize, Serialize)]
pub enum Inline {
    /// The unit value.
    Unit,
    /// A boolean.
    Bool(bool),
    /// A single byte.
    Byte(u8),
    /// A character.
    Char(char),
    /// A number.
    Integer(i64),
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
}

impl Inline {
    /// Perform a partial equality check over two inline values.
    pub(crate) fn partial_eq(&self, other: &Self) -> VmResult<bool> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(true),
            (Inline::Bool(a), Inline::Bool(b)) => VmResult::Ok(*a == *b),
            (Inline::Byte(a), Inline::Byte(b)) => VmResult::Ok(*a == *b),
            (Inline::Char(a), Inline::Char(b)) => VmResult::Ok(*a == *b),
            (Inline::Integer(a), Inline::Integer(b)) => VmResult::Ok(*a == *b),
            (Inline::Float(a), Inline::Float(b)) => VmResult::Ok(*a == *b),
            (Inline::Type(a), Inline::Type(b)) => VmResult::Ok(*a == *b),
            (Inline::Ordering(a), Inline::Ordering(b)) => VmResult::Ok(*a == *b),
            (lhs, rhs) => err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::PARTIAL_EQ.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }

    /// Perform a total equality check over two inline values.
    pub(crate) fn eq(&self, other: &Self) -> VmResult<bool> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(true),
            (Inline::Bool(a), Inline::Bool(b)) => VmResult::Ok(*a == *b),
            (Inline::Byte(a), Inline::Byte(b)) => VmResult::Ok(*a == *b),
            (Inline::Char(a), Inline::Char(b)) => VmResult::Ok(*a == *b),
            (Inline::Float(a), Inline::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
                };

                VmResult::Ok(matches!(ordering, Ordering::Equal))
            }
            (Inline::Integer(a), Inline::Integer(b)) => VmResult::Ok(*a == *b),
            (Inline::Type(a), Inline::Type(b)) => VmResult::Ok(*a == *b),
            (Inline::Ordering(a), Inline::Ordering(b)) => VmResult::Ok(*a == *b),
            (lhs, rhs) => err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::EQ.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }

    /// Partial comparison implementation for inline.
    pub(crate) fn partial_cmp(&self, other: &Self) -> VmResult<Option<Ordering>> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(Some(Ordering::Equal)),
            (Inline::Bool(lhs), Inline::Bool(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Byte(lhs), Inline::Byte(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Char(lhs), Inline::Char(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Float(lhs), Inline::Float(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Integer(lhs), Inline::Integer(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Type(lhs), Inline::Type(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (Inline::Ordering(lhs), Inline::Ordering(rhs)) => VmResult::Ok(lhs.partial_cmp(rhs)),
            (lhs, rhs) => err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::PARTIAL_CMP.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }

    /// Total comparison implementation for inline.
    pub(crate) fn cmp(&self, other: &Self) -> VmResult<Ordering> {
        match (self, other) {
            (Inline::Unit, Inline::Unit) => VmResult::Ok(Ordering::Equal),
            (Inline::Bool(a), Inline::Bool(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Byte(a), Inline::Byte(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Char(a), Inline::Char(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Float(a), Inline::Float(b)) => {
                let Some(ordering) = a.partial_cmp(b) else {
                    return VmResult::err(VmErrorKind::IllegalFloatComparison { lhs: *a, rhs: *b });
                };

                VmResult::Ok(ordering)
            }
            (Inline::Integer(a), Inline::Integer(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Type(a), Inline::Type(b)) => VmResult::Ok(a.cmp(b)),
            (Inline::Ordering(a), Inline::Ordering(b)) => VmResult::Ok(a.cmp(b)),
            (lhs, rhs) => VmResult::err(VmErrorKind::UnsupportedBinaryOperation {
                op: Protocol::CMP.name,
                lhs: lhs.type_info(),
                rhs: rhs.type_info(),
            }),
        }
    }
}

impl fmt::Debug for Inline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Inline::Unit => write!(f, "()"),
            Inline::Bool(value) => value.fmt(f),
            Inline::Byte(value) => value.fmt(f),
            Inline::Char(value) => value.fmt(f),
            Inline::Integer(value) => value.fmt(f),
            Inline::Float(value) => value.fmt(f),
            Inline::Type(value) => value.fmt(f),
            Inline::Ordering(value) => value.fmt(f),
        }
    }
}

impl Inline {
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Inline::Unit => TypeInfo::static_type(static_type::TUPLE),
            Inline::Bool(..) => TypeInfo::static_type(static_type::BOOL),
            Inline::Byte(..) => TypeInfo::static_type(static_type::BYTE),
            Inline::Char(..) => TypeInfo::static_type(static_type::CHAR),
            Inline::Integer(..) => TypeInfo::static_type(static_type::INTEGER),
            Inline::Float(..) => TypeInfo::static_type(static_type::FLOAT),
            Inline::Type(..) => TypeInfo::static_type(static_type::TYPE),
            Inline::Ordering(..) => TypeInfo::static_type(static_type::ORDERING),
        }
    }

    /// Get the type hash for the current value.
    ///
    /// One notable feature is that the type of a variant is its container
    /// *enum*, and not the type hash of the variant itself.
    pub(crate) fn type_hash(&self) -> Hash {
        match self {
            Inline::Unit => static_type::TUPLE.hash,
            Inline::Bool(..) => static_type::BOOL.hash,
            Inline::Byte(..) => static_type::BYTE.hash,
            Inline::Char(..) => static_type::CHAR.hash,
            Inline::Integer(..) => static_type::INTEGER.hash,
            Inline::Float(..) => static_type::FLOAT.hash,
            Inline::Type(..) => static_type::TYPE.hash,
            Inline::Ordering(..) => static_type::ORDERING.hash,
        }
    }
}
