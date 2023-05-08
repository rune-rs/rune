//! Types used for defining native modules.
//!
//! A native module is one that provides rune with functions and types through
//! native Rust-based code.

mod function_meta;
mod function_traits;
pub(crate) mod module;

use core::fmt;

use crate::no_std::collections::HashSet;
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::compile::{ContextError, Docs, IntoComponent, ItemBuf};
use crate::runtime::{FullTypeOf, FunctionHandler, MacroHandler, StaticType, TypeCheck, TypeInfo};
use crate::Hash;

pub(crate) use self::function_meta::{
    AssociatedFunctionName, AssociatedKind, ToFieldFunction, ToInstance,
};

#[doc(hidden)]
pub use self::function_meta::{FunctionMetaData, FunctionMetaKind, MacroMetaData, MacroMetaKind};
pub use self::function_traits::{AssocType, AsyncFunction, AsyncInstFn, Function, InstFn};
#[doc(hidden)]
pub use self::module::Module;

/// Trait to handle the installation of auxilliary functions for a type
/// installed into a module.
pub trait InstallWith {
    /// Hook to install more things into the module.
    fn install_with(_: &mut Module) -> Result<(), ContextError> {
        Ok(())
    }
}

/// Specialized information on `Option` types.
pub(crate) struct UnitType {
    /// Item of the unit type.
    pub(crate) name: Box<str>,
}

/// Specialized information on `GeneratorState` types.
pub(crate) struct InternalEnum {
    /// The name of the internal enum.
    pub(crate) name: &'static str,
    /// The result type.
    pub(crate) base_type: ItemBuf,
    /// The static type of the enum.
    pub(crate) static_type: &'static StaticType,
    /// Internal variants.
    pub(crate) variants: Vec<InternalVariant>,
}

impl InternalEnum {
    /// Construct a new handler for an internal enum.
    fn new<N>(name: &'static str, base_type: N, static_type: &'static StaticType) -> Self
    where
        N: IntoIterator,
        N::Item: IntoComponent,
    {
        InternalEnum {
            name,
            base_type: ItemBuf::with_item(base_type),
            static_type,
            variants: Vec::new(),
        }
    }

    /// Register a new variant.
    fn variant<C, A>(&mut self, name: &'static str, type_check: TypeCheck, constructor: C)
    where
        C: Function<A>,
    {
        let constructor: Arc<FunctionHandler> =
            Arc::new(move |stack, args| constructor.fn_call(stack, args));

        self.variants.push(InternalVariant {
            name,
            type_check,
            args: C::args(),
            constructor,
        });
    }
}

/// Internal variant.
pub(crate) struct InternalVariant {
    /// The name of the variant.
    pub(crate) name: &'static str,
    /// Type check for the variant.
    pub(crate) type_check: TypeCheck,
    /// Arguments for the variant.
    pub(crate) args: usize,
    /// The constructor of the variant.
    pub(crate) constructor: Arc<FunctionHandler>,
}

/// Data for an opaque type. If `spec` is set, indicates things which are known
/// about that type.
pub(crate) struct Type {
    /// The name of the installed type which will be the final component in the
    /// item it will constitute.
    pub(crate) name: Box<str>,
    /// Type information for the installed type.
    pub(crate) type_info: TypeInfo,
    /// The specification for the type.
    pub(crate) spec: Option<TypeSpecification>,
    /// Documentation for the type.
    pub(crate) docs: Docs,
}

/// Metadata about a variant.
pub struct Variant {
    /// Variant metadata.
    pub(crate) kind: VariantKind,
    /// Handler to use if this variant can be constructed through a regular function call.
    pub(crate) constructor: Option<Arc<FunctionHandler>>,
}

impl fmt::Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Variant")
            .field("kind", &self.kind)
            .field("constructor", &self.constructor.is_some())
            .finish()
    }
}

