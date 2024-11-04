use core::borrow::Borrow;
use core::hash::Hash;
use core::ops::Deref;

use crate::alloc::{Box, Vec};

use super::{FromValue, Repr, Rtti, RttiKind, RuntimeError, Tuple, TypeInfo, Value};

use rust_alloc::sync::Arc;

/// A reference to a dynamically defined empty type.
pub struct DynamicEmpty {
    rtti: Arc<Rtti>,
}

impl FromValue for DynamicEmpty {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        match value.take_repr() {
            Repr::Dynamic(value) if matches!(value.rtti().kind, RttiKind::Empty) => Ok(Self {
                rtti: value.rtti().clone(),
            }),
            value => Err(RuntimeError::expected_empty(value.type_info())),
        }
    }
}

impl DynamicEmpty {
    /// Get human readable type information for the dynamic tuple.
    pub fn type_info(&self) -> TypeInfo {
        self.rtti.clone().type_info()
    }
}

/// A reference to a dynamically defined tuple.
///
/// This derefs into a [`Tuple`], which can be used to access the individual
/// fields.
pub struct DynamicTuple {
    rtti: Arc<Rtti>,
    values: Vec<Value>,
}

impl FromValue for DynamicTuple {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        match value.take_repr() {
            Repr::Dynamic(value) if matches!(value.rtti().kind, RttiKind::Tuple) => {
                let mut values = Vec::try_with_capacity(value.len())?;

                for value in value.borrow_ref()?.iter() {
                    values.try_push(value.clone())?;
                }

                Ok(Self {
                    rtti: value.rtti().clone(),
                    values,
                })
            }
            value => Err(RuntimeError::expected_tuple(value.type_info())),
        }
    }
}

impl DynamicTuple {
    /// Get human readable type information for the dynamic tuple.
    pub fn type_info(&self) -> TypeInfo {
        self.rtti.clone().type_info()
    }
}

impl Deref for DynamicTuple {
    type Target = Tuple;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Tuple::new(&self.values)
    }
}

/// A reference to a dynamically defined struct.
pub struct DynamicStruct {
    rtti: Arc<Rtti>,
    values: Vec<Value>,
}

impl FromValue for DynamicStruct {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        match value.take_repr() {
            Repr::Dynamic(value) if matches!(value.rtti().kind, RttiKind::Struct) => {
                let mut values = Vec::try_with_capacity(value.len())?;

                for value in value.borrow_ref()?.iter() {
                    values.try_push(value.clone())?;
                }

                Ok(Self {
                    rtti: value.rtti().clone(),
                    values,
                })
            }
            value => Err(RuntimeError::expected_struct(value.type_info())),
        }
    }
}

impl DynamicStruct {
    /// Get a value from the dynamic struct.
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        Box<str>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let index = *self.rtti.fields.get(key)?;
        self.values.get(index)
    }

    /// Get human readable type information for the dynamic struct.
    pub fn type_info(&self) -> TypeInfo {
        self.rtti.clone().type_info()
    }
}
