use crate::compile::{AssociatedFunctionName, ToFieldFunction, ToInstance};
use crate::hash::{Hash, IntoHash};
use crate::runtime::{FullTypeOf, Protocol};

/// Helper to register a parameterized function.
///
/// This is used to wrap the name of the function in order to associated
/// parameters with it.
#[derive(Clone)]
pub struct Params<T, const N: usize> {
    pub(crate) name: T,
    pub(crate) parameters: [FullTypeOf; N],
}

impl<T, const N: usize> Params<T, N> {
    /// Construct a new parameters wrapper.
    pub const fn new(name: T, parameters: [FullTypeOf; N]) -> Self {
        Self { name, parameters }
    }
}

impl<T, const N: usize> IntoHash for Params<T, N>
where
    T: IntoHash,
{
    #[inline]
    fn into_hash(self) -> Hash {
        self.name.into_hash()
    }
}

impl<T, const N: usize> ToInstance for Params<T, N>
where
    T: ToInstance,
{
    #[inline]
    fn to_instance(self) -> AssociatedFunctionName {
        let info = self.name.to_instance();

        AssociatedFunctionName {
            kind: info.kind,
            parameters: Hash::parameters(self.parameters.iter().map(|t| t.hash)),
            #[cfg(feature = "doc")]
            parameter_type_infos: self
                .parameters
                .iter()
                .map(|t| t.type_info.clone())
                .collect(),
        }
    }
}

impl<T, const N: usize> ToFieldFunction for Params<T, N>
where
    T: ToFieldFunction,
{
    #[inline]
    fn to_field_function(self, protocol: Protocol) -> AssociatedFunctionName {
        let info = self.name.to_field_function(protocol);

        AssociatedFunctionName {
            kind: info.kind,
            parameters: Hash::parameters(self.parameters.iter().map(|p| p.hash)),
            #[cfg(feature = "doc")]
            parameter_type_infos: self
                .parameters
                .iter()
                .map(|p| p.type_info.clone())
                .collect(),
        }
    }
}
