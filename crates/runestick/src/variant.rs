use crate::{Object, Tuple, TypeInfo, VariantRtti, Vm, VmError};
use std::fmt;
use std::sync::Arc;

/// The variant of a type.
pub struct Variant {
    pub(crate) rtti: Arc<VariantRtti>,
    pub(crate) data: VariantData,
}

impl Variant {
    /// Construct a unit variant.
    pub fn unit(rtti: Arc<VariantRtti>) -> Self {
        Self {
            rtti,
            data: VariantData::Unit,
        }
    }

    /// Construct a tuple variant.
    pub fn tuple(rtti: Arc<VariantRtti>, tuple: Tuple) -> Self {
        Self {
            rtti,
            data: VariantData::Tuple(tuple),
        }
    }

    /// Construct a struct variant.
    pub fn struct_(rtti: Arc<VariantRtti>, data: Object) -> Self {
        Self {
            rtti,
            data: VariantData::Struct(data),
        }
    }

    /// Access the rtti of the variant.
    pub fn rtti(&self) -> &VariantRtti {
        &self.rtti
    }

    /// Access the underlying variant data.
    pub fn data(&self) -> &VariantData {
        &self.data
    }

    /// Access the underlying variant data mutably.
    pub fn data_mut(&mut self) -> &mut VariantData {
        &mut self.data
    }

    /// Get type info for the variant.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Variant(self.rtti.clone())
    }

    /// Perform a deep value comparison of two variants.
    pub(crate) fn value_ptr_eq(vm: &mut Vm, a: &Self, b: &Self) -> Result<bool, VmError> {
        debug_assert_eq!(
            a.rtti.enum_hash, b.rtti.enum_hash,
            "comparison only makes sense if enum hashes match"
        );

        if a.rtti.hash != b.rtti.hash {
            return Ok(false);
        }

        Ok(match (&a.data, &b.data) {
            (VariantData::Unit, VariantData::Unit) => true,
            (VariantData::Tuple(a), VariantData::Tuple(b)) => return Tuple::value_ptr_eq(vm, a, b),
            (VariantData::Struct(a), VariantData::Struct(b)) => {
                return Object::value_ptr_eq(vm, a, b)
            }
            _ => false,
        })
    }
}

/// The data of the variant.
pub enum VariantData {
    /// A unit variant.
    Unit,
    /// A struct variant.
    Struct(Object),
    /// A tuple variant.
    Tuple(Tuple),
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rtti.item)?;

        match &self.data {
            VariantData::Unit => {}
            VariantData::Struct(st) => {
                write!(f, "{:?}", st)?;
            }
            VariantData::Tuple(tuple) => {
                write!(f, "{:?}", tuple)?;
            }
        }

        Ok(())
    }
}
