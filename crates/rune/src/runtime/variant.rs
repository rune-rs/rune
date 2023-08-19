use core::cmp::Ordering;
use core::fmt;

use crate::no_std::sync::Arc;
use crate::runtime::{Object, ProtocolCaller, Tuple, TypeInfo, VariantRtti, VmResult};

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

    pub(crate) fn partial_eq_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<bool> {
        debug_assert_eq!(
            a.rtti.enum_hash, b.rtti.enum_hash,
            "comparison only makes sense if enum hashes match"
        );

        if a.rtti.hash != b.rtti.hash {
            return VmResult::Ok(false);
        }

        match (&a.data, &b.data) {
            (VariantData::Unit, VariantData::Unit) => VmResult::Ok(true),
            (VariantData::Tuple(a), VariantData::Tuple(b)) => Tuple::partial_eq_with(a, b, caller),
            (VariantData::Struct(a), VariantData::Struct(b)) => {
                Object::partial_eq_with(a, b, caller)
            }
            _ => VmResult::panic("data mismatch between variants"),
        }
    }

    pub(crate) fn eq_with(a: &Self, b: &Self, caller: &mut impl ProtocolCaller) -> VmResult<bool> {
        debug_assert_eq!(
            a.rtti.enum_hash, b.rtti.enum_hash,
            "comparison only makes sense if enum hashes match"
        );

        if a.rtti.hash != b.rtti.hash {
            return VmResult::Ok(false);
        }

        match (&a.data, &b.data) {
            (VariantData::Unit, VariantData::Unit) => VmResult::Ok(true),
            (VariantData::Tuple(a), VariantData::Tuple(b)) => Tuple::eq_with(a, b, caller),
            (VariantData::Struct(a), VariantData::Struct(b)) => Object::eq_with(a, b, caller),
            _ => VmResult::panic("data mismatch between variants"),
        }
    }

    pub(crate) fn cmp_with(
        a: &Self,
        b: &Self,
        caller: &mut impl ProtocolCaller,
    ) -> VmResult<Ordering> {
        debug_assert_eq!(
            a.rtti.enum_hash, b.rtti.enum_hash,
            "comparison only makes sense if enum hashes match"
        );

        match a.rtti.hash.cmp(&b.rtti.hash) {
            Ordering::Equal => {}
            ordering => return VmResult::Ok(ordering),
        }

        match (&a.data, &b.data) {
            (VariantData::Unit, VariantData::Unit) => VmResult::Ok(Ordering::Equal),
            (VariantData::Tuple(a), VariantData::Tuple(b)) => Tuple::cmp_with(a, b, caller),
            (VariantData::Struct(a), VariantData::Struct(b)) => Object::cmp_with(a, b, caller),
            _ => VmResult::panic("data mismatch between variants"),
        }
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