/// The kind of the variant.
#[derive(Debug)]
pub(crate) enum VariantKind {
    /// Variant is a Tuple variant.
    Tuple(Tuple),
    /// Variant is a Struct variant.
    Struct(Struct),
    /// Variant is a Unit variant.
    Unit,
}

impl Variant {
    /// Construct metadata for a tuple variant.
    #[inline]
    pub fn tuple(args: usize) -> Self {
        Self {
            kind: VariantKind::Tuple(Tuple { args }),
            constructor: None,
        }
    }

    /// Construct metadata for a tuple variant.
    #[inline]
    pub fn st<const N: usize>(fields: [&'static str; N]) -> Self {
        Self {
            kind: VariantKind::Struct(Struct {
                fields: fields.into_iter().map(Box::<str>::from).collect(),
            }),
            constructor: None,
        }
    }

    /// Construct metadata for a unit variant.
    #[inline]
    pub fn unit() -> Self {
        Self {
            kind: VariantKind::Unit,
            constructor: None,
        }
    }
}

/// Metadata about a tuple or tuple variant.
#[derive(Debug)]
pub struct Tuple {
    /// The number of fields.
    pub(crate) args: usize,
}

/// The type specification for a native struct.
#[derive(Debug)]
pub(crate) struct Struct {
    /// The names of the struct fields known at compile time.
    pub(crate) fields: HashSet<Box<str>>,
}

/// The type specification for a native enum.
pub(crate) struct Enum {
    /// The variants.
    pub(crate) variants: Vec<(Box<str>, Variant)>,
}

/// A type specification.
pub(crate) enum TypeSpecification {
    Struct(Struct),
    Enum(Enum),
}

/// A key that identifies an associated function.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub(crate) struct AssociatedKey {
    /// The type the associated function belongs to.
    pub(crate) type_hash: Hash,
    /// The kind of the associated function.
    pub(crate) kind: AssociatedKind,
    /// The type parameters of the associated function.
    pub(crate) parameters: Hash,
}

#[derive(Clone)]
pub(crate) struct ModuleFunction {
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) is_async: bool,
    pub(crate) args: Option<usize>,
    pub(crate) return_type: Option<FullTypeOf>,
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
    #[cfg_attr(not(feature = "doc"), allow(unused))]
    pub(crate) associated_container: Option<Hash>,
    pub(crate) docs: Docs,
}

#[derive(Clone)]
pub(crate) struct ModuleAssociated {
    pub(crate) name: AssociatedFunctionName,
    pub(crate) type_info: TypeInfo,
    pub(crate) handler: Arc<FunctionHandler>,
    pub(crate) is_async: bool,
    pub(crate) args: Option<usize>,
    pub(crate) return_type: Option<FullTypeOf>,
    pub(crate) argument_types: Box<[Option<FullTypeOf>]>,
    pub(crate) docs: Docs,
}

/// Handle to a macro inserted into a module.
pub(crate) struct ModuleMacro {
    pub(crate) handler: Arc<MacroHandler>,
    pub(crate) docs: Docs,
}

/// Handle to a an item inserted into a module which allows for mutation of item
/// metadata.
///
/// This is returned by methods which insert meta items, such as:
/// * [`Module::raw_fn`].
/// * [`Module::function`].
/// * [`Module::async_function`].
/// * [`Module::inst_fn`].
/// * [`Module::async_inst_fn`].
///
/// While this is also returned by `*_meta` inserting functions, it is instead
/// recommended that you make use of the appropriate macro to capture doc
/// comments instead:
/// * [`Module::macro_meta`].
/// * [`Module::function_meta`].
pub struct ItemMut<'a> {
    docs: &'a mut Docs,
}

impl ItemMut<'_> {
    /// Set documentation for an inserted item.
    ///
    /// This completely replaces any existing documentation.
    pub fn docs<I>(self, docs: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        self.docs.set_docs(docs);
        self
    }
}
