use core::borrow::Borrow;
use core::fmt;
use core::hash;

use rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::runtime::{BorrowRef, TypeInfo};

use super::{Rtti, Value};

/// A empty with a well-defined type.
pub struct EmptyStruct<'a> {
    /// The type hash of the empty.
    pub(crate) rtti: &'a Arc<Rtti>,
}

impl<'a> EmptyStruct<'a> {
    /// Access runtime type information.
    pub fn rtti(&self) -> &'a Arc<Rtti> {
        self.rtti
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::rtti(self.rtti.clone())
    }
}

impl fmt::Debug for EmptyStruct<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)
    }
}

/// A tuple with a well-defined type.
pub struct TupleStruct<'a> {
    /// The type hash of the tuple.
    pub(crate) rtti: &'a Arc<Rtti>,
    /// Content of the tuple.
    pub(crate) data: BorrowRef<'a, [Value]>,
}

impl<'a> TupleStruct<'a> {
    /// Access runtime type information.
    pub fn rtti(&self) -> &'a Rtti {
        self.rtti
    }

    /// Access underlying data.
    pub fn data(&self) -> &[Value] {
        &self.data
    }

    /// Get the value at the given index in the tuple.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.data.get(index)
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::rtti(self.rtti.clone())
    }
}

impl fmt::Debug for TupleStruct<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)
    }
}

/// An object with a well-defined type.
pub struct Struct<'a> {
    /// The type hash of the object.
    pub(crate) rtti: &'a Arc<Rtti>,
    /// Contents of the object.
    pub(crate) data: BorrowRef<'a, [Value]>,
}

impl<'a> Struct<'a> {
    /// Access struct rtti.
    pub fn rtti(&self) -> &'a Arc<Rtti> {
        self.rtti
    }

    /// Access truct data.
    pub fn data(&self) -> &[Value] {
        &self.data
    }

    /// Get a field through the accessor.
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        Box<str>: Borrow<Q>,
        Q: hash::Hash + Eq + ?Sized,
    {
        self.data.get(*self.rtti.fields.get(key)?)
    }

    /// Get type info for the typed object.
    pub(crate) fn type_info(&self) -> TypeInfo {
        TypeInfo::rtti(self.rtti.clone())
    }
}

impl fmt::Debug for Struct<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)
    }
}
