use core::borrow::Borrow;
use core::fmt;
use core::hash;
use core::mem::take;

use rust_alloc::sync::Arc;

use crate as rune;
use crate::alloc::prelude::*;
use crate::runtime::{OwnedTuple, TypeInfo};

use super::{FromValue, Mutable, OwnedRepr, Rtti, RuntimeError, Value};

/// A empty with a well-defined type.
#[derive(TryClone)]
#[try_clone(crate)]
pub struct EmptyStruct {
    /// The type hash of the empty.
    pub(crate) rtti: Arc<Rtti>,
}

impl EmptyStruct {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::typed(self.rtti.clone())
    }
}

impl FromValue for EmptyStruct {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        match value.take_repr()? {
            OwnedRepr::Inline(value) => Err(RuntimeError::expected_unit_struct(value.type_info())),
            OwnedRepr::Mutable(Mutable::EmptyStruct(value)) => Ok(value),
            OwnedRepr::Mutable(value) => Err(RuntimeError::expected_unit_struct(value.type_info())),
            OwnedRepr::Any(value) => Err(RuntimeError::expected_unit_struct(value.type_info())),
        }
    }
}

impl fmt::Debug for EmptyStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)
    }
}

/// A tuple with a well-defined type.
#[derive(TryClone)]
pub struct TupleStruct {
    /// The type hash of the tuple.
    pub(crate) rtti: Arc<Rtti>,
    /// Content of the tuple.
    pub(crate) data: OwnedTuple,
}

impl TupleStruct {
    /// Access runtime type information.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Access underlying data.
    pub fn data(&self) -> &OwnedTuple {
        &self.data
    }

    /// Access underlying data mutably.
    pub fn data_mut(&mut self) -> &mut OwnedTuple {
        &mut self.data
    }

    /// Get type info for the typed tuple.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::typed(self.rtti.clone())
    }

    /// Get the value at the given index in the tuple.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.data.get(index)
    }

    /// Get the mutable value at the given index in the tuple.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.data.get_mut(index)
    }
}

impl FromValue for TupleStruct {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        match value.take_repr()? {
            OwnedRepr::Inline(value) => Err(RuntimeError::expected_tuple_struct(value.type_info())),
            OwnedRepr::Mutable(Mutable::TupleStruct(value)) => Ok(value),
            OwnedRepr::Mutable(value) => {
                Err(RuntimeError::expected_tuple_struct(value.type_info()))
            }
            OwnedRepr::Any(value) => Err(RuntimeError::expected_tuple_struct(value.type_info())),
        }
    }
}

impl fmt::Debug for TupleStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:?}", self.rtti.item, self.data)
    }
}

/// An object with a well-defined type.
#[derive(TryClone)]
pub struct Struct {
    /// The type hash of the object.
    pub(crate) rtti: Arc<Rtti>,
    /// Contents of the object.
    pub(crate) data: Box<[Value]>,
}

impl Struct {
    /// Access struct rtti.
    pub fn rtti(&self) -> &Arc<Rtti> {
        &self.rtti
    }

    /// Access truct data.
    pub fn data(&self) -> &[Value] {
        &self.data
    }

    /// Access struct data mutably.
    pub fn data_mut(&mut self) -> &mut [Value] {
        &mut self.data
    }

    /// Get a field through the accessor.
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        Box<str>: Borrow<Q>,
        Q: hash::Hash + Eq + ?Sized,
    {
        self.data.get(*self.rtti.fields.get(key)?)
    }

    /// Get a field through the accessor.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
    where
        Box<str>: Borrow<Q>,
        Q: hash::Hash + Eq + ?Sized,
    {
        self.data.get_mut(*self.rtti.fields.get(key)?)
    }

    /// Get type info for the typed object.
    pub(crate) fn type_info(&self) -> TypeInfo {
        TypeInfo::typed(self.rtti.clone())
    }
}

impl FromValue for Struct {
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        match value.take_repr()? {
            OwnedRepr::Inline(value) => Err(RuntimeError::expected_struct(value.type_info())),
            OwnedRepr::Mutable(Mutable::Struct(value)) => Ok(value),
            OwnedRepr::Mutable(value) => Err(RuntimeError::expected_struct(value.type_info())),
            OwnedRepr::Any(value) => Err(RuntimeError::expected_struct(value.type_info())),
        }
    }
}

impl fmt::Debug for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {{", self.rtti.item)?;

        let mut first = true;

        for (index, field) in self.data.iter().enumerate() {
            let Some((name, _)) = self.rtti.fields.iter().find(|t| *t.1 == index) else {
                continue;
            };

            if !take(&mut first) {
                write!(f, ", ")?;
            }

            write!(f, "{name}: {field:?}")?;
        }

        write!(f, "}}")?;
        Ok(())
    }
}
