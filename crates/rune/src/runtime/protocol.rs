use crate::compile::meta;
use crate::module::{AssociatedFunctionName, ToInstance};
use crate::Hash;

#[doc(inline)]
pub use rune_core::Protocol;

impl ToInstance for Protocol {
    #[inline]
    fn to_instance(self) -> AssociatedFunctionName {
        AssociatedFunctionName {
            kind: meta::AssociatedKind::Protocol(self),
            parameters: Hash::EMPTY,
            #[cfg(feature = "doc")]
            parameter_types: vec![],
        }
    }
}
