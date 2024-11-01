use ::rust_alloc::sync::Arc;

use crate::alloc::{self, Vec};
use crate::compile::Docs;
use crate::function::{Function, Plain};
use crate::runtime::{FunctionHandler, StaticType, TypeCheck};

use super::{Fields, ItemMut, Variant};

/// Specialized information on `GeneratorState` types.
pub(crate) struct InternalEnum {
    /// The name of the internal enum.
    pub(crate) name: &'static str,
    /// The static type of the enum.
    pub(crate) static_type: StaticType,
    /// Internal variants.
    pub(crate) variants: Vec<Variant>,
}

impl InternalEnum {
    /// Construct a new handler for an internal enum.
    pub(super) fn new(name: &'static str, static_type: StaticType) -> Self {
        InternalEnum {
            name,
            static_type,
            variants: Vec::new(),
        }
    }

    /// Register a new variant.
    pub(super) fn variant<C, A>(
        &mut self,
        name: &'static str,
        type_check: TypeCheck,
        constructor: C,
    ) -> alloc::Result<ItemMut<'_>>
    where
        C: Function<A, Plain>,
    {
        let constructor: Arc<FunctionHandler> = Arc::new(move |stack, addr, args, output| {
            constructor.fn_call(stack, addr, args, output)
        });

        self.variants.try_push(Variant {
            name,
            type_check: Some(type_check),
            fields: Some(Fields::Unnamed(C::ARGS)),
            constructor: Some(constructor),
            deprecated: None,
            docs: Docs::EMPTY,
        })?;

        let v = self.variants.last_mut().unwrap();

        Ok(ItemMut {
            docs: &mut v.docs,
            #[cfg(feature = "doc")]
            deprecated: &mut v.deprecated,
        })
    }
}
