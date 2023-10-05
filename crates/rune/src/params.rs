use crate::alloc;
#[cfg(feature = "doc")]
use crate::alloc::prelude::*;
use crate::hash::Hash;
use crate::module::{AssociatedName, ToFieldFunction, ToInstance};
use crate::runtime::Protocol;

#[doc(inline)]
pub use rune_core::Params;

impl<T, const N: usize> ToInstance for Params<T, N>
where
    T: ToInstance,
{
    #[inline]
    fn to_instance(self) -> alloc::Result<AssociatedName> {
        let info = self.name.to_instance()?;

        Ok(AssociatedName {
            kind: info.kind,
            function_parameters: Hash::parameters(self.parameters.iter().map(|t| t.hash)),
            #[cfg(feature = "doc")]
            parameter_types: self.parameters.iter().map(|t| t.hash).try_collect()?,
        })
    }
}

impl<T, const N: usize> ToFieldFunction for Params<T, N>
where
    T: ToFieldFunction,
{
    #[inline]
    fn to_field_function(self, protocol: Protocol) -> alloc::Result<AssociatedName> {
        let info = self.name.to_field_function(protocol)?;

        Ok(AssociatedName {
            kind: info.kind,
            function_parameters: Hash::parameters(self.parameters.iter().map(|p| p.hash)),
            #[cfg(feature = "doc")]
            parameter_types: self.parameters.iter().map(|p| p.hash).try_collect()?,
        })
    }
}
